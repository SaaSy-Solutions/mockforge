/** Calls from the Worker to the MockForge registry's internal API. */
import type { Env, Runtime } from "./shared";

/** Report usage after this many requests or this long, whichever comes first. */
const FLUSH_EVERY_REQUESTS = 100;
const FLUSH_EVERY_MS = 30_000;

/** The mock's runtime, or null when the registry does not serve it from here. */
export async function fetchRuntime(env: Env, org: string, slug: string): Promise<Runtime | null> {
	const res = await fetch(
		`${env.REGISTRY_URL}/api/v1/internal/hosted-mocks/runtime/${org}/${encodeURIComponent(slug)}`,
		{ redirect: "error", signal: AbortSignal.timeout(10_000), headers: { authorization: `Bearer ${env.REGISTRY_INTERNAL_TOKEN}` } },
	);
	if (!res.ok) {
		await res.body?.cancel();
		return null;
	}
	return (await res.json()) as Runtime;
}

interface Counts { requests: number; egress_bytes: number }
export interface UsageBatch extends Counts {
	batch_id: string;
	deployment_id: string;
	period_start: string;
}
export interface UsageState {
	counts: Record<string, Counts>;
	pending: UsageBatch | null;
}
/** The subset of durable storage needed for crash-safe reports. */
export interface UsageStorage {
	get(key: string): Promise<UsageState | undefined>;
	put(key: string, value: UsageState): Promise<void>;
}
const USAGE_KEY = "usage-v1";

/** Durable counts and immutable batches, serialized across overlapping requests. */
export class UsageMeter {
	private lastFlush = Date.now();
	private tail: Promise<unknown> = Promise.resolve();
	private sending: Promise<boolean | null> | null = null;
	constructor(private readonly storage: UsageStorage) {}

	private serial<T>(operation: () => Promise<T>): Promise<T> {
		const result = this.tail.then(operation);
		this.tail = result.catch(() => {});
		return result;
	}
	private async read(): Promise<UsageState> {
		return await this.storage.get(USAGE_KEY) ?? {
			counts: {}, pending: null,
		};
	}

	record(response: Response): Promise<boolean> {
		const period = `${new Date().toISOString().slice(0, 7)}-01`;
		return this.serial(async () => {
			if (response.status < 200 || response.status >= 300) return false;
			const state = await this.read();
			const counts = state.counts[period] ?? { requests: 0, egress_bytes: 0 };
			counts.requests++;
			const declared = Number(response.headers.get("content-length") ?? "256");
			counts.egress_bytes += Number.isSafeInteger(declared) && declared >= 0 ? declared : 256;
			state.counts[period] = counts;
			await this.storage.put(USAGE_KEY, state);
			return state.pending !== null || Object.values(state.counts).reduce((total, count) => total + count.requests, 0) >= FLUSH_EVERY_REQUESTS ||
				Date.now() - this.lastFlush >= FLUSH_EVERY_MS;
		});
	}

	flush(env: Pick<Env, "REGISTRY_URL" | "REGISTRY_INTERNAL_TOKEN">, deploymentId: string): Promise<boolean | null> {
		if (this.sending) return this.sending;
		this.sending = this.sendBatches(env, deploymentId).finally(() => { this.sending = null; });
		return this.sending;
	}

	private async sendBatches(env: Pick<Env, "REGISTRY_URL" | "REGISTRY_INTERNAL_TOKEN">, deploymentId: string): Promise<boolean | null> {
		let blocked: boolean | null = null;
		try {
			// Bound each background flush, even if traffic continues during reporting.
			for (let attempt = 0; attempt < 2; attempt++) {
				const batch = await this.serial(async () => {
					const state = await this.read();
					if (state.pending) return state.pending;
					const entry = Object.entries(state.counts)[0];
					if (!entry) return null;
					const [period, counts] = entry;
					state.pending = {
						...counts, batch_id: crypto.randomUUID(), deployment_id: deploymentId,
						period_start: period,
					};
					delete state.counts[period];
					// Persist the identity and payload BEFORE the network can commit it.
					await this.storage.put(USAGE_KEY, state);
					return state.pending;
				});
				if (!batch) break;
				// Network waits never hold the storage queue: serving and recording
				// can continue while the registry is slow or unavailable.
				const res = await fetch(
					`${env.REGISTRY_URL}/api/v1/internal/hosted-mocks/${batch.deployment_id}/usage`,
					{
						method: "POST", redirect: "error", signal: AbortSignal.timeout(10_000),
						headers: { authorization: `Bearer ${env.REGISTRY_INTERNAL_TOKEN}`, "content-type": "application/json" },
						body: JSON.stringify(batch),
					},
				);
				if (!res.ok) {
					await res.body?.cancel();
					throw new Error(`usage report returned ${res.status}`);
				}
				const result: unknown = await res.json();
				if (!result || typeof result !== "object" || !("blocked" in result) || typeof result.blocked !== "boolean") {
					throw new Error("invalid usage verdict");
				}
				blocked = result.blocked;
				await this.serial(async () => {
					const state = await this.read();
					if (state.pending?.batch_id !== batch.batch_id) throw new Error("usage batch changed before acknowledgement");
					state.pending = null;
					await this.storage.put(USAGE_KEY, state);
				});
				this.lastFlush = Date.now();
			}
		} catch (error) {
			console.error("hosted mock usage report failed", error);
		}
		return blocked;
	}
}
