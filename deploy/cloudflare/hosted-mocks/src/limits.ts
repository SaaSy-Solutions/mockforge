/** The registry proxy's per-mock limits, applied at the edge. */
import { json, type Runtime } from "./shared";

/** Fixed one-second window, matching the registry proxy's per-deployment cap. */
export class SecondWindow {
	private second = -1;
	private count = 0;

	constructor(private readonly limit: number) {}

	/** Records a request at `nowMs`; false when the second's budget is spent. */
	allow(nowMs: number): boolean {
		const second = Math.floor(nowMs / 1000);
		if (second !== this.second) {
			this.second = second;
			this.count = 0;
		}
		if (this.count >= this.limit) return false;
		this.count++;
		return true;
	}
}

/**
 * Rejection for a request the mock's limits do not allow (overage ceiling,
 * body cap, requests per second), or null to let it through. A request that
 * is let through uses one slot of the current second.
 */
export function admit(
	request: Request,
	runtime: Runtime,
	window: SecondWindow | null,
	nowMs: number,
): Response | null {
	if (runtime.blocked) return json(429, { error: "usage_limit_exceeded" });
	const declared = Number(request.headers.get("content-length") ?? "0");
	if (declared > runtime.limits.max_body_bytes) return json(413, { error: "payload_too_large" });
	if (!window?.allow(nowMs)) return json(429, { error: "rate_limited" }, { "retry-after": "1" });
	return null;
}
