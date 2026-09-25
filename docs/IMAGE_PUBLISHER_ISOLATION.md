# GHCR publisher isolation

The image publisher needs a dedicated VM because the current CI hosts run untrusted PR work with rootful Docker. A rootful Docker client can mount host paths and read another runner's package-write credentials, regardless of Unix account separation.

Issue #1056 tracks provisioning. Keep `mockforge-image-publish` unregistered until the dedicated VM has:

- A restricted GitHub Actions runner group named `mockforge-image-publish`, limited to this repository. Set `restricted_to_workflows: true` and allow these exact workflow refs:
  - `SaaSy-Solutions/mockforge/.github/workflows/ashburn-images.yml@refs/heads/main`
  - `SaaSy-Solutions/mockforge/.github/workflows/docker-build.yml@refs/heads/main`
  - Before each `v*` release tag is pushed, add its exact ref, for example `SaaSy-Solutions/mockforge/.github/workflows/docker-build.yml@refs/tags/v1.2.3`. Verify the selected-workflow allowlist through the organization API before creating the tag. Remove retired tag entries after the build. The tag job queues if its exact ref is absent.
- A `mockforge-image-publish` Unix account without sudo or Docker group membership, a private rootless Docker socket at `unix:///run/user/<uid>/docker.sock`, and private runner work/temp paths.
- A root-owned `/etc/mockforge-image-publish/isolated-host` marker containing exactly `SaaSy-Solutions/mockforge:mockforge-image-publish`. Confirm no PR runner is registered on this host.

The publisher jobs use `packages: write` only at job scope and private Docker auth directories. PR smoke has no GHCR write token. Verify the workflow attestation before any live publish; a queued job is the expected state until provisioning is complete.

Runner-group selected workflows are pinned to a branch, tag, or SHA; a `v*` wildcard is not a selected-workflow ref. The group controls which workflow ref can use the publisher. The workflow also requires `github.ref_protected`, so a release tag cannot publish unless a tag ruleset protects it. The active `Trusted MockForge release tags` ruleset limits `v*` creation, updates, and deletion to organization admins. Review the release commit before adding its exact tag ref to the group. The workflow accepts only pushes to protected `main` and protected `v*` tags, or a manual run on protected `main`.
