# Candidate A: a dedicated protocol-contract crate with a core adapter facade

Baseline: `3871677f241ee8e9726ad7646fa356c0a2df04ca`. Source-only design; no
Cargo command, build, native execution or benchmark was run for this candidate.
The architectural comparison rules and local agent/release instructions were read.

## Problem

Core owns protocol-neutral request/response contracts and generic registries,
then hosts configuration-bound auth and process-global logging adapters beside
them. Those have different dependency and authority requirements. Extract the
existing contract implementation and tests into `mockforge-protocol`, with no
edge back to core; retain concrete adapters and public compatibility imports in
core. The immediate benefit is a separately compiled/tested 46-case contract
suite. It does not by itself remove every protocol server's core dependency.

## Usage first

New contract-only code imports the new crate directly:

```rust
use mockforge_protocol::{
    BaseSpecRegistry, MessagePattern, Protocol, ProtocolFixture,
    ProtocolRequest, ProtocolResponse, SpecRegistry,
};
let request = ProtocolRequest { protocol: Protocol::Mqtt, ..Default::default() };
```

Existing consumers retain their imports, including adapter users:

```rust
// Existing server registry implementations keep their canonical trait/type.
use mockforge_core::protocol_abstraction::{ProtocolRequest, SpecRegistry};
// Auth/configuration and logging authority stay in core.
use mockforge_core::protocol_abstraction::{AuthMiddleware, LoggingMiddleware};
// Existing root/prelude exports keep resolving to the same moved definitions.
use mockforge_core::{MessagePattern, ProtocolRequest as OldRequest};
```

Add a compatibility test passing a new-crate request into a function accepting
`mockforge_core::ProtocolRequest` without conversion. Compare foundation Protocol
and Error type identity across both entry points. Do not introduce parallel
request/error enums or map equivalent-looking DTOs at this boundary.

## Shape and source map

Normal edge: `mockforge-core -> mockforge-protocol -> mockforge-foundation`.
The new crate owns the protocol vocabulary, middleware SPI/chain, fixture matching,
file-backed generic registry and streaming handler interfaces. Foundation remains
the sole owner of Error, Result and Protocol. This separates knowledge from core
adapters without turning routing or authentication into pass-through wrappers.

Move these five old `src/protocol_abstraction` files with their relative layout
under a private `protocol_abstraction` container in the new crate:

| File | Physical lines | Literal test attributes |
| --- | ---: | ---: |
| `mod.rs` | 1,331 | 17 |
| `base_registry.rs` | 308 | 11 |
| `matcher.rs` | 402 | 8 |
| `protocol_registry.rs` | 337 | 7 |
| `streaming.rs` | 232 | 3 |
| **Total** | **2,610** | **46** |

The source container's `mod.rs` drops only `pub mod auth`, `pub mod middleware`
and their public reexport groups. Preserve its actual type/function bodies and
all four sibling files. New crate root:

```rust
//! Protocol contracts, generic registries, matching and streaming support.
mod protocol_abstraction;
pub use mockforge_foundation::{Error, Result};
pub use protocol_abstraction::*;
```

Replace core's old module implementation with a facade that retains the adapters:

```rust
pub use mockforge_protocol::*;
pub mod auth;
pub mod middleware;
pub use auth::{AuthMiddleware, AuthResult, Claims};
pub use middleware::{LatencyMiddleware, LoggingMiddleware, MetricsMiddleware};
```

Core's existing `lib.rs` root/prelude reexports remain unchanged. Preserve the
public submodule paths, not merely the root names: `base_registry`, `matcher`,
`protocol_registry` and `streaming` must remain reachable through the facade.

### Actual dependency and test-only references

- All five files use only `crate::Result`/`crate::Error` from core. New root
  reexports the exact foundation definitions already reexported by core.
