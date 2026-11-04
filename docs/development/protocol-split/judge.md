# Cross-judge: seven-file protocol boundary

Baseline: `3871677f241ee8e9726ad7646fa356c0a2df04ca`. Both candidate documents and
AGENTS/CLAUDE instructions were read, followed by the actual seven source files'
imports, submodules, feature references, TCP/SMTP manifests and references,
foundation's error/features, core's adapter ownership and the publish/CI scripts.
No Cargo/build/metadata command or native execution was performed by this judge.

## Decision

Choose the expanded seven-file design shared by both candidates. Use B's
feature-preserving expanded variant as the behavioral base, with A's shorter
`mockforge-protocol` package name and precise public facade spelling. The five-
file versions are not the chosen wave: the two extra helpers close TCP/SMTP's
real dependency graph and make this more than preparation for a later migration.

The selected package has public `fixture_store`, `protocol_abstraction` and
`protocol_server` modules plus the existing protocol root reexports. Foundation
remains the only owner of Error/Result/Protocol. Core retains auth, logging,
metrics and latency adapters. TCP and SMTP replace their normal core edge with
this package and update only crate prefixes in their existing references.

Scores grade the proposals as written, not unexecuted verification results:

| Criterion (5 best) | A expanded | B expanded | Reason |
| --- | ---: | ---: | --- |
| Coherent closed source ownership | 5 | 5 | Contracts, generic fixture loading and lifecycle vocabulary close together |
| Canonical identity and feature preservation | 3 | 5 | A explicitly omits scripting preservation; B identifies the public Error capability |
| Actual graph benefit | 5 | 5 | Expanded variants can remove both TCP/SMTP normal core edges |
| Runtime/auth authority | 5 | 5 | Both leave concrete core adapters and listener/TLS/mailbox behavior in place |
| Publication and CI ownership | 5 | 5 | Both preserve topological publication and derived reverse-consumer scope |
| Native/feature/runtime proof scope | 4 | 5 | B's isolated feature-mode checks address feature-unification masking |
| **Total** | **27** | **30** | Graft A's naming/facade precision into B's behavior-preserving shape |

## Mandatory feature graft

A's statement that TCP/SMTP need not retain an unused transitive scripting
feature is not safe under this task's public compatibility requirement.
`mockforge-core/Cargo.toml` enables `scripting` by default and forwards it to
foundation. `mockforge-foundation/src/error.rs:190` gates the public
`Error::JavaScript` variant and its `From<rquickjs::Error>` implementation on that
feature. TCP/SMTP currently depend on core without disabling its defaults.
Their Error/Result return surfaces therefore expose this capability even when
those server implementations do not directly construct the variant.

Preserve it explicitly, adapting B's package name:

```toml
# mockforge-protocol
[features]
default = []
scripting = ["mockforge-foundation/scripting"]
schema = ["mockforge-foundation/schema"]

# TCP and SMTP: their old core dependency defaults were unconditional.
mockforge-protocol = { path = "../mockforge-protocol", features = ["scripting"] }

# Core: preserve its existing no-default mode and existing direct forwarders.
mockforge-protocol = { path = "../mockforge-protocol", default-features = false }
```

The new package's foundation dependency is default-off. Retain core's existing
`scripting` and `schema` forwarding exactly. Foundation's defaults are empty;
normal core dependencies inspected do not enable foundation/schema implicitly.
Its only inspected schema forwarders are explicit core/intelligence features.
Do not add schema to TCP/SMTP defaults. Scripting retains QuickJS for these two
consumers; the graph benefit is removal of core/intelligence/openapi dependencies,
not removal of all expensive dependencies.

## Source and visibility checks

The exact seven-file closure is 3030 physical lines and 56 literal test
attributes: five abstraction files (2610/46), fixture_store (277/7), and lifecycle
(143/3). The lifecycle adds one Tokio test and a Tokio watch receiver in its public
trait. These are planning counts requiring actual native execution.

All abstraction submodules were checked. Only auth and middleware declarations
and their two adapter reexport lines leave the moved mod.rs; no other production
or test body in it references those adapter types. The remaining external
in-crate references resolve through the new root's foundation Error/Result aliases,
its public protocol_abstraction module and root MessagePattern reexport. Relative
ResponseStatus and Protocol lookups retain their historical hierarchy. No
pub(crate)/pub(super), include/path macro, manifest-directory or external fixture
coupling was found in the selected files.

