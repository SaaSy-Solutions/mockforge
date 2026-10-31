# Free hosting rollout

Free mocks run in the registry's isolated local runtime. Request admission uses
an atomic account allowance shared across the owner's organizations. Paid
external backends remain available on eligible subscriptions.

Platform-funded AI is disabled by default, independently of API keys and plan
allowances. Follow [AI credit recovery and activation](AI_CREDIT_RECOVERY.md)
before enabling `MOCKFORGE_PLATFORM_LLM_ENABLED=true`. BYOK remains available.
This PR does not activate a cloud provider or change the privacy effective date.

## Subscription downgrades

Cloudflare runtime lookup, redirects, usage acknowledgements, start and redeploy
reject an effectively Free subscription. The Worker may retain its existing
runtime for up to its 30-second refresh interval. This is not instantaneous
revocation of in-flight requests.

The orchestrator's cleanup poll also pauses existing external deployments after
a downgrade. It locks each deployment, rechecks its effective subscription, and
requires confirmation from Cloudflare or Fly before recording `stopped`. Failed
provider calls retain state and capacity and are retried. The poll runs every
10 seconds; provider latency, errors, backlog and an unavailable orchestrator
can delay revocation. Direct Fly URLs can remain reachable until shutdown is
confirmed. Do not promise an immediate hard cap for a previously paid external
deployment during this transition. Watch the downgrade-stop warnings and resolve
provider configuration failures before completing a rollout.

Stopping preserves deployment configuration. Restoring a paid subscription permits
restart; a Free customer can create a local mock after the old slot is released.

## Database acceptance

Use a disposable PostgreSQL database and a separate non-owner, non-BYPASSRLS
runtime role. Apply all migrations through the current head, including 108 and
the hosted usage batch receipt migration. Verify the dedicated AI execution pool
uses the restricted role, is capped at four connections, and cannot starve the
request pool. Check for legacy unfenced pending AI reservations before activation
as described in the recovery runbook. Never settle them manually.

The ignored `free_hosted_quota` tests exercise real PostgreSQL and loopback HTTP
fixtures. Set `TEST_DATABASE_URL`, `DATABASE_URL`, `APP_DATABASE_URL`, a synthetic
`JWT_SECRET`, `MOCKFORGE_MOCKS_DOMAIN`, and a synthetic
`MOCKFORGE_INTERNAL_API_TOKEN`. Leave `FLYIO_API_TOKEN` and Cloudflare deployment
credentials unset. Run the allowance suite with explicit local activation:

```sh
MOCKFORGE_PLATFORM_LLM_ENABLED=true scripts/dev/cargo test -p mockforge-registry-server --test free_hosted_quota -- --ignored --test-threads=1 --skip disabled_platform_blocks_direct_execution_and_preserves_byok
```

Run the disabled-platform test separately with the activation variable unset.
It must reject direct platform execution before reserving credits or contacting
the local provider, while allowing BYOK. The `free_hosted_upgrade` test requires
an additional empty `TEST_UPGRADE_DATABASE_URL`; it applies through migration
107, inserts a synthetic legacy pending reservation, applies current migrations,
and verifies the preflight still detects that unresolved reservation.
