/**
 * One Durable Object per hosted mock, in front of that mock's container. It
 * starts the container with the environment the registry returns, applies the
 * registry proxy's limits, and reports usage back.
 */
import { Container } from "@cloudflare/containers";
import { waitUntilServing } from "./health";
import { admit, SecondWindow } from "./limits";
import { fetchRuntime, UsageMeter } from "./registry";
import type { AdminAction } from "./routing";
import { type Env, H_OP, H_ORG, H_SLUG, json, type Runtime } from "./shared";

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
		const rejected = admit(request, runtime, this.window, Date.now());
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
		await waitUntilServing(this.ctx.container!.getTcpPort(this.defaultPort));
	}

	private async flushUsage(): Promise<void> {
		const runtime = this.runtime;
		if (!runtime) return;
		const blocked = await this.usage.flush(this.env, runtime.deployment_id);
		if (blocked !== null) runtime.blocked = blocked;
	}
}
