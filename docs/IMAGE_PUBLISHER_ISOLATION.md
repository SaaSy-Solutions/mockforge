# GHCR publisher isolation

MockForge image writes run only in protected `main` or protected `v*` release
jobs (`ashburn-images.yml` for the registry and tunnel relay, `docker-build.yml`
for the root `mockforge` image). They build on the self-hosted Hetzner runners
(`[self-hosted, linux, x64, rust]`), the same pool the no-write PR smoke build
uses.

## What each publishing job does

1. Checks out the exact commit with `persist-credentials: false`.
2. Points `DOCKER_CONFIG` at a fresh mode-0700 directory under `RUNNER_TEMP`,
   so the GHCR credential and buildx state never land in the shared runner
   user's `~/.docker`.
3. Creates a fresh `docker-container` BuildKit builder named for the run,
   attempt and image, capped at 20 GiB like the PR smoke builder.
4. Logs in to GHCR with the job's ephemeral `GITHUB_TOKEN` (package write) and
   pushes `ghcr.io/saasy-solutions/<image>:<full sha>`. The build uses the
   image's `:buildcache` registry cache, which only these publishing jobs write.
5. Records the immutable digest (`<image> ghcr.io/...:<sha>@sha256:...`), which
   the Ashburn deploy pins.
6. In an `if: always()` step, removes the builder, logs out, and deletes the
   private config directory.

Matrix builds are serial, and all image builds share the
`mockforge-image-builds` concurrency group with the PR smoke build.

## Threat model

Until 2026-09-30 publishing ran on a disposable Fly Machine so that no package
write shared a host with rootful PR runners (#1056). That dependency was
removed on request: the Fly publisher's SSH handoff timed out repeatedly, and
Fly is no longer part of the CI estate.

The accepted residual risk is that a job running as the same runner user on the
same host (for example a PR build) could, while a publish is in progress, read
the job's `GITHUB_TOKEN` or interfere with its BuildKit container through the
shared Docker daemon. Mitigations: the token is ephemeral and scoped to the
job, the builder and credential exist only for the job's lifetime, the build
cache is written only by publishing jobs, and the deploy pins the recorded
digest rather than a mutable tag. If this risk stops being acceptable, run the
publishing jobs on a dedicated runner (its own host, rootless Docker, a label
no other workflow uses) and switch `runs-on` in both workflows.

## Triggers

The workflows accept only pushes to protected `main` and protected `v*` tags,
or a manual run on protected `main`. Automatic publishing on push is gated by
the `MOCKFORGE_AUTO_IMAGE_PUBLISH` repository variable; manual dispatch always
works. The `Trusted MockForge release tags` ruleset limits `v*` creation,
updates and deletion to organization admins. Review the release commit before
creating a tag.
