# Protocol compilation boundary

- [x] Ground: current architecture, active changes, exact dependency/source closure.
- [x] Sketch: compare independent viable boundaries and canonical type ownership.
- [x] Agree: choose shape and record rejected alternatives.
- [ ] Implement: source-preserving move, public facade and publish/CI ownership.
- [ ] Verify: native behavior, strict Clippy, reverse consumers, runtime/feature gates.

Baseline: `3871677f241ee8e9726ad7646fa356c0a2df04ca`.

The user authorizes ongoing Rust build-performance work across repositories.
SaaS billing/budget qualification runs in parallel; this is an isolated MockForge
worktree. Keep core authentication/logging/latency adapters and their authority
in place. Foundation already owns canonical Error/Result and Protocol; never
copy them or introduce a dependency back to core. Preserve matching, routing,
streaming, fixture parsing, feature behavior and consolidated integration suites.

Candidate: the five-file protocol-abstraction vocabulary/registry/matching closure
(2610 physical lines, 46 literal tests). These are source counts, not compiler
measurements. Audit all crate::/super references and module reexports before moving.

Rubric: canonical types and cycle-free layering; useful independently tested
boundary; stable public imports; preserved feature/auth/runtime behavior; minimal
reviewable migration; complete publishing/member/CI validation. Use the checked-in
Cargo launcher, own persistent targets and existing host policy, with serial
compilation per agent. No overrides, cache cleanup, releases or publishing.
