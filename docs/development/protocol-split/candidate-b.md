# Candidate B: isolate protocol contracts without enlarging foundation

Baseline: `3871677f241ee8e9726ad7646fa356c0a2df04ca`. This is a read-only source
and release-path investigation. No MockForge Cargo command, native test, metadata
query or performance measurement was run.

## Recommendation and caller usage

Prefer a dedicated `mockforge-protocol-core` package for the five-file closure.
Retain foundation as the canonical owner of Error, Result and Protocol. Retain
core's auth, logging and latency adapters. Foundation placement is viable but
less useful for future compile isolation; a matcher-only crate is not an
independent leaf without first moving its request/trait owners.

Existing callers must continue to compile unchanged:

```rust
// mockforge-grpc/src/registry.rs and SMTP/FTP/AMQP spec registries
use mockforge_core::protocol_abstraction::{ProtocolRequest, SpecRegistry};

// mockforge-kafka/src/spec_registry.rs
use mockforge_core::{Protocol, Result};

// Existing custom middleware still implements the same trait through core.
use mockforge_core::protocol_abstraction::{MiddlewareAction, ProtocolMiddleware};
```

New consumers may import these contracts directly from the new package. Existing
core imports remain aliases to the same definitions; never copy DTOs or create
an Error conversion layer. This change does not by itself eliminate normal core
dependencies from current server crates: FTP/SMTP/TCP still use fixture_store
and protocol_server; MQTT and others also use templating. Do not claim those
back-edges are removed before migrating their remaining real uses.

## How the boundary works

ProtocolRequest and ProtocolResponse carry protocol-neutral request/response
data. SpecRegistry and ProtocolFixture describe fixture lookup and validation.
UnifiedFixture performs regex/exact/custom-expression matching and converts
fixtures into responses. BaseSpecRegistry provides indexes and synchronous
JSON/YAML fixture-file loading. ProtocolRegistry dispatches handlers; streaming
types describe messages and asynchronous streams. MiddlewareChain preserves
ordered request processing, early short-circuiting and reversed response order.

The contract crate defines extension points. Core's AuthMiddleware reads core
AuthConfig, and LoggingMiddleware uses core request-log functions. Its latency
adapter uses the core latency authority. Moving those adapters too would
reintroduce dependencies on core or force unrelated ownership changes.

## Exact selected closure

| Existing protocol_abstraction file | Physical lines | Test attributes |
| --- | ---: | ---: |
| mod.rs | 1331 | 17 |
| base_registry.rs | 308 | 11 |
| matcher.rs | 402 | 8 |
| protocol_registry.rs | 337 | 7 |
| streaming.rs | 232 | 3 |
| **Total** | **2610** | **46** |

These are physical source and literal test counts, not execution receipts. No
Tokio-specific reference was found in these five source files; async interfaces
use async-trait/futures. Existing tests use tempfile for real fixture loading.
The implementation is not IO-free: BaseSpecRegistry reads files, and message
builders use the clock. Foundation already brings Tokio and reqwest into the
transitive graph. This is an ownership/test-compilation boundary, not a claim of
a dependency-free graph or a measured speedup.

## Dedicated-package shape

Use a private historical `protocol_abstraction` container inside the new crate,
with the five existing relative paths. Export the contract surface at the root;
root reexports are necessary for matcher tests using `crate::MessagePattern`.
The internal container preserves the streaming test's
`crate::protocol_abstraction::Protocol` path and the existing libtest prefixes.

```rust
// new lib.rs; the implementation files remain in protocol_abstraction/
mod protocol_abstraction;
pub use mockforge_foundation::{Error, Result};
pub use protocol_abstraction::*;
```

The moved mod.rs loses only the auth/middleware declarations and adapter
reexports; its canonical Protocol reexport stays exactly from foundation.
Core's original mod.rs becomes a facade that reexports the new contract owners
and retains `pub mod auth`, `pub mod middleware` and their existing public
reexports. The core root and prelude lists can remain unchanged. Avoid forwarding
methods, duplicate state and broad export cleanup during this move.

Normal new-package dependencies close at foundation (`default-features = false`),
async-trait, chrono, futures, regex, serde, serde_json, serde_yaml and tracing.
Tempfile is a dev dependency. Additional Tokio dev support is appropriate only
if new async behavioral tests actually need it. Keep existing pins/features;
do not enable core defaults or invent a dependency on core, including for tests.

All inspected inherent implementations for the moved types are inside this
closure. Core's auth/middleware and server-crate types remain local self types
implementing the now foreign contract traits, so their implementations remain
legal. Error/Result are already foundation reexports in core/error.rs, and
Protocol is already the foundation enum shared with mockforge-contracts. Keeping
those owners preserves trait return types and cross-crate identity without an
orphan impl workaround.

The selected files have no feature-gated implementations. Core currently
forwards schema and scripting to foundation; retain those existing edges. Do
not route core's default data/AI/advanced/scripting features through the new
contract package. A standalone contract build should not acquire core's default
subsystems. Whether a new schema forwarding convenience is needed is separate
from preserving the existing core feature behavior.

