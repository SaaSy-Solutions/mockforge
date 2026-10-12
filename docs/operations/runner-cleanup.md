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
