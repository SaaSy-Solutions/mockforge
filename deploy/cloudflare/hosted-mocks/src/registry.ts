/** Calls from the Worker to the MockForge registry's internal API. */
import type { Env, Runtime } from "./shared";

/** Report usage after this many requests or this long, whichever comes first. */
const FLUSH_EVERY_REQUESTS = 100;
const FLUSH_EVERY_MS = 30_000;

/** The mock's runtime, or null when the registry does not serve it from here. */
export async function fetchRuntime(env: Env, org: string, slug: string): Promise<Runtime | null> {
	const res = await fetch(
		`${env.REGISTRY_URL}/api/v1/internal/hosted-mocks/runtime/${org}/${encodeURIComponent(slug)}`,
		{ headers: { authorization: `Bearer ${env.REGISTRY_INTERNAL_TOKEN}` } },
	);
	if (!res.ok) {
		await res.body?.cancel();
		return null;
	}
	return (await res.json()) as Runtime;
}

/** Counts successful responses and reports them to the registry in batches. */
export class UsageMeter {
	private requests = 0;
	private egress = 0;
	private lastFlush = Date.now();

	/** Records a response; true when a report is due. */
	record(response: Response): boolean {
		if (response.status < 200 || response.status >= 300) return false;
		this.requests++;
		this.egress += Number(response.headers.get("content-length") ?? "256") || 256;
		return this.requests >= FLUSH_EVERY_REQUESTS || Date.now() - this.lastFlush >= FLUSH_EVERY_MS;
	}

	/**
	 * Sends unreported counts and returns the registry's quota verdict, or null
	 * when nothing was sent. Failed reports keep their counts for next time.
	 */
	async flush(env: Env, deploymentId: string): Promise<boolean | null> {
		if (this.requests === 0) return null;
		const requests = this.requests;
		const egress = this.egress;
		this.requests = 0;
		this.egress = 0;
		this.lastFlush = Date.now();
		try {
			const res = await fetch(
				`${env.REGISTRY_URL}/api/v1/internal/hosted-mocks/${deploymentId}/usage`,
				{
					method: "POST",
					headers: {
						authorization: `Bearer ${env.REGISTRY_INTERNAL_TOKEN}`,
						"content-type": "application/json",
					},
					body: JSON.stringify({ requests, egress_bytes: egress }),
				},
			);
			if (!res.ok) throw new Error(`usage report returned ${res.status}`);
			const { blocked } = (await res.json()) as { blocked: boolean };
			return blocked;
		} catch (error) {
			this.requests += requests;
			this.egress += egress;
			console.error("hosted mock usage report failed", error);
			return null;
		}
	}
}
