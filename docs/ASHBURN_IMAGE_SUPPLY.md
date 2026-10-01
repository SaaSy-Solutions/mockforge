# MockForge images for Ashburn

The repository publishes the root `Dockerfile` to
`ghcr.io/saasy-solutions/mockforge` through `docker-build.yml`. The root image
supplies both the persistent `mockforge-server` and the `mockforge-demo` services.
`ashburn-images.yml` publishes `Dockerfile.registry` as
`ghcr.io/saasy-solutions/mockforge-registry`, `Dockerfile.runner` as
`ghcr.io/saasy-solutions/mockforge-test-runner`, and `Dockerfile.tunnel` as
`ghcr.io/saasy-solutions/mockforge-tunnel-relay`. Both workflows build on the
self-hosted runners (see `IMAGE_PUBLISHER_ISOLATION.md`) and share one
concurrency group. Production image tags are
source commit SHAs; Ashburn should pin the resulting immutable digests.

The demo's Fly configuration supplies a process command that is **not part of
the root image**. When moving the Ashburn `mockforge-demo` Compose entry from
its Fly image to the root GHCR image, set its command to:

```yaml
command:
  - serve
  - --spec
  - /app/examples/scenarios/ecommerce-store/openapi.json
  - --spec
  - /app/examples/scenarios/weather-geo/openapi.json
  - --spec
  - /app/examples/scenarios/chat-api/openapi.json
  - --admin
```

Keep `/var/lib/saasy/mockforge-server:/data` on the persistent server entry.
Before removing Fly registry access, verify a clean host can pull all three
GHCR digests and start registry, server, demo, and tunnel relay with healthy
endpoints. The existing root GHCR publisher uses `main-<sha>` tags rather than
bare SHA tags, so resolve that image's digest from its published tag.

## Cloud test runner

`mockforge-test-runner` consumes the registry's `test_runs:queued` Redis list
and executes cloud test, chaos and capture-replay runs. It has no listener and
publishes no port. It reads exactly these variables
(`crates/mockforge-test-runner/src/config.rs`):

| Variable | Required | Ashburn value |
| --- | --- | --- |
| `MOCKFORGE_RUNNER_REDIS_URL` | yes | The same Redis URL as the registry's `REDIS_URL` |
| `MOCKFORGE_RUNNER_REGISTRY_INTERNAL_BASE_URL` | yes | `http://mockforge-registry:8080` (Compose service address, not the public URL) |
| `MOCKFORGE_RUNNER_REGISTRY_INTERNAL_TOKEN` | yes | The registry's `MOCKFORGE_INTERNAL_API_TOKEN` |
| `MOCKFORGE_RUNNER_QUEUE_KEY` | no | Leave unset. The registry always pushes to `test_runs:queued` |
| `MOCKFORGE_RUNNER_MAX_CONCURRENT_JOBS` | no | Default 4 |
| `MOCKFORGE_RUNNER_POLL_TIMEOUT_SECS` | no | Default 5 |
| `MOCKFORGE_SSRF_ALLOW_LOOPBACK` | no | `0` in production. `1` only for local tests |

The registry only enqueues when its own `REDIS_URL` is set. Without it a run is
created as `queued` and nothing is pushed, so a runner alone does not make runs
execute.

The runner reaches hosted mocks by their public URLs (`*.mocks.mockforge.dev`
and the Fly `https://<app>.fly.dev` origin), which pass the strict SSRF policy.
Chaos toggles for `target_kind=hosted_mock` do not go to the mock directly: the
runner calls the registry's `/api/v1/internal/hosted-mocks/{id}/chaos` with the
run's `run_id`, and the registry checks the deployment belongs to that run's
org before forwarding.
