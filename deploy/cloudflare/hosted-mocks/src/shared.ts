import type { HostedMock } from "./hosted-mock";

export interface Env {
	HOSTED_MOCK: DurableObjectNamespace<HostedMock>;
	/** Registry base URL, e.g. https://api.mockforge.dev */
	REGISTRY_URL: string;
	/** Secret: MOCKFORGE_INTERNAL_API_TOKEN of the registry. */
	REGISTRY_INTERNAL_TOKEN: string;
	/** Secret: shared with the registry's MOCKFORGE_CF_MOCKS_EDGE_TOKEN. */
	EDGE_TOKEN: string;
}

/** Headers the Worker uses to talk to its Durable Object; never trusted from clients. */
export const H_ORG = "x-mockforge-edge-org";
export const H_SLUG = "x-mockforge-edge-slug";
export const H_OP = "x-mockforge-edge-op";
export const EDGE_TOKEN_HEADER = "x-mockforge-edge-token";

/** What the registry returns for a mock: its container environment and limits. */
export interface Runtime {
	deployment_id: string;
	env: Record<string, string>;
	limits: { max_body_bytes: number; rps: number };
	blocked: boolean;
}

export function json(status: number, body: unknown, headers: Record<string, string> = {}): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { "content-type": "application/json", ...headers },
	});
}
