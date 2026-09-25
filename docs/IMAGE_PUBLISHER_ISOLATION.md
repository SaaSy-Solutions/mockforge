# GHCR publisher isolation

The image publisher needs a dedicated VM because the current CI hosts run untrusted PR work with rootful Docker. A rootful Docker client can mount host paths and read another runner's package-write credentials, regardless of Unix account separation.

Issue #1056 tracks provisioning. Keep `mockforge-image-publish` unregistered until the dedicated VM has:

- A restricted GitHub Actions runner group named `mockforge-image-publish`, allowed only for `ashburn-images.yml`, `docker-build.yml` at trusted refs, with the selected-workflow allowlist verified through the organization API.
- A `mockforge-image-publish` Unix account without sudo or Docker group membership, a private rootless Docker socket at `unix:///run/user/<uid>/docker.sock`, and private runner work/temp paths.
- A root-owned `/etc/mockforge-image-publish/isolated-host` marker containing exactly `SaaSy-Solutions/mockforge:mockforge-image-publish`. Confirm no PR runner is registered on this host.

The publisher jobs use `packages: write` only at job scope and private Docker auth directories. PR smoke has no GHCR write token. Verify the workflow attestation before any live publish; a queued job is the expected state until provisioning is complete.
