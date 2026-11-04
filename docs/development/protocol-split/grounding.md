# Protocol contracts and current ownership

Baseline: `3871677f241ee8e9726ad7646fa356c0a2df04ca` (main).

Core owns protocol-neutral request/response vocabulary, adapter and middleware
traits, registries, matching and streaming. Server crates import this vocabulary
through core. Protocol already comes from foundation, and core Error/Result also
reexport foundation: keep those canonical owners and preserve type identity.

The five candidate files are protocol_abstraction/{mod,base_registry,matcher,
protocol_registry,streaming}.rs, about2610 physical lines/46 literal test attributes.
They include filesystem fixture loading and async/runtime support; do not call
this a dependency-free or IO-free package. Foundation itself has runtime/HTTP
dependencies. The performance boundary is independent contract/test compilation
and removing unnecessary core back-edges, not a claimed benchmark improvement.

Auth and middleware adapter files remain in core because they use core config,
logging and latency authority. Any new crate must never depend on core. Preserve
core::protocol_abstraction and root reexports, including canonical foundation
Error/Result/Protocol. Existing auth extraction1329 and integration-consolidation1455
are merged; no openPR overlaps were observed in the initial audit.

Compare a dedicated protocol-contract crate with viable existing-foundation or
smaller matching/registry alternatives. Trace public/private imports, trait/type
orphan constraints, test-only references, feature propagation, all realcallers,
publish topological order and existing CI. Preserve auth/routing/matching/streaming
behavior. Strict Clippy and affected reverse-consumer checks are required, followed
by relevant consolidated core/HTTP integration tests and runtime checks as needed.

Use scripts/dev/cargo, Rust1.96 and this worktree's canonical target. No shared
mutable target, broad release build, cache deletion, feature/profile/compiler
flag change or policy override. Publishing/member list updates are source-only;
no release, version bump or crates.io publication is authorized by this task.
