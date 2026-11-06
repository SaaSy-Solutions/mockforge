# Protocol extraction verification

Issue #1477 tracks this extraction. The source baseline is
`3871677f241ee8e9726ad7646fa356c0a2df04ca`.

The new `mockforge-protocol` crate owns seven existing files, totaling 3,030
physical Rust lines before the four adapter declarations are removed. Core keeps
its authentication, logging, metrics, and latency adapters. Its historical module,
root, and prelude imports refer to the same types through reexports.

TCP and SMTP now depend directly on the new owner. Their normal, build, and test
crate graphs exclude `mockforge-core`, `mockforge-intelligence`, and
`mockforge-openapi`. Both transport dependencies explicitly enable `scripting`,
including when their own default features are disabled. Foundation remains the
canonical owner of `Error`, `Result`, and `Protocol`. Core retains its original
feature forwarding, and its new protocol dependency has default features disabled.

## Source preservation

`python3 scripts/dev/verify_protocol_extraction.py` compares this extraction
against the pinned Git baseline. This is a migration proof, not an ongoing source
freeze. It checks all seven moved files, all fourteen transport Rust files, the
three compatibility facades, Core's retained adapters and exports, and the exact
manifest feature changes. The only source exceptions are four adapter wiring
lines, three documentation import substitutions, transport namespace changes,
and one induced rustfmt wrap in TCP's base64 error mapping.

Seven isolated Python tests demonstrate rejection of behavior changes, missing
facades, feature drift, transport inventory changes, and aliased dependencies in
target-specific normal, build, and development tables.

## Validation record

The local checkout uses Rust and Cargo 1.96.0 and its own `target` directory.
Compilation runs through `scripts/dev/cargo` with the existing host policy.
There are no target, resource, profile, or compiler flag overrides.

Completed checks:

- Protocol's 56 native tests pass. Its lifecycle documentation example compiles.
  The existing ignored fixture documentation example remains ignored.
- TCP's 77 library tests and four socket roundtrip tests pass with no ignored or
  filtered cases. Its 207 emitted Cargo artifact records exclude the three
  forbidden packages and confirm Foundation's `scripting` feature is enabled.
- Source identity proof, seven mutation tests, formatting, and diff checks pass.
- The publication list includes the new crate after Foundation and before Core
  and both transports. Publish drift, two ordering tests, five publisher status
  tests, and fifteen CI scope tests pass. No crate is published or version bumped.

SMTP's 63 library tests, 18 integration tests, and six delivery tests pass with
zero ignored or filtered cases. Delivery tests include AUTH PLAIN, AUTH LOGIN,
and a real STARTTLS socket upgrade. Its 235 emitted artifact records exclude the
three forbidden packages and preserve Foundation's `scripting` feature. The SMTP
benchmark check and strict all-target Clippy for protocol, TCP, and SMTP pass.

All 56 protocol native cases also pass with all features enabled. Both transports
repeat their complete native suites successfully with default features disabled,
plus one compiling documentation example each. Their actual selected artifacts
retain scripting without Core or schema feature unification. Protocol rustdoc
with all features passes without warnings.

Core's separate no-default library check and strict all-target Clippy pass on
HAL. Its full default package run passes with 1,180 library, 10 configuration,
483 consolidated integration and six scenario tests. There are zero failures and
two existing ignored native cases. Core's documentation reports 13 passes and
seven existing ignores. The five new compatibility cases include protected-route
authentication denial through the moved trait. Existing cross-protocol and
registry cases also pass.

HAL uses the identical frozen tracked patch SHA-256
`767a5a69b895727c772b54672d10cae92f2e2ae8550d4d314f3f38b4dac091fe` and untracked
overlay SHA-256
`9ec0d6e4735ff1cb34c18decb5f47590b125be1a5cec4f42f82038069f66a573`.
The first HAL attempt stops before project compilation because its inherited
Cargo configuration selects `lld` directly and cannot resolve system C libraries.
Successful validation selects `/usr/bin/cc` only through the command-local
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER` environment variable. Global
configuration, compiler flags, targets, profiles and resource limits are unchanged.
This environment correction is not performance evidence.

The normal local commit hook's workspace-wide, all-feature, all-target Clippy
check passes. That broader check is required by the root manifest and lockfile
changes. Cargo audit passes with its existing advisory policy and three allowed
warnings. The first hook attempt also identifies a missing executable bit on the
proof script and unavailable local `typos` and `cargo-audit` tools. The mode is
fixed, and official tool releases run from a task-owned temporary directory.
No hook, lint, advisory, test filter or resource policy is weakened.

No measured whole-build speedup is claimed. The all-protocol CLI still depends
on Core, and both transports retain QuickJS through their existing error feature.
