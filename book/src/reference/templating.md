# Templating Reference

MockForge supports lightweight templating across HTTP responses, overrides, and (soon) WS/gRPC). This page documents all supported tokens and controls.

## Enabling

- Environment: `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true|false` (default: false)
- Config: `http.response_template_expand: true|false`
- CLI: `--response-template-expand`
- Determinism: `MOCKFORGE_FAKE_TOKENS=false` disables faker token expansion.

## Time Tokens

- `{{now}}` — RFC3339 timestamp.
- `{{now±And|Nh|Nm|Ns}}` — Offset from now by Days/Hours/Minutes/Seconds.
  - Examples: `{{now+2h}}`, `{{now-30m}}`, `{{now+10s}}`, `{{now-1d}}`.

## Random Tokens

- `{{rand.int}}` — random integer in [0, 1_000_000].
- `{{rand.float}}` — random float in [0,1).
- `{{randInt a b}}` / `{{rand.int a b}}` — random integer between a and b (order-agnostic, negatives allowed).
  - Examples: `{{randInt 10 99}}`, `{{randInt -5 5}}`.

## UUID

- `{{uuid}}` — UUID v4.

## Request Data Access

- `{{request.body.field}}` — Access fields from request body JSON.
  - Example: `{{request.body.name}}` extracts the `name` field from request body.
- `{{request.path.param}}` — Access path parameters.
  - Example: `{{request.path.id}}` extracts the `id` path parameter.
- `{{request.query.param}}` — Access query parameters.
  - Example: `{{request.query.limit}}` extracts the `limit` query parameter.

## Faker Tokens

Faker expansions can be disabled via `MOCKFORGE_FAKE_TOKENS=false`.

- Minimal (always available): `{{faker.uuid}}`, `{{faker.email}}`, `{{faker.name}}`.
- Extended (when feature `data-faker` is enabled):
  - `{{faker.address}}`, `{{faker.phone}}`, `{{faker.company}}`, `{{faker.url}}`, `{{faker.ip}}`
  - `{{faker.color}}`, `{{faker.word}}`, `{{faker.sentence}}`, `{{faker.paragraph}}`

## Where Templating Applies

- HTTP (OpenAPI): media-level `example` bodies and synthesized responses.
- HTTP Overrides: YAML patches loaded via `validation_overrides`.
- WS/gRPC: provider is registered now; expansion hooks will be added as features land.

## Status Codes for Validation Errors

- `MOCKFORGE_VALIDATION_STATUS=400|422` (default 400). Affects HTTP request validation failures in enforce mode.

## Security & Determinism Notes

- Tokens inject random/time-based values; disable faker to reduce variability.
- For deterministic integration tests, set `MOCKFORGE_FAKE_TOKENS=false` and prefer explicit literals.
