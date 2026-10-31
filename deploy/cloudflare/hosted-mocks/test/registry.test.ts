import { afterEach, describe, expect, it, vi } from "vitest";
import { UsageMeter, type UsageState, type UsageStorage } from "../src/registry";

const env = { REGISTRY_URL: "https://registry.test", REGISTRY_INTERNAL_TOKEN: "t" };
const ok = (bytes = "10") => new Response("x", { status: 200, headers: { "content-length": bytes } });
function storage(): UsageStorage {
	let value: UsageState | undefined;
	return {
		async get() { return structuredClone(value); },
		async put(_key, next) { value = structuredClone(next); },
	};
}
afterEach(() => { vi.unstubAllGlobals(); vi.useRealTimers(); });

describe("UsageMeter", () => {
	it("counts only successful responses and asks for a report after 100", async () => {
		const meter = new UsageMeter(storage());
		expect(await meter.record(new Response("no", { status: 404 }))).toBe(false);
		let due = false;
		for (let i = 0; i < 100; i++) due = await meter.record(ok());
		expect(due).toBe(true);
	});
	it("sends durable counts and returns the registry verdict", async () => {
		const fetchMock = vi.fn(async (_url: string, _init: RequestInit) => Response.json({ blocked: true }));
		vi.stubGlobal("fetch", fetchMock);
		const store = storage();
		const meter = new UsageMeter(store);
		await meter.record(ok("10"));
		await meter.record(ok("15"));
		// Recorded responses also survive eviction before the first flush.
		expect(await new UsageMeter(store).flush(env, "dep-1")).toBe(true);
		const [url, init] = fetchMock.mock.calls[0];
		expect(url).toBe("https://registry.test/api/v1/internal/hosted-mocks/dep-1/usage");
		expect(JSON.parse(String(init.body))).toMatchObject({ requests: 2, egress_bytes: 25, batch_id: expect.any(String) });
		expect(await meter.flush(env, "dep-1")).toBeNull();
	});
	it("retries an immutable batch after lost acknowledgement and object eviction", async () => {
		const bodies: string[] = [];
		vi.stubGlobal("fetch", vi.fn(async (_url: string, init: RequestInit) => {
			bodies.push(String(init.body));
			if (bodies.length === 1) throw new Error("acknowledgement lost after commit");
			return Response.json({ blocked: false });
		}));
		const store = storage();
		const first = new UsageMeter(store);
		await first.record(ok("10"));
		expect(await first.flush(env, "dep-1")).toBeNull();
		const restored = new UsageMeter(store);
		await restored.record(ok("5"));
		expect(await restored.flush(env, "dep-1")).toBe(false);
		expect(bodies[1]).toBe(bodies[0]);
		const retried = JSON.parse(bodies[1]);
		const next = JSON.parse(bodies[2]);
		expect(retried).toMatchObject({ requests: 1, egress_bytes: 10, batch_id: expect.any(String) });
		expect(next).toMatchObject({ requests: 1, egress_bytes: 5 });
		expect(next.batch_id).not.toBe(retried.batch_id);
		expect(await restored.flush(env, "dep-1")).toBeNull();
	});
	it.each([false, true])("keeps usage in its original month across rollover (failed report: %s)", async (reportBeforeRollover) => {
		vi.useFakeTimers();
		vi.setSystemTime(new Date("2026-10-31T23:59:59Z"));
		const bodies: string[] = [];
		vi.stubGlobal("fetch", vi.fn(async (_url: string, init: RequestInit) => {
			bodies.push(String(init.body));
			if (reportBeforeRollover && bodies.length === 1) throw new Error("lost acknowledgement");
			return Response.json({ blocked: false });
		}));
		const store = storage();
		const meter = new UsageMeter(store);
		await meter.record(ok("10"));
		if (reportBeforeRollover) await meter.flush(env, "dep-1");
		vi.setSystemTime(new Date("2026-11-01T00:00:01Z"));
		const restored = new UsageMeter(store);
		await restored.record(ok("5"));
		await restored.flush(env, "dep-1");
		const october = JSON.parse(bodies[bodies.length - 2]);
		const november = JSON.parse(bodies[bodies.length - 1]);
		expect(october).toMatchObject({ requests: 1, egress_bytes: 10, period_start: "2026-10-01" });
		expect(november).toMatchObject({ requests: 1, egress_bytes: 5, period_start: "2026-11-01" });
		if (reportBeforeRollover) expect(bodies[1]).toBe(bodies[0]);
	});

	it("serializes overlapping records and flushes without lost or duplicate counts", async () => {
		const fetchMock = vi.fn(async (_url: string, _init: RequestInit) => Response.json({ blocked: false }));
		vi.stubGlobal("fetch", fetchMock);
		const meter = new UsageMeter(storage());
		await Promise.all(Array.from({ length: 10 }, () => meter.record(ok())));
		await Promise.all([meter.flush(env, "dep-1"), meter.flush(env, "dep-1")]);
		expect(fetchMock).toHaveBeenCalledOnce();
		expect(JSON.parse(String(fetchMock.mock.calls[0][1].body))).toMatchObject({ requests: 10, egress_bytes: 100 });
	});
	it("keeps recording while a usage acknowledgement is pending", async () => {
		let release = () => {};
		let started = () => {};
		const pending = new Promise<void>((resolve) => { release = resolve; });
		const sending = new Promise<void>((resolve) => { started = resolve; });
		vi.stubGlobal("fetch", vi.fn(async () => {
			started(); await pending; return Response.json({ blocked: false });
		}));
		const meter = new UsageMeter(storage());
		await meter.record(ok());
		const flush = meter.flush(env, "dep-1");
		await sending;
		let recorded = false;
		const record = meter.record(ok()).then(() => { recorded = true; });
		try { await vi.waitFor(() => expect(recorded).toBe(true), { timeout: 500 }); }
		finally { release(); await record; await flush; }
	});

	it("replays the committed batch when clearing its durable receipt fails", async () => {
		const store = storage();
		const meter = new UsageMeter(store);
		await meter.record(ok());
		const bodies: string[] = [];
		vi.stubGlobal("fetch", vi.fn(async (_url: string, init: RequestInit) => {
			bodies.push(String(init.body));
			if (bodies.length === 1) vi.spyOn(store, "put").mockRejectedValueOnce(new Error("acknowledgement storage failed"));
			return Response.json({ blocked: false });
		}));
		await meter.flush(env, "dep-1");
		await new UsageMeter(store).flush(env, "dep-1");
		expect(bodies).toHaveLength(2);
		expect(bodies[1]).toBe(bodies[0]);
	});

	it("does not send a batch before durable storage acknowledges it", async () => {
		const store = storage();
		const meter = new UsageMeter(store);
		await meter.record(ok());
		vi.spyOn(store, "put").mockRejectedValueOnce(new Error("storage failed"));
		const fetchMock = vi.fn(async () => Response.json({ blocked: false }));
		vi.stubGlobal("fetch", fetchMock);
		expect(await meter.flush(env, "dep-1")).toBeNull();
		expect(fetchMock).not.toHaveBeenCalled();
		expect(await meter.flush(env, "dep-1")).toBe(false);
	});
});
