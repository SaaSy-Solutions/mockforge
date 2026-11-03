import { afterEach, describe, expect, it, vi } from "vitest";
import { HostedMock } from "../src/hosted-mock";
import { H_ORG, H_SLUG, type Env, type Runtime } from "../src/shared";

// Only the Cloudflare platform transport is replaced; fetch admission is real.
vi.mock("@cloudflare/containers", () => ({
	Container: class {
		constructor(protected ctx: DurableObjectState, protected env: Env) {}
		containerFetch(request: Request) { return transport(request); }
	},
}));
const { transport } = vi.hoisted(() => ({
	transport: vi.fn(async (request: Request) => new Response(await request.text())),
}));
const runtime: Runtime = {
	deployment_id: "deployment", env: {}, limits: { max_body_bytes: 4, rps: 10 }, blocked: false,
};
const env = { REGISTRY_URL: "https://registry.test", REGISTRY_INTERNAL_TOKEN: "t" } as Env;
function hostedMock() {
	// Node cannot construct a real DurableObjectState.
	const ctx = { container: { running: true }, waitUntil: vi.fn(), storage: {
		get: vi.fn(), put: vi.fn(), delete: vi.fn(),
	} } as unknown as ConstructorParameters<typeof HostedMock>[0];
	return new HostedMock(ctx, env);
}
function request(body?: string, headers: Record<string, string> = {}) {
	return new Request("https://edge/mock", {
		method: body === undefined ? "GET" : "POST", body,
		headers: { [H_ORG]: "org", [H_SLUG]: "slug", ...headers },
	});
}
afterEach(() => { vi.unstubAllGlobals(); vi.useRealTimers(); transport.mockClear(); });
describe("HostedMock admission", () => {
	it.each<Record<string, string>>([{}, { "content-length": "1" }])("rejects oversized bodies before forwarding, headers %j", async (headers) => {
		vi.stubGlobal("fetch", vi.fn(async () => Response.json(runtime)));
		const response = await hostedMock().fetch(request("12345", headers));
		expect(response.status).toBe(413);
		expect(transport).not.toHaveBeenCalled();
	});
	it("cancels an unknown-length stream as soon as it exceeds the cap", async () => {
		vi.stubGlobal("fetch", vi.fn(async () => Response.json(runtime)));
		const cancel = vi.fn();
		const body = new ReadableStream<Uint8Array>({
			pull(controller) { controller.enqueue(new Uint8Array(3)); }, cancel,
		});
		const init = { method: "POST", body, duplex: "half", headers: { [H_ORG]: "org", [H_SLUG]: "slug" } };
		expect((await hostedMock().fetch(new Request("https://edge/mock", init))).status).toBe(413);
		expect(cancel).toHaveBeenCalledOnce();
		expect(transport).not.toHaveBeenCalled();
	});

	it("forwards bodies exactly at the cap intact", async () => {
		vi.stubGlobal("fetch", vi.fn(async () => Response.json(runtime)));
		const response = await hostedMock().fetch(request("1234"));
		expect(response.status).toBe(200);
		expect(await response.text()).toBe("1234");
	});
	it("refreshes a blocked quota after the cache lifetime", async () => {
		vi.useFakeTimers();
		const registry = vi.fn().mockImplementation(async () => Response.json({ blocked: false })).mockResolvedValueOnce(Response.json({ ...runtime, blocked: true }))
			.mockResolvedValueOnce(Response.json(runtime));
		vi.stubGlobal("fetch", registry);
		const mock = hostedMock();
		expect((await mock.fetch(request())).status).toBe(429);
		vi.advanceTimersByTime(30_001);
		expect((await mock.fetch(request())).status).toBe(200);
		expect(registry.mock.calls.filter(([url]) => String(url).includes("/runtime/"))).toHaveLength(2);
	});
});