Core's auth/middleware access public request/response fields, not private parent
state. Their local concrete types implement the moved foreign traits legally;
the same is true of local TCP/SMTP server and registry types. The moved inherent
implementations remain with their own types. No orphan-rule workaround, parallel
DTO, Error mapping or new provider abstraction is necessary.

Use `pub use mockforge_protocol::protocol_abstraction::*` inside core's
protocol_abstraction facade, then retain core-local auth/middleware declarations
and reexports. Do not reexport the entire new root there and accidentally nest
fixture/lifecycle namespaces under protocol_abstraction. Preserve core's existing
root and prelude exports and its fixture_store/protocol_server module paths.

The complete TCP/SMTP source references resolve only to the moved abstractions,
fixture store, lifecycle trait or Error/Result. Their integration/E2E tests and
SMTP benchmark do not add a core import. Their manifests have no other internal
normal/build/dev or target-specific dependency. Migrating every `mockforge_core::`
reference and replacing the dependency therefore closes the inspected graph.
CLI converts core config into the server crates' local config structs; it does
not require those crates to import core configuration.

Allow only the three documented helper-example prefix changes: two lifecycle
`no_run` imports and one ignored fixture-loader example. Keep the ignored status;
never add core as a dev dependency to compile documentation. Keep core's effective
`missing_docs = deny` policy for moved APIs, plus strict local all-target Clippy;
workspace-only inheritance would otherwise lower missing_docs to warn.

## Publication and CI audit

Current literal publication order places foundation before core and core before
SMTP/TCP. Inserting mockforge-protocol immediately after foundation satisfies all
new normal edges: foundation < protocol < core/SMTP/TCP. Foundation has no normal
internal MockForge dependency. Its dev-only template-expansion edge is excluded
from publication order and is not activated merely by consuming foundation.

Use inherited workspace version/MSRV/edition/license; do not bump versions or
align unrelated existing 0.3.239 requirements. Add the member and canonical
CRATES entry, preserve publish-list drift and normal/build/optional-edge order
checks, and run their tests without invoking publication or release scripts.

`scripts/ci/rust_scope.py` derives changed packages plus reverse normal/dev/build
consumers. Root manifest/lock and script changes fail closed to the full required
lane. Preserve this behavior. The existing CI all-feature Clippy job and affected
native/runtime suites must select the new crate; checking core alone does not
execute dependency-owned tests. Keep consolidated integration binaries and the
all-protocol CLI executable provenance path intact.

## Required proof before completion

1. Pinned source comparison: seven moved bodies, declared facade removals,
   narrowly listed doc corrections, and TCP/SMTP crate-prefix substitutions.
2. Execute the actual 56 moved cases and any added boundary/behavior cases;
   run strict all-target Clippy, rustdoc and the lifecycle no_run doctest.
3. Prove the selected protocol/TCP/SMTP normal and test graphs exclude core and
   intelligence using actual Cargo edges/artifact records, including cached
   artifacts. A full-workspace graph cannot establish selected-package isolation.
4. Validate new protocol default and scripting/schema modes, TCP/SMTP standalone
   defaults and no-default mode, and core alone under default, no-default,
   scripting/schema modes. Keep core-only no-default validation separate from
   TCP/SMTP selection so additive feature unification cannot hide a regression.
5. In core's existing consolidated integration target, pass new-crate request/
   response types through old core entry points, assign Protocol/Error across
   foundation aliases, and check core AuthMiddleware implements the canonical
   new trait. Retain existing cross-protocol, registry and auth/middleware cases.
6. Execute existing TCP socket roundtrips and SMTP delivery, authentication and
   real STARTTLS upgrade cases; check SMTP's benchmark target. Preserve all
   listener, TLS provider, framing and mailbox implementation bodies.
7. Run existing membership/publication-order guards and required CI. No version
   bump, release or publication is authorized, and no benchmark claim follows
   from source-line counts or changed graph edges.

No further architectural blocker was found once the scripting compatibility graft
is applied. Omitting that graft, keeping a hidden normal/dev edge back to core,
or replacing canonical types with equivalents would each block this migration.
