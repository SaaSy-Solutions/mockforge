# Local Rust build workflow

Use the checked-in launcher in Claude Code, Codex, Cursor, and interactive shells:

```bash
scripts/dev/cargo check -p mockforge-core
scripts/dev/cargo test -p mockforge-core
scripts/dev/cargo clippy -p mockforge-core --all-targets -- -D warnings
scripts/dev/cargo build -p mockforge-cli
```

A check supplies compiler feedback; completion also requires relevant tests,
feature configurations, security and integration checks, and existing required CI.
For registry changes, preserve PostgreSQL non-owner RLS and runtime verification.
Shared contracts and workspace configuration require downstream validation. Broad
Make targets retain their existing meaning; do not run them after every small edit.

## Isolation and resource admission

The Linux/WSL launcher uses this checkout's canonical `target` directory. It
rejects external `CARGO_TARGET_DIR`, target symlinks, `--target-dir`, and inline
Cargo configuration overrides. Compatible invocations reuse artifacts; separate
worktrees retain separate targets. It preserves compiler flags, profiles, feature
arguments, incremental compilation, wrappers, toolchain selection, and SQLx mode.
Cargo itself continues to invalidate artifacts when inputs change. Never execute
an artifact borrowed from another checkout as proof of current source behavior.

Compilation commands share one versioned host policy with participating SaaSy,
Aether, CodeDig, and Hive launchers. User-owned state is under
`${XDG_RUNTIME_DIR:-/run/user/<uid>}/saasy-dev-cargo-admission`. Every cooperating
client reads the same policy snapshot; environment variables cannot independently
increase host capacity. On a capable host the versioned fallback provides four
aggregate slots, with two reserved for measured, eligible lightweight checks.
Every command retains 12 GiB memory, no swap, 200% CPU quota and two Cargo jobs.
Four slots mean at most 48 GiB and 800% CPU in aggregate, not four checks plus two
heavy builds. Smaller hosts reduce capacity to fit visible resources.

Admission classes are separate from Cargo operation classes and their resource
limits. `LIGHT_CHECK` requires an explicit reviewed repository/package/command
recipe, compatible features/profile/targets/compiler inputs, and a successful
warm-target receipt. A targeted `cargo check` alone never qualifies. Unknown or
broader checks are `STANDARD_CHECK`; linked tests, executable/release builds and
dependency-heavy validation are `HEAVY_BUILD`. Standard checks and heavy builds
share the two ordinary slots on the initial policy; eligible light checks may use
all four. There is currently **no measured Mockforge light-check recipe**: its
checks conservatively use ordinary capacity. Do not add an allowlist entry based
solely on the command name or another repository's benchmark.

A shared queue admits older eligible requests first while allowing reserved
capacity to serve light checks when ordinary slots are occupied. Cancellation and
process death release leases; descendants keep their inherited admission leases
until exit. Queue waiting and execution remain separately observable. Fairness
covers participating launchers only, not direct Cargo or old clients. There is no
preemption: finite progress assumes admitted commands finish and resource pressure
clears. No completion deadline can be guaranteed for a hung child.

Total capacity, reservations, class limits and reviewed light recipes remain
configurable within finite host CPU/memory budgets. The initial reservation is a
provisional policy, not a demonstrated optimum for mixed workloads. No-swap
scopes remain mandatory. Unsupported or malformed policies fail closed.

A policy lease covers both queued and active commands. Scope construction and
kernel verification use that snapshot; compiler children inherit policy and slot
leases, including if their launcher is killed. An update refuses to proceed while
any cooperating request is queued/active, or a known legacy slot is held. This is
not safe rolling compatibility with old clients: an old launcher could start after
the update and ignore the new capacity or class reservation. Drain and upgrade
**all participating worktrees and repositories**, including old SaaSy launchers,
before changing policy; do not allow mixed clients to resume. No global process
interception or revocation of old launchers is claimed.

### Review and change host policy

```bash
python3 scripts/dev/rust_build_policy.py show
python3 scripts/dev/rust_build_policy.py example > /absolute/owned/policy-candidate.json
# Review and edit the complete candidate, drain all active/queued build callers,
# and verify every participating worktree uses this versioned contract.
python3 scripts/dev/rust_build_policy.py apply /absolute/owned/policy-candidate.json
```

The `apply` command changes user runtime admission state only, without installing
a daemon, touching compiler flags, clearing targets, or editing global WSL/shell
configuration. It takes an exclusive policy lease and probes legacy slots before
an atomic update; active or queued work causes refusal. Do not work around a
refusal by deleting locks. Runtime state may disappear at logout/reboot: keep the
reviewed candidate in backed-up project documentation and explicitly reapply it
when all clients are quiescent. All four modules must remain byte-identical.

An experiment may compare capacity one, two, and four, class CPU/jobs, and a check
reservation, provided aggregate budgets validate and actual resource conditions
permit it. Measure successful relevant validations per hour, admission and
execution latency, failures/timeouts, and useful-feedback tail latency. Choose
using representative data; fake-worker concurrency tests prove enforcement only.
No new application benchmark or optimal-concurrency claim is included here.