## Structurally distinct alternatives

**Place the five files in foundation.** Add foundation/protocol_abstraction and
make the same core facade point there. Foundation already supplies canonical
Error/Result, so crate::Error paths would work; the selected Protocol reexport
would need one explicit self-crate path correction. Foundation would add futures
normally and tempfile for its fixture tests. This is cycle-free and avoids a new
publishable package. It also adds request/middleware/fixture concerns to a base
already containing 49 source files / 14982 physical lines. All foundation
consumers then inherit rebuild exposure to these protocol changes, and the 46
cases join the larger foundation test artifact. Favor the dedicated package for
this user's smaller-crate objective; do not treat foundation as an unlimited
destination for otherwise closed code.

**Extract matcher.rs alone.** It is 402 lines / eight tests, but depends on
ProtocolRequest and RequestMatcher from its parent module. A normal core
dependency creates a cycle when core reexports the new matcher. Moving those
DTO/trait owners into foundation first avoids the cycle but splits one protocol
knowledge boundary across additional packages and source edits. A registry-only
variant has the same issue through SpecOperation/SpecRegistry/request/response
types. Both are less compelling than moving the complete five-file closure.

**Also move protocol_server.rs.** Its lifecycle trait currently depends only on
Protocol, async-trait and Tokio's shutdown receiver. It is a plausible later
contract extraction, but it adds a Tokio-specific public lifecycle surface and
requires documentation-path changes. It does not by itself remove server core
dependencies while fixture_store/templating remain. Keep this wave's five-file
scope; record that remaining edge instead of promising a fully detached server
layer.

## Source, publishing and validation obligations

1. Qualify the five moved bodies against the pinned source. Allow only the
   declared module/adapter wiring removal and narrowly recorded documentation
   corrections. Preserve regex fallback, metadata hashing, duplicate-fixture
   indexing, enabled-handler behavior, streaming metadata and middleware order.
   A source comparator qualifies this migration; it must not freeze future
   legitimate changes in ongoing native CI.
2. Run the 46 actual library cases and strict all-target Clippy on Rust1.96.
   Preserve the existing package's effective lint policy rather than silently
   introducing broad production allowances. New boundary tests must prove no
   dependency back to core. A meaningful cheap async test can exercise request
   short-circuiting and reverse response order, which the existing creation-only
   MiddlewareChain test does not cover.
3. Put cross-entry-point identity proofs in core's existing consolidated
   integration target (`tests/integration/main.rs`), not a dev back-dependency
   from the contract crate to core or one new binary per assertion. Assign
   core/new-package request types and foundation Error/Protocol across their
   reexports, and require AuthMiddleware to satisfy the new ProtocolMiddleware
   trait. Retain existing auth and protocol-registry integration cases.
4. Add the workspace member and core path dependency. Add the new publishable
   package to canonical `scripts/publish-crates.sh` after foundation and before
   core/any direct consumer. `scripts/check-publish-drift.sh` derives member
   coverage, and check_publish_order.py checks normal/build edges including
   optional dependencies; dev-only fixture cycles do not define publish order.
   The canonical publisher already adds versions to path-only mockforge deps
   before packaging. Do not run publish/release scripts or bump versions here.
5. `scripts/ci/rust_scope.py` derives package/reverse-consumer selection. Root
   Cargo/member/lock/workflow changes correctly fall back to the broader suite;
   preserve that behavior and publication guards. Native CI already runs the
   affected packages' tests; it must include the new package and existing
   consolidated core/HTTP integration suites, not only core's dependency build.
6. Check affected protocol consumers and core defaults/schema/scripting paths
   as warranted. Keep the existing CI all-protocols CLI binary provenance and
   runtime/auth gates intact. A successful package check does not prove live
   middleware/auth behavior or replace required CI.

This design keeps canonical foundational types, moves one connected contract
owner and leaves configuration and runtime authority with the existing core.
The candidate is a source-level recommendation; no native result is claimed.

## Expanded seven-file variant: preferred after the TCP/SMTP audit

The additional evidence changes the scope recommendation above. Add
`fixture_store.rs` (277 physical lines / seven test attributes) and
`protocol_server.rs` (143 lines / three attributes) to the five-file closure.
The selected baseline closure totals 3030 lines / 56 attributes and can remove
two actual normal core edges immediately, rather than only preparing a future
migration. Counts
remain source observations; no MockForge native result is claimed.

Both added files are closed over the selected contracts and foundation aliases.
Fixture loading uses serde/JSON/YAML, std filesystem and tracing. It accepts its
options and directory from the caller; it does not own core configuration,
authorization or request logging. The lifecycle file declares MockProtocolServer
and its Tokio shutdown receiver; its production body contains no socket startup,
configuration decision or concrete server implementation. The three existing
tests include an actual asynchronous shutdown case. Leave concrete TCP/SMTP
listeners, TLS, mailbox storage and framing behavior in their server crates.