- `mod.rs` already reexports `mockforge_foundation::protocol::Protocol`; retain it.
- Base registry uses serde/JSON/YAML and filesystem fixture loading. Keep that
  behavior; do not claim this is an IO-free data-only package.
- Matcher test request construction refers to `crate::MessagePattern`. The new
  root's public type reexports preserve it without test body edits.
- Protocol-registry tests refer to `super::super::ResponseStatus`. Retaining the
  container/sibling layout preserves this parent lookup.
- Streaming's test refers to `crate::protocol_abstraction::Protocol::Mqtt`.
  The private container preserves that crate-internal path.
- No `pub(crate)`, `pub(super)`, `include_str!`, `include_bytes!` or `#[path]`
  coupling was found in these five files. Their inline tests move with them.
- Auth's `super::{ProtocolMiddleware, ProtocolRequest, ...}` lookups resolve
  through the core facade. The concrete auth and three middleware types remain
  local to core, so implementing the moved foreign traits remains legal. External
  protocol registries already implement foreign traits on local types; that also
  remains legal. No shared-state/global-emitter owner changes.

Do not confuse protocol matching's public `RequestFingerprint` with core's other
private `request_fingerprint` family used by replay/priority handling. They are
already distinct types; this extraction must not consolidate them.

Normal new-crate dependencies are foundation, serde, serde_json, serde_yaml,
regex, async-trait, futures and chrono. The five-file closure has no direct
`tokio::` use: all 46 literal cases are synchronous `#[test]`, and asynchronous
registry cases use `futures::executor::block_on`. Do not add a direct Tokio
dependency merely because core has one. Use existing workspace pins/features;
`tempfile` is needed for the moved fixture loader tests. Foundation itself has runtime/HTTP dependencies, so package isolation
means absence of core/intelligence back-edges, not a dependency-free graph.

### Feature behavior

The five contract modules are currently unconditionally compiled. They do not
use core's `ai`, `advanced`, `contracts`, `workspace-mgmt`, `scripting`, `schema`
or `database` feature gates. Do not add a new default-enabled optional capability
or silently drop existing core defaults. Retain current foundation feature
forwarding from core (`scripting`, `schema`) exactly; the new contract dependency
can remain foundation-default-off as it has no need for those optional features.
Run core default, no-default and all-feature checks/tests required by existing
policy. Any future schema derives for the new crate are separate scope.

## Real consumers and the limit of this first wave

Direct contract users include registry/server implementations in MQTT, Kafka,
AMQP, SMTP, FTP, TCP, gRPC and GraphQL, plus CLI governance mapping and core's own
cross-protocol integration tests. All can use the new contracts through current
core imports immediately; local library users can directly depend on the new
crate without pulling core's intelligence/configuration graph.

However, **none of those inspected whole server crates can simply delete its core
dependency after only this five-file move**:

- TCP/SMTP use core fixture loading, Error/Result and `MockProtocolServer`.
- FTP also uses `config::FtpConfig` and command fixture helpers.
- AMQP/Kafka use core configuration and other server helpers.
- gRPC/GraphQL retain OpenAPI, template or listener helpers; WebSocket also has
  core lifecycle integration.

Do not report a removed existing Cargo back-edge or a measured build speedup
until actual manifest migrations demonstrate it. Follow-up contract consumers
should migrate imports only when their complete remaining dependency closure can
also move; merely adding another direct dependency does not reduce compilation.

A small coherent graft is `protocol_server.rs` (143 physical lines, three literal
tests), whose `MockProtocolServer` trait depends only on canonical Protocol,
async-trait and Tokio's shutdown receiver. Moving it with a core reexport would
complete lifecycle vocabulary, but still does not solve fixture/config coupling.
It is optional synthesis scope, not included in the 2,610/46 estimate above.
Another follow-up is fixture_store (277 lines), but expanding this wave into
configuration/template extraction would obscure the independently verifiable unit.

## Alternatives considered

