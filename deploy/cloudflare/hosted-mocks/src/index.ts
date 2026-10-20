/**
 * MockForge hosted mocks on Cloudflare Containers.
 *
 * `mocks.mockforge.dev/<org_id>/<slug>/<path>` is served by one container per
 * hosted mock. The container starts on the first request with the environment
 * the registry returns for that mock and sleeps after `sleepAfter` with no
 * traffic, so an idle mock costs nothing. The Durable Object in front of each
 * container applies the same body cap and requests-per-second cap as the
 * registry proxy and reports request counts back for metering.
 *
 * The registry calls `POST /__mockforge-edge/<org>/<slug>/{restart,destroy}`
 * with the shared edge token when a mock is redeployed, stopped or deleted.
 */
import { Container, getContainer } from "@cloudflare/containers";
import { constantTimeEqual, parseRoute, SecondWindow } from "./routing";

export interface Env {
	HOSTED_MOCK: DurableObjectNamespace<HostedMock>;
	/** Registry base URL, e.g. https://api.mockforge.dev */
	REGISTRY_URL: string;
	/** Secret: MOCKFORGE_INTERNAL_API_TOKEN of the registry. */
	REGISTRY_INTERNAL_TOKEN: string;
	/** Secret: shared with the registry's MOCKFORGE_CF_MOCKS_EDGE_TOKEN. */
	EDGE_TOKEN: string;
}

/** Headers the Worker uses to talk to its Durable Object; never trusted from clients. */
const H_ORG = "x-mockforge-edge-org";
const H_SLUG = "x-mockforge-edge-slug";
const H_OP = "x-mockforge-edge-op";
const EDGE_TOKEN_HEADER = "x-mockforge-edge-token";

/** Report usage after this many requests or this long, whichever comes first. */
const FLUSH_EVERY_REQUESTS = 100;
const FLUSH_EVERY_MS = 30_000;
/** How long a cold start may take before the request fails. */
const HEALTH_TIMEOUT_MS = 60_000;

interface Runtime {
	deployment_id: string;
	env: Record<string, string>;
	limits: { max_body_bytes: number; rps: number };
	blocked: boolean;
}

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		const url = new URL(request.url);
		const route = parseRoute(url.pathname);
		if (!route) return json(404, { error: "not_found" });

		const stub = getContainer(env.HOSTED_MOCK, `${route.org}/${route.slug}`);

		if (route.kind === "admin") {
			const token = request.headers.get(EDGE_TOKEN_HEADER) ?? "";
			if (request.method !== "POST" || !constantTimeEqual(token, env.EDGE_TOKEN ?? "")) {
				return json(403, { error: "forbidden" });
			}
			return stub.fetch(
				new Request("https://edge/admin", {
					method: "POST",
					headers: { [H_OP]: route.action, [H_ORG]: route.org, [H_SLUG]: route.slug },
				}),
			);
		}

		// Forward with the mock's own path, dropping any edge-control headers a
		// client tried to set.
		const target = new URL(route.rest + url.search, "https://edge");
		const headers = new Headers(request.headers);
		for (const name of [H_ORG, H_SLUG, H_OP, EDGE_TOKEN_HEADER]) headers.delete(name);
		headers.set(H_ORG, route.org);
		headers.set(H_SLUG, route.slug);
		return stub.fetch(new Request(target, new Request(request, { headers })));
	},
} satisfies ExportedHandler<Env>;

export class HostedMock extends Container<Env> {
	defaultPort = 3000;
	sleepAfter = "10m";
	enableInternet = true; // spec URLs, log/capture ingest, upstream proxying

	private runtime: Runtime | null = null;
	private window: SecondWindow | null = null;
	private pendingRequests = 0;
	private pendingEgress = 0;
	private lastFlush = Date.now();

