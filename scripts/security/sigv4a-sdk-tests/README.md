# Offline production S3 signer regression

This isolated crate retains Mockforge's production S3 SDK version and feature
configuration. Its test invokes the real SDK's MRAP presigner with static public
test credentials. It asserts ECDSA signing, a regionless scope, the region set and
the session token. It makes no AWS requests.

The existing Security Audit job runs this test and the vendored upstream signing
suite with locked dependencies. The vendor's separate lockfile protects the
upstream unit and MRAP regression tests; this fixture locks the SDK consumer.
The application workspace lockfile continues to govern production consumers.

```bash
cargo test --locked --manifest-path vendor/aws-sigv4/Cargo.toml --all-features --lib --test crypto_dependencies --test sigv4a_mrap
cargo test --locked --manifest-path scripts/security/sigv4a-sdk-tests/Cargo.toml
python3 scripts/precommit/cargo_audit.py
cargo deny check licenses sources bans
```

## Local validation, 2026-10-24

Rust 1.96.0; an isolated native-ext4 target; existing shared heavy admission;
12 GiB memory, no swap, 200% CPU quota and two Cargo jobs:

- RustSec scan through the unchanged wrapper: zero vulnerabilities; existing
  warning policy retained. License/source/bans checks passed.
- Official signer: 131 unit tests passed, 10 upstream tests ignored unchanged.
- Patched dependency closure: one regression passed.
- Offline MRAP headers and presigning: two regressions passed, including
  independent published-key verification and rejection of tampered input.
- Actual S3 SDK consumer: one MRAP presigning regression passed.
- Runner routing: 18 tests passed; owned security tools: 20 tests passed.

The default local mold invocation failed to link a proptest object with an
`.eh_frame` relocation error before running tests. The successful retry used
only the bundled Rust toolchain's lld via per-command `RUSTFLAGS`, a separate
target and unchanged resource limits. Global linker settings were untouched.
The combined successful signer/SDK invocation took 126.62 seconds with 0.00022
seconds admission wait. These are validation durations, not speedup claims.
CI must still pass on its normal configured toolchain and linker.

Remove this temporary fixture and the vendor patch together after an official
compatible AWS signer release incorporates the security fix and equivalent
regressions are preserved upstream. See the vendor's `PROVENANCE.md`.
