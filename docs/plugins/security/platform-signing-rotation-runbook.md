# Platform signing-root rotation — operator runbook

Implements RFC §8.2 and §9 of [`cloud-trust-permissions-rfc.md`](./cloud-trust-permissions-rfc.md).

The **platform signing root** is the AWS KMS-backed key that authenticates
first-party MockForge cloud plugins and signs the kill-switch blocklist.
This runbook covers:

1. [One-time setup](#one-time-setup) — provisioning the very first key.
2. [Normal rotation](#normal-rotation) — periodic, scheduled handover.
3. [Emergency revocation](#emergency-revocation) — active key is
   believed compromised.
4. [Post-rotation verification](#post-rotation-verification).
5. [Upgrade path: CloudHSM Custom Key Store](#upgrade-path-cloudhsm-custom-key-store)
   — FIPS 140-2 L3.

**Audience:** SaaSy Solutions platform operator (RFC §1 "Operator"
persona). Org admins do NOT perform this — their trust-roots are
managed via `/api/v1/organizations/{org_id}/trust-roots`, see
[`trust_roots.rs`](../../../crates/mockforge-registry-server/src/handlers/trust_roots.rs).

**Pre-reqs:**

- AWS account with IAM permissions to create and manage KMS CMKs.
- The registry server's IAM role has `kms:Sign`, `kms:GetPublicKey`,
  `kms:DescribeKey` on the active CMK. (NOT `kms:DisableKey` — that
  step is performed by an operator out-of-band; see [security note](#why-the-registry-role-cannot-disable-keys).)
- `aws` CLI v2 logged in to the right account/region.

---

## One-time setup

Only run this when provisioning a brand-new MockForge cloud deployment
that has never had a platform signing root.

### 1. Create the KMS CMK

```bash
aws kms create-key \
  --description "MockForge platform signing root (initial)" \
  --key-usage SIGN_VERIFY \
  --customer-master-key-spec ECC_NIST_P256 \
  --origin AWS_KMS \
  --tags TagKey=mockforge:component,TagValue=platform-signing-root \
         TagKey=mockforge:version,TagValue=1
```

> **Why P-256?** AWS KMS does not support Ed25519. P-256 has the
> smallest signatures and the fastest verify on the host fleet. For
> higher-assurance deployments, use P-384 (`ECC_NIST_P384`); the host
> verifier accepts both.

The output includes a `KeyId` (a UUID) and an `Arn`. Capture the ARN.

### 2. Create a stable alias

KMS key UUIDs are stable but cryptic. Create a friendly alias so the
rotation procedure can refer to it:

```bash
aws kms create-alias \
  --alias-name alias/mockforge-platform-signing-active \
  --target-key-id <arn-from-step-1>
```

### 3. Grant the registry role minimal permissions

Attach to the registry server's IAM role:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["kms:Sign", "kms:GetPublicKey", "kms:DescribeKey"],
      "Resource": "<arn-from-step-1>"
    }
  ]
}
```

Do **not** grant `kms:DisableKey`, `kms:ScheduleKeyDeletion`, or
`kms:DeleteAlias` — see [security note](#why-the-registry-role-cannot-disable-keys).

### 4. Configure the registry server

Set the env var in the registry deployment (Fly.io secret, k8s
Secret, etc.):

```bash
MOCKFORGE_PLATFORM_SIGNING_KMS_KEY_ID=arn:aws:kms:us-east-1:<acct>:key/<uuid>
```

Restart the registry. On boot it will call `kms:GetPublicKey` once to
discover the key spec; check the logs for:

```
AwsKmsSigner ready key_id=<arn> algorithm=EcdsaSha256P256
```

If the key is unreachable, the registry will refuse to start. This is
deliberate — the cloud plugin trust chain is load-bearing.

### 5. Publish the initial public key to plugin-host releases

The first plugin-host build needs to embed this public key as a
**trusted root**. Until rotation #1 fires, every plugin-host accepts
only this key. Bundle the public key (PEM) into the plugin-host
release tarball; the build system pulls it from S3.

```bash
aws kms get-public-key \
  --key-id alias/mockforge-platform-signing-active \
  --output text \
  --query PublicKey | base64 -d > platform-signing-root-v1.spki.der
openssl ec -pubin -inform DER -in platform-signing-root-v1.spki.der -out platform-signing-root-v1.pem
# Commit `platform-signing-root-v1.pem` to the plugin-host release pipeline.
```

---

## Normal rotation

Run this on a regular schedule (default: every 6 months; max: every
12 months to stay ahead of cryptographic best practice).

The rotation is **dual-control**: the current key signs the new key's
public bytes, proving authorization. Plugin-hosts that already trust
the current key can validate the rotation event and start trusting
the new key without any out-of-band coordination.

### 1. Generate the next CMK

```bash
aws kms create-key \
  --description "MockForge platform signing root (rotation $(date +%Y-%m-%d))" \
  --key-usage SIGN_VERIFY \
  --customer-master-key-spec ECC_NIST_P256 \
  --origin AWS_KMS \
  --tags TagKey=mockforge:component,TagValue=platform-signing-root \
         TagKey=mockforge:version,TagValue=$((CURRENT_VERSION + 1))
```

Capture the new ARN as `NEW_ARN`.

### 2. Grant the registry role permissions on the new key

```json
{
  "Effect": "Allow",
  "Action": ["kms:Sign", "kms:GetPublicKey", "kms:DescribeKey"],
  "Resource": "<NEW_ARN>"
}
```

### 3. Drive the handover

Call the registry's internal rotation endpoint (auth: platform-operator
JWT, scope: `platform.signing.rotate`):

```bash
curl -X POST https://registry.mockforge.dev/api/internal/platform-signing/begin-handover \
  -H "Authorization: Bearer $OPERATOR_JWT" \
  -H "Content-Type: application/json" \
  -d "{
    \"to_key_id\": \"$NEW_ARN\",
    \"transition_window_days\": 30
  }"
```

For air-gapped deployments (or for the very first rotation in a
brand-new cluster, before the operator JWT is provisioned), the same
audit-aware [`begin_handover`](../../../crates/mockforge-registry-server/src/platform_signing.rs)
call is also exposed as a one-shot binary that runs directly against
the registry's database:

```bash
cargo run -p mockforge-registry-server --bin rotate-platform-key -- \
  --to-key-id "$NEW_ARN" \
  --transition-window-days 30 \
  --operator-org-id  "$OPERATOR_ORG_UUID" \
  --operator-user-id "$OPERATOR_USER_UUID"
```

Both paths write the same `platform_signing_rotation_started` audit
row; auditors can't tell them apart from the row contents (good — the
audit story is identical either way).

The response is a JSON `RotationEvent`:

```json
{
  "payload": {
    "version": 1,
    "from_algorithm": "ecdsa-sha256-p256",
    "from_key_id": "<OLD_ARN>",
    "from_public_key_b64": "...",
    "to_algorithm": "ecdsa-sha256-p256",
    "to_key_id": "<NEW_ARN>",
    "to_public_key_b64": "...",
    "issued_at": "2026-05-18T12:00:00Z",
    "transition_until": "2026-06-17T12:00:00Z"
  },
  "handover_signature_b64": "..."
}
```

The registry:

1. Calls `kms:Sign` on the OLD key over the canonical payload bytes.
2. Writes a `platform_signing_rotation_started` row to `audit_logs`
   (op id, ip, timestamp, both ARNs, both pub keys).
3. Exposes the event on `/api/internal/plugin-rotation-events` for
   plugin-hosts to poll.

### 4. Watch the fleet pick up the new key

Plugin-hosts poll
`GET /api/internal/plugin-rotation-events` on the same 60-second
cadence they use for the kill-switch (RFC §8.2), driven by the
[`MOCKFORGE_PLUGIN_HOST_ROTATION_URL`](../../../crates/mockforge-plugin-host/src/main.rs)
env var. Within 60s of step 3, every host should be in the
"two trusted keys" state.

The host's trust set surfaces through its IPC Health response (which
the parent `mockforge` process is responsible for re-exposing on its
own `/healthz`):

```bash
# Single host — fastest path is to talk to the IPC socket directly.
echo '{"kind":"health","id":"00000000-0000-0000-0000-000000000000"}' \
  | nc -U /tmp/plugin-host.sock \
  | jq '.trust.platform_signing_keys'
# Expect: ["<OLD_ARN>", "<NEW_ARN>"]

# Fleet — main mockforge surfaces the IPC payload on its healthz.
curl https://<mockforge-fleet-host>/healthz | jq '.plugin_host.trust.platform_signing_keys'
```

If hosts are still showing only `<OLD_ARN>` after 5 minutes, do NOT
proceed to step 5 — there's a propagation problem, debug the host
poll loop (`MOCKFORGE_PLUGIN_HOST_ROTATION_URL` set? bearer token
correct? `/api/internal/plugin-rotation-events` returning the event?).

### 5. Update the plugin-host release pipeline

Same as step 5 of the initial setup, but for the new key. Bundle
`platform-signing-root-v$((CURRENT_VERSION + 1)).pem` into the next
plugin-host release. This is **defense in depth** — fresh hosts that
start up during the transition window can verify the rotation event
against the embedded *previous* root, but pinning the new root in the
release means even a registry-side compromise can't unilaterally
re-rotate during the next deploy cycle.

### 6. Retire the old key

After `transition_until` has passed (default: 30 days later), retire:

```bash
# Tell the registry to drop the old key from in-memory state.
curl -X POST https://registry.mockforge.dev/api/internal/platform-signing/retire-old \
  -H "Authorization: Bearer $OPERATOR_JWT"

# Disable the old CMK in AWS — this is irreversible after 7 days.
aws kms disable-key --key-id "$OLD_ARN"

# Schedule deletion (minimum 7-day window — gives time to recover
# if step 6 was premature).
aws kms schedule-key-deletion --key-id "$OLD_ARN" --pending-window-in-days 30
```

The registry writes a `platform_signing_key_retired` row to
`audit_logs` containing both ARNs and the transition close timestamp.

### 7. Move the alias

```bash
aws kms update-alias \
  --alias-name alias/mockforge-platform-signing-active \
  --target-key-id "$NEW_ARN"
```

Update `MOCKFORGE_PLATFORM_SIGNING_KMS_KEY_ID` in the registry
deployment to the new ARN and restart. The alias move is just a
convenience for the next rotation — the registry already runs against
the new ARN since step 3.

---

## Emergency revocation

Run this when the active platform signing root is **believed
compromised** — leaked credentials, insider abuse, AWS CloudTrail
shows unexpected `kms:Sign` calls, etc.

The model: stop trusting the active key everywhere, then immediately
follow with a [normal rotation](#normal-rotation) to a fresh key. The
gap between revocation and rotation is the **outage window** — no new
cloud plugins can be loaded or verified — so prepare both before
running step 1.

### 0. Prepare in parallel

While compiling the incident response:

```bash
# Provision the replacement key (see normal rotation step 1).
NEW_ARN=$(aws kms create-key ... --output text --query KeyMetadata.Arn)

# Get the replacement public key ready to publish out-of-band — email,
# Signal, whatever the incident-response playbook says — so customers
# can manually verify rotation #N+1 when they receive it.
aws kms get-public-key --key-id "$NEW_ARN" ...
```

### 1. Revoke the current key in the registry

```bash
curl -X POST https://registry.mockforge.dev/api/internal/platform-signing/emergency-revoke \
  -H "Authorization: Bearer $OPERATOR_JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "leaked credential in CI artifact 2026-05-18"
  }'
```

The registry:

1. Refuses any further `Sign` requests against the compromised key.
2. Writes a `platform_signing_key_revoked` row to `audit_logs` with
   the operator id, ip, reason, and timestamp.
3. **Does not** publish a rotation event (there's no new key to hand
   over to yet — that's step 2).

### 2. Disable the CMK in AWS

```bash
aws kms disable-key --key-id "$OLD_ARN"
```

After this, even a CloudTrail-visible attacker can't call `Sign`.

### 3. Immediately drive a normal rotation

Follow [Normal rotation](#normal-rotation) steps 3-7 against `NEW_ARN`.
The "transition window" is reduced from 30 days to **0** for emergency
rotations — there's no `from` key to overlap with, so the event is
effectively just "everyone trust this new key, signed by … nothing".

Plugin-hosts will NOT accept a rotation event with no valid handover
signature. The way around this is the **embedded root** mechanism:
plugin-hosts trust the root that was bundled into their release. The
operator therefore ships a new plugin-host release containing the new
public key, and customers update.

> This is intentionally painful. Emergency revocation is a "trust
> reset" — by design, every plugin-host has to consciously opt back
> in to a new root.

### 4. Customer notification

Open a status-page incident, email all paying customers, file an
audit-log export with the timestamp range covering the revocation
and the next rotation event. The compliance team handles disclosure.

---

## Post-rotation verification

Whether normal or emergency, after a rotation:

### Verify the audit trail

```bash
# Query the audit log for the most recent rotation events.
curl https://registry.mockforge.dev/api/v1/internal/audit-logs \
  -H "Authorization: Bearer $OPERATOR_JWT" \
  -G \
  --data-urlencode "event_types=platform_signing_rotation_started,platform_signing_key_retired,platform_signing_key_revoked"
```

Each row's `metadata` includes the `to_public_key_b64`. Compare with
`aws kms get-public-key --key-id "$NEW_ARN"`. They must match — if
they don't, the registry is rotating against a different key than the
operator believes.

### Verify the rotation event signature

The same crypto every plugin-host runs:

```bash
cargo run -p mockforge-platform-signing --example verify-event \
  -- --event-file event.json
```

This is the explicit "I verified the handover myself" check — useful
for the post-rotation incident review and for satisfying the
"two-person-rule" expectation (one operator drives the registry, a
second independently verifies the published event).

### Verify the host fleet

```bash
# Every host's trust set surfaces via main mockforge's healthz
# (which forwards the plugin-host IPC Health payload).
for host in $(get-host-fleet); do
  curl -s "https://$host/healthz" \
    | jq -r '.plugin_host.trust.platform_signing_keys[]'
done | sort -u
```

After the transition window closes, the set should equal `{"$NEW_ARN"}`
exactly. Stragglers indicate a host that didn't poll — investigate,
restart if necessary.

The registry's own view is on
`GET /api/internal/plugin-rotation-events`:

```bash
curl -s https://registry.mockforge.dev/api/internal/plugin-rotation-events \
  -H "Authorization: Bearer $MOCKFORGE_INTERNAL_API_TOKEN" \
  | jq '{phase, trusted_key_ids}'
```

If the fleet's set and the registry's `trusted_key_ids` diverge, the
registry is the source of truth — stragglers need to refresh.

---

## Upgrade path: CloudHSM Custom Key Store

The vanilla KMS path is FIPS 140-2 Level 2. For deployments that need
FIPS 140-2 Level 3 (HIPAA, FedRAMP High, certain financial-services
contracts), upgrade to a CloudHSM-backed Custom Key Store.

This is a deliberate **upgrade**, not a migration — keys live entirely
in the customer's CloudHSM cluster and never enter the KMS service.
The trade-off is operational cost: a 2-node CloudHSM cluster runs
~$3,500/month in `us-east-1`.

**Crate-side support:** none required. The signer uses `aws-sdk-kms`,
which transparently routes through CloudHSM when the CMK's
`CustomKeyStoreId` is set. Only the runbook changes:

1. Provision a CloudHSM cluster + activate the HSM (multi-officer
   PIN ceremony — see AWS docs).
2. Create a Custom Key Store backed by the cluster.
3. In step 1 of [One-time setup](#one-time-setup), replace
   `--origin AWS_KMS` with `--origin AWS_CLOUDHSM --custom-key-store-id <id>`.
4. Everything else (alias, IAM, env var, rotation procedure) is
   unchanged.

Rotation cadence stays the same; CloudHSM-backed CMKs do not auto-rotate
(unlike standard KMS CMKs).

---

## Why the registry role cannot disable keys

The IAM policy granted to the registry role deliberately excludes
`kms:DisableKey` and `kms:ScheduleKeyDeletion`. The reason:

> If the registry process is compromised, the attacker can still
> issue `kms:Sign` requests for the lifetime of the credential — but
> they cannot **destroy** the rotation history. Disabling and
> deleting the key would lock out every plugin-host (because no
> further verification of past signatures is possible), turning a
> credential-leak into a denial-of-service.

By keeping `DisableKey` out-of-band, the operator must consciously
perform the irreversible step — typically from a separate
admin-workstation IAM identity, not the registry's runtime role.

This is the same reasoning behind `kms:DeleteAlias` exclusion: an
attacker that flips the alias to point at an attacker-controlled key
would force every subsequent rotation to start from a poisoned root.

---

## Cross-references

- RFC: [`cloud-trust-permissions-rfc.md`](./cloud-trust-permissions-rfc.md) §8.2, §9
- Crate: [`crates/mockforge-platform-signing/`](../../../crates/mockforge-platform-signing/)
- Audit-log enum: [`crates/mockforge-registry-core/src/models/audit_log.rs`](../../../crates/mockforge-registry-core/src/models/audit_log.rs)
- Migration: [`crates/mockforge-registry-server/migrations/20250101000077_platform_signing_audit_events.sql`](../../../crates/mockforge-registry-server/migrations/20250101000077_platform_signing_audit_events.sql)
- Org-scoped trust roots (contrast): [`trust_roots.rs`](../../../crates/mockforge-registry-server/src/handlers/trust_roots.rs)
- Related: [Issue #416](https://github.com/SaaSy-Solutions/mockforge/issues/416) (org trust roots),
  [Issue #549](https://github.com/SaaSy-Solutions/mockforge/issues/549) (plugin-host trust cache),
  [Issue #550](https://github.com/SaaSy-Solutions/mockforge/issues/550) (this work)
