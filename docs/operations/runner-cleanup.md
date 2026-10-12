# Shared runner cleanup ownership

MockForge shares its self-hosted CI machines with other repositories. Host-wide
cleanup belongs to the infrastructure repository, not to this consumer repo:

- [Maintained script](https://github.com/SaaSy-Solutions/saas-platform/blob/main/infrastructure/github-runners/host-shared/runner-host-cleanup.sh)
- [Deployment and rollback runbook](https://github.com/SaaSy-Solutions/saas-platform/blob/main/infrastructure/github-runners/host-shared/README.md)
- [Maintained systemd units](https://github.com/SaaSy-Solutions/saas-platform/tree/main/infrastructure/github-runners/host-shared)

The former MockForge copy incorrectly claimed to be the shared source of truth.
Its worker pattern required a literal `bin.` even though workers can run through
`bin/Runner.Worker`; its activity check only inspected top-level target directory
mtime, which can remain old while compilation writes nested artifacts. It also
claimed ambiguity handling that it did not implement. Installing that copy could
replace newer host protections. The entry point now stops with exit status 64
without performing cleanup, and the obsolete installable units are removed.

Use the infrastructure runbook to compare the installed script checksum with the
reviewed revision and deploy only the host-owned files. Do not run cleanup or
replace a timer merely to resolve a queued MockForge job. Check runner occupancy
and disk headroom first; do not delete another job's build artifacts.

## Read-only audit, October 11, 2026

The shared host at 91.107.206.8 had 99 GB free (66% used), an active cleanup timer,
and a successful last cleanup invocation. The retired hourly target-cleanup
script was absent. The installed host script includes the corrected worker
pattern and a second guard for processes with their working directory inside a
candidate workspace.

A deployment gap remains: installed script SHA-256
`3f73e506431bfbf50695713d092ace9498a60e86c9492dfa2d24d94a943c7834`
differs from maintained source SHA-256
`05296d0ac079ec2ff32839f3b801b5aaea18a290031b51f39c2e2804391a23fd`.
The installed aggressive path still runs Docker system prune; the maintained
source defers Docker garbage collection until runners are drained. No host
cleanup or deployment was performed during this audit.

Issue #706 remains open for infrastructure deployment verification, the requested
recent nested-activity backstop, and investigation of the original missing-artifact
failure. Current headroom and worker guards do not establish the cause of that
historical failure, nor do they prove restored Rust caches are intact.

## Integration cache readiness

The integration workflow runs `scripts/ci/check_rust_cache.py` before and after
Rust-cache restoration, before starting Cargo. Both checks are read-only. They
require at least 10 GiB free on the checkout filesystem and report the restored
target's logical size; a target above 30 GiB produces a capacity warning instead
of an automatic deletion. Headroom is an observation at that instant, not a
reservation against other runners' later allocations.

The structural check examines completed library fingerprints. When Rust's
cached dep-info declares library outputs, each declared artifact must be a
nonempty regular file. This detects a missing `.rlib` even when its `.rmeta`
still exists. Cached dep-info can contain another runner's absolute paths:
only known output basenames are matched against the corresponding profile's
`deps` directory in this checkout. The workflow pins `CARGO_TARGET_DIR` to the
checkout's `target` so the inspected and compiled paths agree.
Those stored paths are never opened. Cargo-check metadata and procedural-macro
libraries are valid cache entries. Binary fingerprints are excluded because
Rust-cache deliberately prunes profile-level executables.

Targets and nested directory symlinks are rejected. The guard neither repairs
nor deletes caches, including active or foreign artifacts. Run it before Cargo
starts, as the workflow does; files changing during inspection can cause a
readiness failure. A failure stops this job before compilation and prints a
specific diagnostic. Resolve capacity through the infrastructure-owned process,
or invalidate a proven inconsistent cache through the CI cache controls.

This is a structural preflight, not a checksum verification of every artifact or
a guarantee against later I/O failures. It does not establish the cause of the
historical missing-rlib incident. Unit fixtures cover pressure, deleted and empty
artifacts, metadata-only caches, procedural macros, cross-target profiles and
symlink boundaries without modifying foreign data.
