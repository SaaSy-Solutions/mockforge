import { afterEach, describe, expect, it, vi } from "vitest";
import { UsageMeter } from "../src/registry";
import type { Env } from "../src/shared";

const env = { REGISTRY_URL: "https://registry.test", REGISTRY_INTERNAL_TOKEN: "t" } as Env;
const ok = (bytes = "10") => new Response("x", { status: 200, headers: { "content-length": bytes } });

afterEach(() => vi.unstubAllGlobals());

describe("UsageMeter", () => {
	it("counts only successful responses and asks for a report after 100", () => {
		const meter = new UsageMeter();
		expect(meter.record(new Response("no", { status: 404 }))).toBe(false);
		let due = false;
		for (let i = 0; i < 100; i++) due = meter.record(ok());
		expect(due).toBe(true);
	});

	it("sends the counts and returns the registry's verdict", async () => {
		const fetchMock = vi.fn(async () => Response.json({ blocked: true }));
		vi.stubGlobal("fetch", fetchMock);
		const meter = new UsageMeter();
		meter.record(ok("10"));
		meter.record(ok("15"));
		expect(await meter.flush(env, "dep-1")).toBe(true);
		const [url, init] = fetchMock.mock.calls[0] as unknown as [string, RequestInit];
		expect(url).toBe("https://registry.test/api/v1/internal/hosted-mocks/dep-1/usage");
		expect(JSON.parse(init.body as string)).toEqual({ requests: 2, egress_bytes: 25 });
		expect(await meter.flush(env, "dep-1")).toBeNull(); // nothing left to send
	});

	it("keeps the counts when a report fails", async () => {
		vi.stubGlobal("fetch", vi.fn(async () => new Response("down", { status: 503 })));
		const meter = new UsageMeter();
		meter.record(ok("10"));
		expect(await meter.flush(env, "dep-1")).toBeNull();

		const fetchMock = vi.fn(async () => Response.json({ blocked: false }));
		vi.stubGlobal("fetch", fetchMock);
		meter.record(ok("5"));
		expect(await meter.flush(env, "dep-1")).toBe(false);
		const [, init] = fetchMock.mock.calls[0] as unknown as [string, RequestInit];
		expect(JSON.parse(init.body as string)).toEqual({ requests: 2, egress_bytes: 15 });
	});
});
