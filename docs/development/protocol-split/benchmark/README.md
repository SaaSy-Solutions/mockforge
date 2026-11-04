# MockForge TCP incremental compilation probe

For this bounded synthetic warm-check workload, execution time decreased from
**7.955 seconds to 0.724 seconds median**, five runs per side. Before range:
6.860–10.803 seconds; after range: 0.643–0.969 seconds. These execution measurements
exclude shared admission. They do not describe full CLI builds, runtime throughput,
cold builds, or total developer wait time.

## Workload and controls

- Before: `3871677f241ee8e9726ad7646fa356c0a2df04ca`.
- After: `56dc1ffa2dabdcaf67d914cf6dc85ce41f9b7166`.
- Separate detached worktrees and each checked-in launcher's canonical `target/`.
- Same host and installed Rust 1.96.0, default TCP features `[]`, development
  profile opt-level 0, debuginfo 2, debug assertions and overflow checks enabled.
- Command: `scripts/dev/cargo check --locked -p mockforge-tcp --lib --message-format=json`.
- Both sides warmed successfully, followed by successful completely fresh checks.
- For each pair, replaced exactly the AuthMiddleware constructor's diagnostic name
  with `AuthMiddlewareBenchmarkPairN`, identically on both endpoints. This is a
  real body edit, not a comment; it changes a middleware display name, not an auth
  decision. No application or API was run and no temporary edit was committed.
- Five paired orders: AB, BA, AB, BA, AB. No failed trials were discarded.
- Same policy SHA, two Cargo jobs, 200% CPU quota and 12 GiB memory. No toolchain,
  resource, flags, target, feature, compiler-wrapper or profile override was used.
  No shared compiler cache wrapper was configured. Unique body values plus actual
  fresh=false Core artifacts prove the baseline compiler work was not skipped.

## Actual results

| Pair | Order | Before execution s | After execution s | Before admission s | After admission s |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | AB | 10.803 | 0.969 | 371.507 | 405.531 |
| 2 | BA | 6.895 | 0.720 | 0.000 | 0.000 |
| 3 | AB | 7.955 | 0.789 | 0.000 | 0.000 |
| 4 | BA | 8.680 | 0.724 | 0.001 | 0.000 |
| 5 | AB | 6.860 | 0.643 | 199.953 | 0.001 |

All ten commands exited successfully and emitted successful build-finished
receipts. In every before trial, exactly Core and TCP were nonfresh. In every
after trial, Core was absent from the selected graph and TCP and every dependency
artifact remained fresh. The command still checked the actual TCP package.

The ratio of execution medians is approximately 11.0; the reduction is 90.9% for
this probe. Prefer the explicit seconds and workload rather than presenting that
ratio as a general product improvement.

## Why the number moved

An unreported profiling pilot sampled only compiler/Cargo processes whose working
directory belongs to the owned benchmark worktree, plus its own bounded wrapper
scope. Core consumed at least 3.57 CPU seconds over a sampled 3.76-second span;
TCP consumed 0.12 CPU seconds and Cargo 0.45. The scope accumulated 4.244 CPU
seconds without CPU quota throttling during a 4.637-second execution. This identifies
Core's frontend check as the dominant removed work; the after graph excludes it.
The counters are sampled lower bounds, not a per-crate exact timing decomposition.

An initial profiling attempt captured no processes because it matched only the
workspace root, while Cargo runs rustc in member directories. Those samples were
rejected, the matcher was corrected, and the unreported pilot repeated before
starting measured trials. The original source was restored and rewarmed again.
The scope's io.stat was unavailable; the monitor records that missing data rather
than inventing I/O measurements.

## Noise and practical limits

This was a busy 16-core host, with most trial endpoint one-minute loads around
13–17. Global sampled I/O-wait fractions ranged roughly 0.2–2.1%; snapshots for
queued commands include the queue interval and are not isolated compiler profiles.
Interleaving limits temporal bias, and measured execution ranges do not overlap,
but the variation prevents treating the median as a portable prediction.

Admission was the dominant cost of three commands. Pair 1's actual outer wall was
382.5 seconds before and 406.8 seconds after, despite lower execution afterward.
Therefore this experiment does not demonstrate a reduction in overall developer
wait when competing jobs occupy admission slots. Wall and queue times remain in
the raw results. Warm-up times were excluded and are not comparable cold-build
measurements.

## Reproducibility and restoration

The published runner is [benchmark_protocol_rebuilds.py](../../../../scripts/dev/benchmark_protocol_rebuilds.py).
It expects the two named detached benchmark worktrees in `/mnt/build/worktrees`
at the exact commits above. Run its `self-test`, `qualify` and `trials` phases in
that order. The runner refuses dirty/unpinned source, compiler overrides, policy
drift, unexpected rebuilds and insufficient disk. It restores the original auth
source on completion or interruption. It writes local evidence under
`/tmp/mockforge-protocol1477-probe`.

The [full evidence archive](evidence.tar.xz) preserves the measured script,
identities, all ten Cargo JSON/stderr/results and profiling samples. Its SHA-256
is `a9b90097783bf646b557e376a859d2ad8e5d62a8fc750f547368de1588976878`.
[Summary](summary.json) retains each queue, execution and outer-wall result.
Both benchmark worktrees finished clean with original auth SHA-256
`234ac7847cfc07ce55d3be839f63e51f2827abfefd4ef665b5e39ced5f0434fc`.
No application ran, no temporary source edit was committed, and no cache was
shared or cleared. Targets occupied about 1.3 GiB before and 417 MiB after.
Available disk stayed above the 60 GiB floor.

The exact measured script is preserved as `captured-probe.py` (SHA-256
`21ef4be8b4360b991f142bd7bb4535f88a81a71cc79697c6be17d849197ca0e8`).
The publication candidate adds an automatic positive owned Core CPU-delta guard
before qualification, plus cheap empty/no-Core/zero-Core rejection tests. Its
SHA-256 is `4f1c8902466cd10ed1e0b78078482111347a30f149362bbf078cee3f4b541750`.
The actual pilot passes with 3.47 seconds sampled CPU delta and 3.57 seconds
maximum observed Core CPU. AST comparison proves command, runner, perturbation,
freshness checks, setup and sampling functions are unchanged. No Rust measurement
was rerun for this post-capture qualification guard. See `qualification-guard.json`.
