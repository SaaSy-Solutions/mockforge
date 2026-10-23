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

Compilation commands share two host admission slots with participating SaaSy,
Aether, CodeDig, and Hive launchers. Locks are private, user-owned files under
`${XDG_RUNTIME_DIR:-/run/user/<uid>}/saasy-dev-cargo-admission`; slot 3 is reserved.
The launcher waits without starting compilation, then holds its lock through the
child lifetime. User systemd scopes bound each admitted invocation to two CPU
cores, 12 GiB memory, no swap, and at most two Cargo jobs. Missing scope support
fails closed. These are provisional safety limits, not a measured throughput
optimum. Twenty agents do not imply twenty simultaneous compilations.

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

Run one watcher per task. Every watcher event acquires admission anew; an idle
watcher holds no slot. Run focused tests and Clippy explicitly when appropriate.
Do not run routine `cargo clean`, `cargo update`, release builds, overlapping
watchers, or repeated broad validation. The scoped pre-commit Clippy hook uses the
same launcher and retains all existing feature checks and fail-closed selection.

## Observation, validation, and rollback

The launcher emits one JSON summary to stderr per admitted invocation: command
name, hashed worktree identity, admission wait, child execution duration, and exit
status. Execution includes scope startup and is not isolated compiler CPU time.
It does not report unchanged checks as compiler-cache hits or log environment
values, arbitrary command arguments, or secrets. Failed commands retain failure
status. Admission wait is not compilation time. No representative before/after
speedup has been measured for this change.

Behavior tests use fake Cargo and scope executables and real filesystem locks:

```bash
python3 -m unittest discover -s scripts/tests -p test_dev_cargo.py -v
```

Required CI and application features remain unchanged. Revert this PR to roll
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
this change leaves their workflows and source-freshness policies untouched.
Legacy scripts (including the standalone RLS gate) and tool installation commands
still invoke Cargo directly. Their required validation remains intact, but their
callers must arrange reviewed admission; this PR does not claim complete global
interception or measured adoption by every agent process.