### Complete caller and graph audit

TCP's only internal normal dependency is core. Its source references only
Error/Result, fixture_store, protocol_abstraction and protocol_server. SMTP's
only internal normal dependency is also core, and all its core references use
those same four owners. Source imports, fully qualified error constructors and
inline test imports all need the crate-prefix migration. The audited TCP E2E
file, SMTP integration/E2E files and SMTP Criterion bench import their own server
crate and external dependencies, with no further core reference or hidden dev
core dependency. Neither protocol package declares its own feature table or a
build dependency. No additional internal target-specific dependency was found.

After replacing the normal dependency and those prefixes, the only internal
normal path for either package is protocol-core -> foundation. Foundation has no
normal dependency on another MockForge package. Its existing dev-only template
expansion dependency is not a normal edge and is not activated merely because a
protocol package tests against foundation. Thus the proposed normal graph has no
remaining core path. This is a static manifest/source proof to confirm using
actual selected-package Cargo graphs; an all-workspace graph would not prove
isolation.

The CLI converts core configuration into TCP/SMTP's own configuration structs
(`serve.rs`, SMTP around line2957 and TCP around line3231), rather than requiring
those crates to import core config. SMTP already selects its Ring TLS provider
explicitly (`server.rs:178`); no TLS/provider or feature edits are needed to
perform this import/dependency migration. Keep those paths unchanged and execute
the existing real socket, mail delivery and STARTTLS regressions.

### Public namespace and canonical types

For this expanded shape, make the new historical protocol_abstraction module
public so TCP/SMTP can change only the crate prefix of existing imports:

```rust
pub use mockforge_foundation::{Error, Result};
pub mod protocol_abstraction;
pub mod fixture_store;
pub mod protocol_server;
pub use protocol_abstraction::*;
```

Core keeps its three public modules. Its protocol_abstraction facade reexports
`mockforge_protocol_core::protocol_abstraction::*` and retains core-local auth
and middleware modules/reexports. Fixture_store and protocol_server become
reexport facades for their new owners. Core's existing root/prelude exports stay
in place. CLI's old core MockProtocolServer alias and TCP/SMTP's new direct trait
are then one definition; foundation Error/Result/Protocol also remain one owner.
No duplicate trait, enum, DTO or conversion implementation is needed.

### Preserve the inherited scripting capability explicitly

There is one concrete feature compatibility trap. TCP/SMTP currently depend on
core with defaults, and core defaults enable foundation/scripting. That feature
adds the public Error::JavaScript variant and its From<rquickjs::Error>
implementation (`foundation/src/error.rs:190`). Merely replacing the dependency
with a default-empty protocol package would remove that incidental public error
capability in isolated builds, even though TCP/SMTP do not use it internally.

Use explicit forwarding rather than silently narrowing the contract:

```toml
# new protocol package
[features]
default = []
scripting = ["mockforge-foundation/scripting"]
schema = ["mockforge-foundation/schema"]

# both TCP and SMTP normal dependencies
mockforge-protocol-core = { path = "../mockforge-protocol-core", features = ["scripting"] }

# core's new dependency; existing core feature forwarders stay unchanged
mockforge-protocol-core = { path = "../mockforge-protocol-core", default-features = false }
```

The unconditional TCP/SMTP dependency feature preserves their old behavior for
both package defaults and --no-default-features: neither previously disabled
core's dependency defaults. The new package can remain default-empty for other
consumers, and core's existing no-default mode does not acquire scripting through
this new edge. Preserve the existing core/schema and core/scripting forwarders.
Validate those modes in separate selected-package invocations, because Cargo's
additive feature unification with TCP/SMTP would mask a core no-default mistake.
Scripting compatibility retains QuickJS transitively for TCP/SMTP; do not promise
those isolated graphs lose every heavy dependency.

### Qualification and publication scope

The new package now needs Tokio normally for MockProtocolServer, plus the prior
normal dependencies; preserve the workspace pin/features. Seven source files
move, with only the declared facade changes and documentation crate-name rewrites
(the lifecycle no_run example currently imports core). TCP/SMTP source changes
should be crate-prefix substitutions only. Keep fixture parse/error policy,
framing, mailbox behavior, TLS and server configuration body-identical.

Update the publish order to foundation < protocol-core < core/TCP/SMTP, preserving
all other dependency constraints. Add the workspace member and replace both
normal core dependencies; do not leave an optional core dependency as a hidden
compatibility escape. Retain existing publish drift/order guards and affected
reverse-consumer CI. No publication or version bump is part of this task.

I recommend this expanded variant over the earlier five-file-only wave. No
architecture blocker was found after explicit scripting preservation. Native56,
strict Clippy, selected normal/feature graph proofs, core facade identity and
integration checks, plus the existing TCP/SMTP E2E and STARTTLS cases remain
required execution evidence before claiming the migration complete.
