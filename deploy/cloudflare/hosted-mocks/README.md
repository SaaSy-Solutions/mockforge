# Hosted mocks on Cloudflare Containers

This Worker serves MockForge hosted mocks that use HTTP, WebSocket or GraphQL at
`https://mocks.mockforge.dev/<org_id>/<slug>/<path>`. Each mock runs in its own
container, which starts on the first request and sleeps after 10 minutes without
traffic. You pay only while a container runs. Mocks that need gRPC, Kafka, MQTT,
AMQP, SMTP or raw TCP, or that have plugins attached, run on Fly instead
(machines there also stop when idle).

## How a request is served

1. The Worker parses `/<org_id>/<slug>/...` and routes to the mock's Durable Object.
2. On a cold start the Durable Object reads the mock's environment and limits from
   the registry (`GET /api/v1/internal/hosted-mocks/runtime/{org}/{slug}`, internal
   token), starts the container with that environment, and waits for `/health/live`.
3. It enforces the plan's body cap and requests-per-second cap, forwards the request,
   and reports request counts to the registry every 100 requests or 30 seconds
   (`POST /api/v1/internal/hosted-mocks/{id}/usage`). The registry answers whether
   the org is over its quota ceiling; if so, requests get 429.

The registry calls `POST /__mockforge-edge/<org>/<slug>/{restart,destroy}` with the
`x-mockforge-edge-token` header when a mock is redeployed, stopped or deleted.

## Deploy

Requires the Workers Paid plan (Containers) and Docker with buildx 0.13 or later
(wrangler builds the image from `Dockerfile` and pushes it to Cloudflare's registry).

```sh
pnpm install
pnpm wrangler secret put REGISTRY_INTERNAL_TOKEN   # the registry's MOCKFORGE_INTERNAL_API_TOKEN
pnpm wrangler secret put EDGE_TOKEN                # a new random secret
pnpm deploy
```

Then set on the registry:

| Variable | Value |
|---|---|
| `MOCKFORGE_CF_MOCKS_URL` | `https://mocks.mockforge.dev` |
| `MOCKFORGE_CF_MOCKS_EDGE_TOKEN` | the same value as the Worker's `EDGE_TOKEN` |

Without both, the registry keeps deploying every mock to Fly.

To pin the MockForge version, build with `--build-arg MOCKFORGE_IMAGE=ghcr.io/saasy-solutions/mockforge@sha256:...`
or edit the `ARG` default in `Dockerfile`.

## Local development

```sh
printf 'REGISTRY_INTERNAL_TOKEN=...\nEDGE_TOKEN=...\n' > .dev.vars
pnpm wrangler dev --var REGISTRY_URL:http://127.0.0.1:8080
pnpm type-check && pnpm test
```

`wrangler dev` runs the containers in local Docker.
