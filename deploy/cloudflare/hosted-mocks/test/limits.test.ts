import { describe, expect, it } from "vitest";
import { admit, SecondWindow } from "../src/limits";
import type { Runtime } from "../src/shared";

const runtime = (over: Partial<Runtime> = {}): Runtime => ({
	deployment_id: "dep-1",
	env: {},
	limits: { max_body_bytes: 100, rps: 2 },
	blocked: false,
	...over,
});
const req = (bytes = 0) => new Request("https://edge/x", { headers: { "content-length": String(bytes) } });

describe("admit", () => {
	it("lets requests through within the limits", () => {
		expect(admit(req(10), runtime(), new SecondWindow(2), 0)).toBeNull();
	});

	it("rejects an org over its overage ceiling", () => {
		expect(admit(req(), runtime({ blocked: true }), new SecondWindow(2), 0)?.status).toBe(429);
	});

	it("rejects a declared body over the cap", () => {
		expect(admit(req(101), runtime(), new SecondWindow(2), 0)?.status).toBe(413);
	});

	it("rejects requests past the per-second cap with Retry-After", () => {
		const w = new SecondWindow(2);
		admit(req(), runtime(), w, 0);
		admit(req(), runtime(), w, 10);
		const res = admit(req(), runtime(), w, 20);
		expect(res?.status).toBe(429);
		expect(res?.headers.get("retry-after")).toBe("1");
	});
});
