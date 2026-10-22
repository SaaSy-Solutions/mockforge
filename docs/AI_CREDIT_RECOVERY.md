# Recovering uncertain platform AI credits

Platform AI admission reserves a bounded budget before contacting a provider.
An interrupted or failed response may leave the reservation `uncertain` because
the provider could have consumed credits. Age alone never releases that budget.

Free organizations owned by the same verified account share a monthly AI
allowance. Admission holds budget in both the shared account ledger and the
organization counter. Organization deletion does not remove shared account
spend. Both ledgers use the same captured UTC month; settlement and operator
recovery update the admission's original rows after a month changes. Unknown
usage remains held in both ledgers. Paid-plan and customer-funded BYOK calls do
not consume this Free account pool. Trusted administrative quota grants can
raise the shared allowance; creating another default Free organization cannot
multiply it.

A platform administrator can reconcile an **uncertain** reservation after
checking authoritative provider records. Use the authenticated endpoint:

```text
POST /api/v1/admin/organizations/{org_id}/ai-reservations/{reservation_id}/reconcile
Content-Type: application/json

{"actual_tokens": 901, "evidence_reference": "provider-ticket-123"}
```

`actual_tokens` is total provider usage, including input and output. Use zero
only when provider records establish that no tokens were consumed. Preserve the
provider evidence in your operator ticket system; send only its opaque identifier
here. References accept 1–128 ASCII letters, digits, dots, underscores and
hyphens. Do not send credentials, provider response bodies or signed URLs.

The endpoint checks the caller's current administrator status, binds the target
organization for row-level security, then locks the shared account allowance
before the original organization usage counter. It charges the original admission period, even if the call belongs to an earlier
month, records the administrator and evidence reference, and appends an audit
entry in the same transaction. An audit failure rolls accounting back. Actual
usage above the reserved budget is charged in full.

Repeating the same token total and evidence reference succeeds without another
charge or audit entry. A conflicting retry fails. A reservation in another
organization is not accessible through the supplied organization ID.

New platform calls use a database execution fence (migration 108). Recovery
acquires that same fence without waiting and rejects a still-running task.
A pending call whose task has stopped can be reconciled with final authoritative
provider usage evidence. A task racing recovery checks reservation state under
the fence before contacting the provider, preventing execution after recovery.
Never infer zero usage from process death, elapsed time or a released database
connection: the remote provider may still be processing. Confirm final usage
before submitting the evidence reference. Do not manually change reservation
state or counters to bypass these controls.

### Legacy (unfenced) pending calls

A pending reservation without `execution_fenced` predates migration 108 and is
ineligible for online recovery. Released builds cannot create one: the
reservations table (migration 96) and the fence (migration 108) ship and apply
together, and every admission inserts `execution_fenced = TRUE`. Only an
environment that ran an unreleased intermediate build can hold such rows.

Before and after every deployment that applies these migrations, confirm the
count is zero with the owner role:

```sql
SELECT count(*) FROM ai_token_reservations
WHERE state = 'pending' AND NOT execution_fenced;
```

A non-zero count means that environment ran unreleased code. Do not reconcile
those rows or edit them by hand: provider usage for them cannot be fenced, so
resolve that environment by restoring it rather than settling in place.

## Verified identity continuity

Migration 106 privately binds verified email identities to retained Free allowance
history. Deleting and recreating an account with the same verified email carries
spent AI credits and hosted requests into the recreated account. Existing usage
on that account is combined once; repeated verification does not add it again.
Outstanding platform calls must still be resolved before account deletion.

The binding stores a salted SHA-256 digest rather than a raw email. The database
salt and digest table have row-level security enabled with no policy and no
grants, so request roles cannot read them; only owner-run functions and the
owner-pool cleanup worker do.
These digests remain pseudonymous anti-abuse data, not anonymous data, and are
covered by the Free allowance retention disclosure. Deleted-account bindings
expire after 35 days; the daily owner-pool cleanup normally removes them within
36 days. Previous verified
email aliases remain attached to a live account's allowance identity; attempting
to reuse an alias while that account exists is rejected. No deployed acceptance
or migration-upgrade qualification is claimed by the local runtime tests.

Provider execution fences use a separate lazy database pool with the same
restricted role as API requests. It is capped at four connections per process
(or the configured database pool limit when smaller). Include these additional
connections when sizing PostgreSQL. Slow provider calls do not consume the
request accounting/recovery pool. Waiting tasks have already reserved credits;
their state is checked under the execution fence before contacting a provider.
