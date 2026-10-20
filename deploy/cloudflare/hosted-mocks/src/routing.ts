/** Pure helpers for the hosted-mocks Worker, kept free of runtime bindings so
 * they can be unit-tested in Node. */

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
/** Same rule as the registry's slug check: letters, digits and hyphens. */
const SLUG = /^[\p{L}\p{N}-]{1,255}$/u;

/** Path prefix of the registry-only admin endpoints. */
export const ADMIN_PREFIX = "__mockforge-edge";

export type AdminAction = "restart" | "destroy";

export type Route =
	| { kind: "admin"; org: string; slug: string; action: AdminAction }
	| { kind: "mock"; org: string; slug: string; rest: string };

/**
 * `/<org>/<slug>/<rest>` addresses a mock; `/__mockforge-edge/<org>/<slug>/<action>`
 * is an admin call from the registry. `rest` keeps its leading slash and the
 * original percent-encoding; it is `/` when the path stops at the slug.
 */
export function parseRoute(pathname: string): Route | null {
	const parts = pathname.split("/");
	// parts[0] is the empty string before the leading slash.
	if (parts[1] === ADMIN_PREFIX) {
		const [, , org, rawSlug, action, ...extra] = parts;
		const slug = decodeSegment(rawSlug);
		if (extra.length || !isOrg(org) || !isSlug(slug)) return null;
		if (action !== "restart" && action !== "destroy") return null;
		return { kind: "admin", org, slug, action };
	}
	const [, org, rawSlug, ...rest] = parts;
	const slug = decodeSegment(rawSlug);
	if (!isOrg(org) || !isSlug(slug)) return null;
	return { kind: "mock", org, slug, rest: "/" + rest.join("/") };
}

function isOrg(value: string | undefined): value is string {
	return value !== undefined && UUID.test(value);
}

function decodeSegment(value: string | undefined): string | undefined {
	if (value === undefined) return undefined;
	try {
		return decodeURIComponent(value);
	} catch {
		return undefined;
	}
}

function isSlug(value: string | undefined): value is string {
	return value !== undefined && SLUG.test(value);
}

/** Length-checked comparison that does not stop at the first differing byte. */
export function constantTimeEqual(a: string, b: string): boolean {
	const enc = new TextEncoder();
	const x = enc.encode(a);
	const y = enc.encode(b);
	if (x.length !== y.length || x.length === 0) return false;
	let diff = 0;
	for (let i = 0; i < x.length; i++) diff |= x[i] ^ y[i];
	return diff === 0;
}

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
