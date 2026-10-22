import { describe, expect, it } from "vitest";
import { SecondWindow } from "../src/limits";
import { constantTimeEqual, parseRoute } from "../src/routing";

const ORG = "7f4c2a1e-3b5d-4e6f-8a9b-0c1d2e3f4a5b";

describe("parseRoute", () => {
	it("splits org, slug and the rest of the path", () => {
		expect(parseRoute(`/${ORG}/shop`)).toEqual({ kind: "mock", org: ORG, slug: "shop", rest: "/" });
		expect(parseRoute(`/${ORG}/shop/`)).toEqual({ kind: "mock", org: ORG, slug: "shop", rest: "/" });
		expect(parseRoute(`/${ORG}/shop/api/v1/users/42`)).toMatchObject({ rest: "/api/v1/users/42" });
		expect(parseRoute(`/${ORG}/shop/a%2Fb`)).toMatchObject({ rest: "/a%2Fb" });
	});

	it("accepts every slug the registry accepts", () => {
		expect(parseRoute(`/${ORG}/My-Shop`)).toMatchObject({ slug: "My-Shop" });
		expect(parseRoute(`/${ORG}/-shop-`)).toMatchObject({ slug: "-shop-" });
		expect(parseRoute(`/${ORG}/%E3%83%A2%E3%83%83%E3%82%AF`)).toMatchObject({ slug: "モック" });
	});

	it("recognises registry admin calls", () => {
		expect(parseRoute(`/__mockforge-edge/${ORG}/shop/restart`)).toEqual({
			kind: "admin", org: ORG, slug: "shop", action: "restart",
		});
		expect(parseRoute(`/__mockforge-edge/${ORG}/shop/destroy`)).toMatchObject({ action: "destroy" });
	});

	it("rejects anything else", () => {
		for (const path of [
			"/", "/favicon.ico", `/not-a-uuid/shop`, `/${ORG}`, `/${ORG}/sh.op`, `/${ORG}/a%2Fb`, `/${ORG}/%E0%A4%A`,
			`/${ORG.toUpperCase()}/shop`, `/__mockforge-edge/${ORG}/shop/delete`,
			`/__mockforge-edge/${ORG}/shop/restart/extra`, `/__mockforge-edge/${ORG}/shop`,
		]) {
			expect(parseRoute(path), path).toBeNull();
		}
	});
});

describe("constantTimeEqual", () => {
	it("matches only identical non-empty strings", () => {
		expect(constantTimeEqual("s3cret", "s3cret")).toBe(true);
		expect(constantTimeEqual("s3cret", "s3creT")).toBe(false);
		expect(constantTimeEqual("s3cret", "s3cre")).toBe(false);
		expect(constantTimeEqual("", "")).toBe(false);
	});
});

describe("SecondWindow", () => {
	it("allows the limit per second and resets on the next second", () => {
		const w = new SecondWindow(2);
		expect([w.allow(1000), w.allow(1500), w.allow(1999)]).toEqual([true, true, false]);
		expect(w.allow(2000)).toBe(true);
	});
});