1. Put all five modules in existing foundation. Canonical types/errors remain
   correct and no new publishable package is required. It loses the narrower
   independently selected test/interface boundary and expands a widely depended-on
   general foundation crate with file loading, registry and middleware knowledge.
   The dedicated package has a more explicit owner and smaller future change
   blast radius while keeping foundation's existing canonical primitives.
2. Move only matching and the generic registry. These need the shared request,
   protocol, status and trait vocabulary. Leaving that vocabulary in core retains
   the back-edge; copying it creates distinct canonical types. Moving it to
   foundation first converges on a two-stage version of the same complete boundary.
3. Move auth/logging adapters too. Auth imports core AuthConfig and API-key types;
   logging uses global request-log helpers, latency uses core's injector bridge.
   Moving them pulls configuration/global state or introduces new provider abstractions
   unrelated to this source-preserving compilation change. Keep them in core.

The facade is a necessary compatibility boundary, not a second implementation.
No new actor owns privileged state; no runtime validation/policy decision is moved.
One crate owns each type/trait, and one foundation owns each shared error/protocol.
Avoid new wrapper request types, conversion maps or generic plugin architectures.

## CI and publishing ownership

- Add the member and an explicit path dependency from core. Inherit the actual
  workspace package version/MSRV/edition/license metadata; do not hardcode a new
  release version or bump anything. Current baseline workspace version is 0.3.240;
  some existing intercrate minimums are 0.3.239. Do not "align" unrelated versions
  or perform publishing-script mutation during source migration.
- Add `mockforge-protocol` to `scripts/publish-crates.sh`'s literal `CRATES` list
  after foundation and before core (and every migrated consumer). The pre-push
  drift guard derives actual publishable workspace members, then calls
  `check_publish_order.py`, which checks normal/build edges including optional
  dependencies. Preserve those tests and the version/caret consistency checks.
  A path-only dependency is allowed locally under the existing publisher's
  temporary version-injection process. No publication or release is authorized.
- CI derives the changed package plus all reverse normal/dev/build dependents
  using `scripts/ci/rust_scope.py`. Root manifest/lock/script edits fail closed to
  the full required lane. Keep the new crate in that graph; do not add a skip.
- Existing CI Clippy checks libs/bins with all features and correctness/qualification
  denial. Local validation must additionally use the documented strict
  `scripts/dev/cargo clippy -p mockforge-protocol --all-targets -- -D warnings`.
  Avoid blanket production lint allowances to conceal newly surfaced issues.
- Execute all moved native tests and any new dependency/type-identity guard tests.
  A raw source count, no-run or empty filter is not verification. Compile against
  the real artifact, and run reverse consumers as selected by their actual closure.
- Preserve the consolidated `mockforge-core/tests/integration/main.rs` binary.
  Its `cross_protocol_tests` and `protocol_registry_integration` modules are key
  adapter/type-identity coverage. Do not restore one binary per module. Preserve
  HTTP and registry-server consolidation and the required all-protocols CLI binary
  runtime path (`MOCKFORGE_TEST_BINARY`) for real wire tests.
- No UI/template behavior changes are proposed, so no UI feature claims follow
  from library tests. Run applicable runtime protocol behavior, publishing-list
  tests, formatting, affected features and required CI before declaring completion.

## Synthesis and first implementation step

This is an independent candidate, not the synthesis verdict. If selected, register
one new package, move this exact source closure behind the existing facade, and
prove the new native test count and absence of core back-dependencies before any
broader consumer migration. Consider the lifecycle-trait graft separately and
account for its three existing tests if selected.

## Expanded seven-file variant: prefer this graft

Independent follow-up tracing confirms a better coherent unit than the five-file
baseline: add `fixture_store.rs` (277 lines/seven tests) and `protocol_server.rs`
(143 lines/three tests: two synchronous, one Tokio). The resulting seven source
files have **3,030 physical lines and 56 existing literal cases**. This modest
expansion actually allows TCP and SMTP to remove their normal core dependency,
so prefer it over the five-file-only recommendation above.