No installation or global Cargo interception is performed. Python 3.9+, Linux
`flock`, and a working user systemd manager are required. Start from this worktree
or a subdirectory. For format, metadata, and other non-compiling commands, the
launcher delegates directly. Release publishing, CI runner admission, nested
third-party build tools, direct Cargo, and legacy scripts not routed through the
launcher are outside this local policy. Use an explicitly reviewed workflow for
those cases. Do not infer host-wide enforcement from only these locks.

## Iteration and watchers

```bash
make check-focused PACKAGE=mockforge-core
make test-focused PACKAGE=mockforge-core
make clippy-focused PACKAGE=mockforge-core
make dev PACKAGE=mockforge-core
```

A long-running `scripts/dev/cargo run` retains admission for the entire server
lifetime, not just compilation. Prefer the existing managed executable build/start
workflow for persistent services; do not park a server in a build slot.

Run one watcher per task. Every watcher event acquires admission anew; an idle
watcher holds no slot. Run focused tests and Clippy explicitly when appropriate.
Do not run routine `cargo clean`, `cargo update`, release builds, overlapping
watchers, or repeated broad validation. The scoped pre-commit Clippy hook uses the
same launcher and retains all existing feature checks and fail-closed selection.

## Observation, validation, and rollback

The launcher emits one JSON summary to stderr per admitted invocation: command
name, hashed worktree identity, admission wait, child execution duration, and exit
status, operation class, admission class, classification reason, effective policy
fingerprint, capacity/reservation, and class limits. Execution starts after admission inside the scope and excludes
scope startup; it is not isolated compiler CPU time.
It does not report unchanged checks as compiler-cache hits or log environment
values, arbitrary command arguments, or secrets. Failed commands retain failure
status. Admission wait is not compilation time. No representative before/after
speedup has been measured for this change.

Behavior tests use fake Cargo and scope executables and real filesystem locks:

```bash
python3 -m unittest discover -s scripts/tests -p test_dev_cargo.py -v
python3 -m unittest discover -s scripts/tests -p test_rust_build_policy.py -v
```

Required CI and application features remain enforced. Revert this PR to roll
back launcher routing; preserve target artifacts and source work. Do not delete
caches or worktrees. Do not silently bypass the launcher when resource admission
fails; report the missing user-systemd prerequisite.

### Initial validation (October 22, 2026)

Twelve launcher behavior tests pass, including a killed-launcher regression that
proves a surviving child retains its lock. A dependency-free, session-owned Rust
fixture under `target/dev-cargo-smoke-20261022` also executed one real unit test
successfully twice. Its build script asserted actual kernel limits:
`memory.max=12884901888`, `memory.swap.max=0`, `cpu.max=200000 100000`.
Cargo reported 1.76 s initially and 0.04 s unchanged; launcher execution was
1.987 s and 0.112 s, with zero admission wait in both runs. These tiny-fixture
results demonstrate resource enforcement and compatible artifact reuse, not an
application performance improvement or representative benchmark. No application
Rust, production profiles, CI gates, dependencies, or compiler flags changed.

Existing CI optimization PRs #1354, #1350, and #1385 were inspected for overlap;
this change preserves their runner routing and source-freshness policies. The
existing lightweight scope-detection job also runs the launcher and shared-policy regressions;
no duplicate workflow or extra application compilation is added.
Legacy scripts (including the standalone RLS gate) and tool installation commands
still invoke Cargo directly. Their required validation remains intact, but their
callers must arrange reviewed admission; this PR does not claim complete global
interception or measured adoption by every agent process.

A cross-repository smoke also ran all three adapters alongside the existing SaaSy
admission wrapper, using isolated runtime locks, real kernel scopes, and fake
Cargo workers. Four workers exited successfully with observed peak admitted
concurrency two. This proves protocol interoperability, not application throughput.

### Configurable policy validation (October 23, 2026)

Fourteen adapter tests and eleven shared-contract tests pass. Idle subprocesses
verify capacity one/two/four and reserved-check progress while a heavy request
waits. Tests also cover active/queued/legacy update refusals, malformed policies,
finite aggregate budgets, workload classes and aliases/custom profiles, dynamic
CPU/memory verification, class jobs above two, and inherited policy/slot leases
after launcher death. The initial October 22 fixture measurements above describe
the former fallback smoke only; they do not measure this policy change or establish
which configuration maximizes useful validations per hour. No application compile
or host policy update was performed during this correction.

### Workload-aware policy update

The canonical SSD measurements and evidence are in SaaS Platform's
`docs/development/rust-build-performance-2026-10-22/SSD_RECALIBRATION.md` (PR #12852),
not in this repository. Four warm Aether shared-edit checks completed in median
6.76 seconds at four-way concurrency versus 12.24 seconds at two-way concurrency.
These measurements support eligible warm checks, not Mockforge heavy builds,
actual agent idle time, mixed-workload fairness performance, or a platform-wide
speedup. Increasing CPU quota did not repeatably help that measured workload.

Before applying policy, drain and upgrade every participating launcher/worktree.
Keep the prior reviewed configuration for rollback; update only while quiescent.
Do not globally intercept Cargo, delete lock state, or alter WSL limits. Observe
admission and execution latency, failed validations, memory/CPU/I/O pressure and
interactive responsiveness before increasing capacity or broadening light recipes.
