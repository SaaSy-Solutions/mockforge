# Cloud Tunnel Relay Boundary

The cloud relay does not yet implement authenticated reverse client transport.
A registry reservation claims a name; it does not connect a client or authorize
forwarding to a relay-local URL. Do not deploy this as a working public tunnel.

## Current behavior

`Dockerfile.tunnel` builds `tunnel-server` with `cloud-validation`. That build
requires nonempty `REGISTRY_URL` and `MOCKFORGE_INTERNAL_API_TOKEN` before opening
storage or a listener. `REGISTRY_URL` must be an HTTP(S) origin without embedded
credentials, a path, a query, or a fragment. Missing configuration never selects
the unrestricted local store.

The cloud wrapper refuses management requests with HTTP 501. Host-based and
ID-based forwarding return HTTP 404, including for entries already present in
local storage. Registry reservations never materialize a synthetic destination,
and old cached destinations cannot bypass registry checks. Refusal preserves
existing stored entries. `/health` remains available; its success proves process
health, not a connected tunnel.

The standalone build without `cloud-validation` retains its direct forwarding
API for trusted local development. That API is not cloud tenant authentication.

## Registry lookup contract

The relay uses:

```text
GET /api/v1/internal/tunnel-reservations/by-subdomain/{subdomain}
Authorization: Bearer ${MOCKFORGE_INTERNAL_API_TOKEN}
```

A successful response with `status: "reserved"` proves only a name claim.
Transport errors, invalid responses and other statuses refuse the lookup.
Implementation: `mockforge-registry-server::handlers::internal_test_runs::get_tunnel_reservation_by_subdomain`.

## Work required before public deployment

Implement an authenticated reverse connection initiated by the user's client,
bound to an authorized organization and reservation. Forward only through that
connection, enforce revocation and connection lifecycle, and bound requests,
responses and resource use. Custom-domain routing must require verified ownership.

Qualification must exercise an owned backend through public DNS and TLS, prove
that another tenant and revoked credentials cannot connect or forward, and prove
cleanup after disconnect. Local relay fixtures and reservation lookups do not
satisfy those checks. No public-tunnel success is claimed here.

## Verification

Run the supported Cargo launcher:

```bash
scripts/dev/cargo test --locked -p mockforge-tunnel --all-features
```

`cloud_connection_boundary` exercises reservation-only refusal, legacy-cache
refusal and actual HTTP management/forwarding boundaries. The remaining package
tests cover the standalone relay. This verifies containment, not reverse transport.