The complete TCP and SMTP trees were scanned, including source, integration tests,
SMTP benchmarks and both manifests. Every core-qualified reference is one of:
protocol_abstraction contracts, fixture_store, protocol_server, Error or Result.
Neither manifest has another MockForge normal/build/dev dependency. Their own
config types are local; unlike FTP they do not import core config. Their existing
wire tests and SMTP benchmark do not import core. Thus replacing the one manifest
edge and all `mockforge_core::` namespace references is a closed consumer migration,
not a guessed "the top-level import looks small" simplification.

Use this public package shape instead of the first sketch's private container:

```rust
pub mod fixture_store;
pub mod protocol_abstraction;
pub mod protocol_server;
pub use mockforge_foundation::{Error, Result};
pub use protocol_abstraction::*;
```

The five protocol files still retain their relative layout and test references.
The new public module preserves transport prefixes, including the streaming
unit test's `crate::protocol_abstraction::Protocol` path. Core's adapter facade
reexports **`mockforge_protocol::protocol_abstraction::*`**, then keeps auth and
middleware declarations/reexports local. That precise inner reexport avoids
nesting the new crate's whole module tree into its own compatibility module.
Core's fixture/lifecycle module declarations become aliases or thin reexport
modules of the new package, preserving every existing core import path.

Fixture loading uses only canonical Error/Result, serde/JSON/YAML, path/filesystem
and tracing. The lifecycle trait uses canonical Protocol, async-trait and Tokio's
watch receiver. Neither contains core config, global logging, auth, templating,
policy, daemon/store state or a callback into core. Add tracing and a direct
Tokio dependency for this expanded variant; the five-file variant alone did not
need direct Tokio. Preserve both helpers' bodies and behavior.

Explicit documentation-only exceptions are required: the lifecycle `no_run`
example imports core twice, and the fixture loader's ignored example imports
core. Change those example crate names to the new package; do not add core as a
dev-dependency to make docs compile. Keep the fixture example ignored as before.
Neither helper uses `CARGO_MANIFEST_DIR`, include macros, test path attributes or
external asset fixtures. Their inline tests create their own temporary fixtures.

There is no normal/build cycle: foundation has no internal MockForge dependency,
new protocol depends on foundation, TCP/SMTP depend on new protocol, and core
also depends on new protocol. Core auth/middleware implementations and concrete
TCP/SMTP server types stay local to their implementing crates, so their moved
trait implementations satisfy orphan rules. Existing foundation Error and
Protocol are aliases throughout; no enum conversion or copied type is needed.
Foundation's dev-only template-expansion edge is not activated merely by consuming
foundation as a dependency. Keep compatibility/type-identity tests in core's
existing consolidated integration binary, never in the new package's dev graph.

The publish order must be foundation → protocol → core/TCP/SMTP; inserting the
new crate after foundation and before core in the current CRATES array satisfies
all four edges. Core's existing feature forwarding stays unchanged. TCP/SMTP
focused builds intentionally cease activating unrelated core default features;
new protocol need not opt into foundation scripting merely to reproduce an unused
transitive feature. Canonical type identity still unifies when a caller also
selects core. Validate actual consumer/default/all-feature behavior without making
an unmeasured timing claim.

Qualification should now assert an actual core-free compilation graph for selected
new-protocol/TCP/SMTP tests (including dev/build edges and cached artifact records),
execute all 56 moved cases, retain existing TCP roundtrip and SMTP delivery/STARTTLS
behavior tests, compile the SMTP benchmark, and run strict all-target Clippy on
all three packages. Then validate core's facade/default/feature variants and
consolidated protocol integration modules. No new package may acquire a normal,
build or dev edge back to core/intelligence to satisfy a test or documentation
example; such a back-edge would undo the point of the expanded extraction.
