# Temporary AWS SigV4 security backport

This directory contains the official crates.io `aws-sigv4` 1.6.3 archive.
`upstream-provenance.json` records its verified SHA-256 and upstream patch revision.
The Apache-2.0 license and original copyright notices remain intact.

The production Rust source is unchanged. The manifests raise `crypto-bigint`
from 0.5.4 to 0.7.6, following smithy-lang/smithy-rs#4958, to remove the
RUSTSEC-2026-0387 affected dependency closure without removing SigV4a/MRAP.
The two regression tests are copied from that exact upstream revision and
registered explicitly because the published archive disables automatic tests.
The standalone lockfile is regenerated for this dependency change.

The root workspace uses a path patch; this is not a claim that an official
1.6.3 release contains the fix. Remove the patch and vendor directory when an
official compatible release includes the fix, then rerun the existing security
checks and signer/MRAP regression tests. No advisory ignore is added.

Local validation: 131 upstream unit tests passed (10 upstream ignored), one
closure test and two MRAP regressions passed; the real SDK consumer passed.
See `scripts/security/sigv4a-sdk-tests/README.md` for constraints and the local
mold linker failure versus the successful per-command lld retry.
