/** Readiness check for a freshly started mock container. */

/** How long a cold start may take before the request fails. */
const HEALTH_TIMEOUT_MS = 60_000;

/**
 * The port accepts connections before MockForge has loaded the spec, so a
 * cold request forwarded right away fails. Wait for the mock's own liveness
 * endpoint instead.
 */
export async function waitUntilServing(port: Fetcher): Promise<void> {
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