	override async fetch(request: Request): Promise<Response> {
		const org = request.headers.get(H_ORG);
		const slug = request.headers.get(H_SLUG);
		if (!org || !slug) return json(400, { error: "bad_request" });

		const op = request.headers.get(H_OP);
		if (op === "restart" || op === "destroy") {
			await this.flushUsage();
			this.runtime = null;
			this.window = null;
			if (this.ctx.container?.running) await this.destroy();
			return json(200, { ok: true, action: op });
		}

		const runtime = await this.loadRuntime(org, slug);
		if (!runtime) return json(404, { error: "mock_not_found" });
		if (runtime.blocked) return json(429, { error: "usage_limit_exceeded" });

		const declared = Number(request.headers.get("content-length") ?? "0");
		if (declared > runtime.limits.max_body_bytes) return json(413, { error: "payload_too_large" });
		if (!this.window?.allow(Date.now())) {
			return json(429, { error: "rate_limited" }, { "retry-after": "1" });
		}

		if (!this.ctx.container?.running) {
			await this.startAndWaitForPorts({
				startOptions: { envVars: runtime.env },
				cancellationOptions: { portReadyTimeoutMS: 60_000 },
			});
			await this.waitUntilServing();
		}

		const forwarded = new Request(request);
		for (const name of [H_ORG, H_SLUG]) forwarded.headers.delete(name);
		const response = await this.containerFetch(forwarded, this.defaultPort);

		this.recordUsage(response);
		return response;
	}

	override async onStop(): Promise<void> {
		await this.flushUsage();
		// The next start reads the registry again, so config changes apply.
		this.runtime = null;
	}

	/**
	 * The port accepts connections before MockForge has loaded the spec, so a
	 * cold request forwarded right away fails. Wait for the mock's own
	 * liveness endpoint instead.
	 */
	private async waitUntilServing(): Promise<void> {
		const port = this.ctx.container!.getTcpPort(this.defaultPort);
		const deadline = Date.now() + HEALTH_TIMEOUT_MS;
		let lastError: unknown;
		while (Date.now() < deadline) {
			try {
				const res = await port.fetch("http://container/health/live", {
					signal: AbortSignal.timeout(2_000),
				});
				await res.body?.cancel();
				if (res.ok) return;
				lastError = new Error(`health check returned ${res.status}`);
			} catch (error) {
				lastError = error;
			}
			await scheduler.wait(200);
		}
		throw new Error("hosted mock did not become healthy", { cause: lastError });
	}

	/** Runtime for this mock, read from the registry once per container start. */
	private async loadRuntime(org: string, slug: string): Promise<Runtime | null> {
		if (this.runtime) return this.runtime;
		const res = await fetch(
			`${this.env.REGISTRY_URL}/api/v1/internal/hosted-mocks/runtime/${org}/${encodeURIComponent(slug)}`,
			{ headers: { authorization: `Bearer ${this.env.REGISTRY_INTERNAL_TOKEN}` } },
		);
		if (!res.ok) {
			await res.body?.cancel();
			return null;
		}
		const runtime = (await res.json()) as Runtime;
		this.runtime = runtime;
		this.window = new SecondWindow(Math.max(1, runtime.limits.rps));
		return runtime;
	}

	private recordUsage(response: Response): void {
		if (response.status < 200 || response.status >= 300) return;
		this.pendingRequests++;
		this.pendingEgress += Number(response.headers.get("content-length") ?? "256") || 256;
		if (
			this.pendingRequests >= FLUSH_EVERY_REQUESTS ||
			Date.now() - this.lastFlush >= FLUSH_EVERY_MS
		) {
			this.ctx.waitUntil(this.flushUsage());
		}
	}

	/** Send unreported counts to the registry and pick up its quota verdict. */
	private async flushUsage(): Promise<void> {
		const runtime = this.runtime;
		if (!runtime || this.pendingRequests === 0) return;
		const requests = this.pendingRequests;
		const egress = this.pendingEgress;
		this.pendingRequests = 0;
		this.pendingEgress = 0;
		this.lastFlush = Date.now();
		try {
			const res = await fetch(
				`${this.env.REGISTRY_URL}/api/v1/internal/hosted-mocks/${runtime.deployment_id}/usage`,
				{
					method: "POST",
					headers: {
						authorization: `Bearer ${this.env.REGISTRY_INTERNAL_TOKEN}`,
						"content-type": "application/json",
					},
					body: JSON.stringify({ requests, egress_bytes: egress }),
				},
			);
			if (!res.ok) throw new Error(`usage report returned ${res.status}`);
			const { blocked } = (await res.json()) as { blocked: boolean };
			runtime.blocked = blocked;
		} catch (error) {
			// Keep the counts for the next attempt rather than losing them.
			this.pendingRequests += requests;
			this.pendingEgress += egress;
			console.error("hosted mock usage report failed", error);
		}
	}
}

function json(status: number, body: unknown, headers: Record<string, string> = {}): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { "content-type": "application/json", ...headers },
	});
}
