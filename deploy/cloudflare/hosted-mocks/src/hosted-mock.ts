/**
 * One Durable Object per hosted mock, in front of that mock's container. It
 * starts the container with the environment the registry returns, applies the
 * registry proxy's limits, and reports usage back.
 */
import { Container } from "@cloudflare/containers";
import { fetchRuntime, UsageMeter } from "./registry";
import { type AdminAction, SecondWindow } from "./routing";
import { type Env, H_OP, H_ORG, H_SLUG, json, type Runtime } from "./shared";

/** How long a cold start may take before the request fails. */
const HEALTH_TIMEOUT_MS = 60_000;

export class HostedMock extends Container<Env> {
	defaultPort = 3000;
	sleepAfter = "10m";
	enableInternet = true; // spec URLs, log/capture ingest, upstream proxying

	private runtime: Runtime | null = null;
	private window: SecondWindow | null = null;
	private readonly usage = new UsageMeter();

	override async fetch(request: Request): Promise<Response> {
		const org = request.headers.get(H_ORG);
		const slug = request.headers.get(H_SLUG);
		if (!org || !slug) return json(400, { error: "bad_request" });

		const op = request.headers.get(H_OP);
		if (op === "restart" || op === "destroy") return this.reset(op);

		const runtime = await this.loadRuntime(org, slug);
		if (!runtime) return json(404, { error: "mock_not_found" });
		const rejected = this.admit(request, runtime);
		if (rejected) return rejected;

		await this.ensureRunning(runtime);
		const forwarded = new Request(request);
		for (const name of [H_ORG, H_SLUG]) forwarded.headers.delete(name);
		const response = await this.containerFetch(forwarded, this.defaultPort);
		if (this.usage.record(response)) this.ctx.waitUntil(this.flushUsage());
		return response;
	}

	override async onStop(): Promise<void> {
		await this.flushUsage();
		// The next start reads the registry again, so config changes apply.
		this.runtime = null;
	}

	/** Stop the container and forget its runtime, so the next request reloads it. */
	private async reset(op: AdminAction): Promise<Response> {
		await this.flushUsage();
		this.runtime = null;
		this.window = null;
		if (this.ctx.container?.running) await this.destroy();
		return json(200, { ok: true, action: op });
	}

	/** The registry proxy's limits: quota ceiling, body cap, requests per second. */
	private admit(request: Request, runtime: Runtime): Response | null {
		if (runtime.blocked) return json(429, { error: "usage_limit_exceeded" });
		const declared = Number(request.headers.get("content-length") ?? "0");
		if (declared > runtime.limits.max_body_bytes) return json(413, { error: "payload_too_large" });
		if (!this.window?.allow(Date.now())) {
			return json(429, { error: "rate_limited" }, { "retry-after": "1" });
		}
		return null;
	}

	private async loadRuntime(org: string, slug: string): Promise<Runtime | null> {
		if (this.runtime) return this.runtime;
		const runtime = await fetchRuntime(this.env, org, slug);
		if (runtime) {
			this.runtime = runtime;
			this.window = new SecondWindow(Math.max(1, runtime.limits.rps));
		}
		return runtime;
	}

	private async ensureRunning(runtime: Runtime): Promise<void> {
		if (this.ctx.container?.running) return;
		await this.startAndWaitForPorts({
			startOptions: { envVars: runtime.env },
			cancellationOptions: { portReadyTimeoutMS: 60_000 },
		});
		await this.waitUntilServing();
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

	private async flushUsage(): Promise<void> {
		const runtime = this.runtime;
		if (!runtime) return;
		const blocked = await this.usage.flush(this.env, runtime.deployment_id);
		if (blocked !== null) runtime.blocked = blocked;
	}
}
