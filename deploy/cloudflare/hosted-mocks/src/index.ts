/**
 * MockForge hosted mocks on Cloudflare Containers.
 *
 * `mocks.mockforge.dev/<org_id>/<slug>/<path>` is served by one container per
 * hosted mock (see `hosted-mock.ts`). The container starts on the first
 * request and sleeps when idle, so an unused mock costs nothing.
 *
 * The registry calls `POST /__mockforge-edge/<org>/<slug>/{restart,destroy}`
 * with the shared edge token when a mock is redeployed, stopped or deleted.
 */
import { getContainer } from "@cloudflare/containers";
import type { HostedMock } from "./hosted-mock";
import { type AdminAction, constantTimeEqual, parseRoute } from "./routing";
import { EDGE_TOKEN_HEADER, type Env, H_OP, H_ORG, H_SLUG, json } from "./shared";

export { HostedMock } from "./hosted-mock";
export type { Env } from "./shared";

type Stub = DurableObjectStub<HostedMock>;

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		const route = parseRoute(new URL(request.url).pathname);
		if (!route) return json(404, { error: "not_found" });
		const stub = getContainer(env.HOSTED_MOCK, `${route.org}/${route.slug}`);
		return route.kind === "admin"
			? forwardAdmin(request, env, stub, route)
			: forwardToMock(request, stub, route);
	},
} satisfies ExportedHandler<Env>;

/** Registry-only restart/destroy, authorised by the shared edge token. */
function forwardAdmin(
	request: Request,
	env: Env,
	stub: Stub,
	route: { org: string; slug: string; action: AdminAction },
): Promise<Response> | Response {
	const token = request.headers.get(EDGE_TOKEN_HEADER) ?? "";
	if (request.method !== "POST" || !constantTimeEqual(token, env.EDGE_TOKEN ?? "")) {
		return json(403, { error: "forbidden" });
	}
	return stub.fetch(
		new Request("https://edge/admin", {
			method: "POST",
			headers: { [H_OP]: route.action, [H_ORG]: route.org, [H_SLUG]: route.slug },
		}),
	);
}

/** Forward with the mock's own path, dropping any edge-control headers a client set. */
function forwardToMock(
	request: Request,
	stub: Stub,
	route: { org: string; slug: string; rest: string },
): Promise<Response> {
	const target = new URL(route.rest + new URL(request.url).search, "https://edge");
	const headers = new Headers(request.headers);
	for (const name of [H_ORG, H_SLUG, H_OP, EDGE_TOKEN_HEADER]) headers.delete(name);
	headers.set(H_ORG, route.org);
	headers.set(H_SLUG, route.slug);
	return stub.fetch(new Request(target, new Request(request, { headers })));
}
