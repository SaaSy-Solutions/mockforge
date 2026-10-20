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

## Installed-guard verification, October 20, 2026

The shared host at 91.107.206.8 had 70.8 GiB free during this audit. Its cleanup
timer was active, and the service last completed successfully at 20:58:23 UTC.
The retired `/etc/cron.hourly/cleanup-runner-tmp` entry was absent.
The installed timer previously specified a 10-minute interval with a one-minute
randomized delay. Its original unit was backed up under `/root/ci-backups`, the
maintained timer passed `systemd-analyze verify`, and it was installed atomically.
After reloading systemd and restarting only `runner-host-cleanup.timer`, live
readback confirmed an active timer with `OnUnitActiveUSec=30min` and
`RandomizedDelayUSec=2min`. Its SHA-256 now matches the maintained unit:
`bf200ef0bf23d8b66c2d85c3565b141c48f55befb580ca2438b98d520f2be62e`.
The cleanup script and service unit retained their verified hashes.

The installed cleanup script SHA-256 is
`7d8a9c8398a7c12fa8d53ece6a75d8d23acada5d9191199aa81a26f4dff6ad5b`.
It exactly matches the infrastructure repository's reviewed revision
[`6994906a70670e97da2c5b1a4784ae356f6394e9`](https://github.com/SaaSy-Solutions/saas-platform/blob/6994906a70670e97da2c5b1a4784ae356f6394e9/infrastructure/github-runners/host-shared/runner-host-cleanup.sh).
That version already includes the corrected worker/cwd checks, the nested
15-minute target-activity backstop requested in #706, and deferral of Docker
garbage collection to drained-host maintenance.

All 17 activity and cleanup-wiring checks passed against the installed script
on the host using isolated, owned fixtures. These cover fresh nested artifacts
under stale parent directories, genuinely idle workspaces, symlinks, failed or
timed-out scans, active workers, process working directories, and a job starting
during the scan. Ten additional local live/idle safety and scoped ownership
recovery checks passed against a byte-identical copy of the installed script.
The tests exercised extracted production sections in owned fixtures. No
whole-host cleanup command was run during the probes, and runner services and
active build artifacts were preserved.

Infrastructure revision
[`19cb0ba2b1932a392f6d49b9517929fb0334984d`](https://github.com/SaaSy-Solutions/saas-platform/commit/19cb0ba2b1932a392f6d49b9517929fb0334984d)
has cleanup-script SHA-256
`675157f43a4922edce5153c9816e0fa7f7ef5c1ffe5913613de1efd5393d2bdf`.
The sole script-content difference adds a separate native-observer `/run`
namespace sweep. Its rollout is separate from the verified #706 safeguards;
the existing cleanup script and service were preserved while the timer unit was
reconciled to the reviewed cadence.

Issue #706 remains open for investigation of the original missing-artifact
failure and verification of the restored-cache path in an actual CI run.
Installed guards and current headroom do not establish that historical cause,
nor do they prove restored Rust caches are intact.

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
