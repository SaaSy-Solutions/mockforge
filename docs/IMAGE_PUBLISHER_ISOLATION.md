# GHCR publisher isolation

MockForge image writes run only in protected `main` or protected `v*` release
jobs. The GitHub-hosted job creates a disposable Fly Machine from a pinned
`moby/buildkit:rootless` image, builds with UID 1000 through `buildctl`,
records the immutable GHCR digest, and destroys the Machine. No self-hosted
publisher runner is registered on the old rootful CI host. PR smoke retains
its separate no-write build path.

Provision a new `mockforge-image-publisher` Fly app in the existing MockForge
organization, with no public service. Store a **deploy token scoped only to
that app** as the repository secret `FLY_IMAGE_PUBLISHER_TOKEN`. The existing
production Fly token must not be used. The trusted job's ephemeral
`GITHUB_TOKEN` has package-write permission; the script writes a mode-0600
Docker config, transfers it over Fly SSH/SFTP, and deletes it before destroying
the guest. Source comes from `git archive HEAD` after an exact SHA check.

Each image build creates an 8-vCPU/64-GiB/100-GB-rootfs Machine with a unique
run/attempt/image name. `finally` destroys the returned ID directly and
repeatedly checks the unique name; an `if: always()` workflow step repeats the
check. The hourly `fly-publisher-cleanup.yml` job destroys only publisher-named
Machines older than six hours, beyond the 90-minute build timeout. Check app
inventory after an API outage or failed cleanup job; a stopped Machine's
rootfs still costs money. Matrix builds are serial and the root
publisher shares a GitHub concurrency group with registry/tunnel publishing.

Before enabling full publishing, prove one protected-main canary with the
app-scoped token: Machine creation, SFTP handoff, BuildKit registry cache,
GHCR push and immutable digest, root image metadata aliases/signature/SBOM,
Ashburn read-token pull, and zero remaining builder Machines. A secret-free
2-vCPU/4-GiB proof on 2026-09-26 ran rootless BuildKit and exported a real
Alpine build; rootless Docker daemon failed on denied tap setup. The proof app
and Machines were destroyed. See #1056 for the supply and canary gate.

The workflow accepts only pushes to protected `main` and protected `v*` tags,
or a manual run on protected `main`. The active `Trusted MockForge release
tags` ruleset limits `v*` creation, updates, and deletion to organization
admins. Review the release commit before creating a tag. The old selected
self-hosted runner workflow allowlist no longer applies because the package
write job is on GitHub-hosted infrastructure.
