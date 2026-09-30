# MockForge images for Ashburn

The repository publishes the root `Dockerfile` to
`ghcr.io/saasy-solutions/mockforge` through `docker-build.yml`. The root image
supplies both the persistent `mockforge-server` and the `mockforge-demo` services.
`ashburn-images.yml` publishes `Dockerfile.registry` as
`ghcr.io/saasy-solutions/mockforge-registry` and `Dockerfile.tunnel` as
`ghcr.io/saasy-solutions/mockforge-tunnel-relay`. Both workflows build in
disposable rootless BuildKit Fly Machines and share one concurrency group. Production image tags are
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
