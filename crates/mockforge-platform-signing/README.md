# mockforge-platform-signing

HSM-backed platform signing-root for MockForge — implements RFC §8.2 and §9
of [`cloud-trust-permissions-rfc.md`](../../docs/plugins/security/cloud-trust-permissions-rfc.md).

The **platform signing root** is the key that authenticates first-party
MockForge cloud plugins. The private bytes must never reach
general-purpose compute; this crate keeps them behind an HSM boundary
(AWS KMS for v1) and only exposes a `sign(...)` operation that round-trips
through the KMS service.

See [`docs/plugins/security/platform-signing-rotation-runbook.md`](../../docs/plugins/security/platform-signing-rotation-runbook.md)
for the operator-facing rotation procedure.

## Why a separate crate

- The registry server needs the **signer** (to publish rotation events
  and sign the plugin-blocklist).
- Plugin-hosts (`mockforge-plugin-host`) need the **verifier** (to
  trust newly-rotated platform keys).
- Pulling the AWS SDK into either crate directly would bloat the
  self-hosted build. This crate is feature-gated (`aws-kms`) so the
  trait + verifier are always available; the AWS backend is opt-in.

## Backend choice (v1 = AWS KMS)

| Backend | FIPS level | Trade-off |
| ------- | ---------- | --------- |
| AWS KMS standard CMK | FIPS 140-2 L2 | Cheapest, fastest, no extra ops |
| AWS KMS Custom Key Store (CloudHSM-backed) | FIPS 140-2 L3 | Documented upgrade path; same SDK call surface |
| GCP KMS | FIPS 140-2 L3 | Future backend (trait-based) |
| YubiHSM (on-prem) | FIPS 140-2 L3 | Future backend; for air-gapped installs |

We picked AWS KMS for the first cut because AWS credentials are already
present in the registry environment (via the `storage-s3` feature). The
upgrade path to CloudHSM Custom Key Store does not change the trait
surface — only the `KeyId` ARN changes.

### Why ECDSA P-256 (not Ed25519)

AWS KMS does not support Ed25519 signing keys as of writing. The platform
signing root therefore uses `ECC_NIST_P256` with `ECDSA_SHA_256`. This is
distinct from the per-publisher Ed25519 keys (see
`mockforge-plugin-host::signing`) — those continue to live in the
existing trust-store. The platform signer authenticates the **rotation
event** that introduces a new publisher trust-root; it never replaces
the per-publisher signature on the WASM bytes.

## Rotation model (dual-control)

1. Operator generates a new KMS CMK out-of-band (see runbook).
2. New CMK's public bytes are fetched via `GetPublicKey`.
3. The **current** CMK signs the new CMK's public bytes via `Sign` —
   this is the cryptographic handover. The signature proves the new
   key was authorized by the predecessor.
4. The registry publishes a `RotationEvent { from_key_id, to_key_id,
   to_public_key_der, handover_signature_der, transition_until }` —
   plugin-hosts trust **both** keys during the transition window
   (default 30 days, per RFC §9).
5. After the transition window expires, the old CMK is disabled in
   AWS via `DisableKey` (and eventually scheduled for deletion).

Every step emits an `audit_logs` row tagged with the operator user-id.

## Out of scope (filed as follow-ups)

- **Plugin-host live reload** of the rotation event — depends on the
  in-flight #549 trust-cache PR. Tracked separately.
- **CloudHSM Custom Key Store** provisioning automation — runbook
  covers the manual upgrade path.
- **Two-of-three quorum** signing (RFC §9 long-term target). AWS KMS
  alone is single-operator; quorum requires either CloudHSM with
  multi-officer PIN, or a custodial multisig overlay. Out of scope
  for v1.
