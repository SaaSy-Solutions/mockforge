# Intentionally-Local Features — Decision Record

The cloud-enablement plan (#1–#15) covers features that should be cloud-enabled. This document captures the inverse: features that **stay local** and the reasoning, so future contributors don't relitigate.

This is a decision record, not a design doc. Tracks task #16 in the cloud-enablement plan.

## Features staying local

### Protocol brokers: SMTP, MQTT, Kafka

**Sidebar items**: `smtp-mailbox`, `mqtt-broker`, `kafka-broker`.

**Why local:**
- Running brokers in cloud means standing up Postfix/Mosquitto/Kafka clusters per tenant, which is heavy infra and competes directly with established managed-service vendors (SendGrid, HiveMQ, Confluent Cloud, AWS MSK).
- The mocking value-add is in the *behavior* (responses, scenarios) not in being a managed broker.
- Cloud customers who need a managed broker should use a real managed broker; MockForge proxies to it via the existing protocol clients.

**When this might change:** if a customer-facing test scenario requires a *sandboxed* broker that they cannot stand up themselves and the existing managed services don't fit (e.g., specific protocol versions, deterministic message ordering). Until then, no.

**Crate impact:** `mockforge-smtp`, `mockforge-mqtt`, `mockforge-kafka`, `mockforge-amqp` stay as local protocol implementations. They can still ship logs/metrics to cloud via #2 Observability if `--cloud-ship` is enabled.

---

### Virtual backends

**Sidebar item**: `virtual-backends`.

**Why local:**
- Virtual backends model in-memory data stores that mocks can read from / write to. They're a developer-experience feature for stateful mocking, not a hosted service.
- Hosted-mocks already provide the cloud-side equivalent: a workspace's state lives in the deployment.
- Building cloud virtual-backends would duplicate hosted-mock state management.

**When this might change:** if customers ask for a way to share virtual-backend data across multiple hosted mocks (cross-deployment shared state). At that point, it's basically a shared cache product — could live as a cloud add-on.

---

### Proxy inspector

**Sidebar item**: `proxy-inspector`.

**Why local:**
- The inspector captures local network traffic from the user's machine. Without an agent installed on the user's network, the cloud can't see this traffic.
- Building a cloud agent that ships traffic captures upward duplicates #6 Recorder.
- The use case (inspect what my local app is sending) is inherently local.

**When this might change:** if we expand to a "proxy as managed service" (intercept traffic between two cloud services), but that's a different product, not a cloudification of this feature.

**Migration path for users who want cloud-shipped captures:** use #6 Recorder with `--cloud-ship`; it's the same traffic data with a different presentation.

---

### Plugin loader

**Sidebar item**: `plugins` (the loader/management page; `plugin-registry` marketplace is already cloud).

**Why local:**
- Plugins are dynamic libraries (.so/.dylib) loaded into the MockForge process. Loading happens in the runtime, not in the cloud control plane.
- The cloud play already exists: `plugin-registry` is the discovery/marketplace surface, and hosted-mocks load plugins server-side based on workspace config.
- Splitting the loader UI to cloud would require replicating runtime state (which plugins are loaded, version, health) and offers no monetization angle.

**When this might change:** never, probably. The split (registry = cloud, loader = runtime) is correct.

---

## Features that look local but aren't (already cloud)

For completeness, since these can confuse new contributors:

| Sidebar item | Status |
|---|---|
| `dashboard`, `workspaces`, `federation` | Already cloud |
| `services`, `fixtures`, `hosted-mocks` | Already cloud |
| `template-marketplace`, `scenario-marketplace`, `plugin-registry` | Already cloud |
| `pillar-analytics`, `status` | Already cloud |
| `config`, `organization`, `billing`, `api-tokens`, `publisher-keys`, `byok`, `usage` | Already cloud |
| `faq`, `support` | Already cloud |

See `crates/mockforge-ui/ui/src/components/layout/AppShell.tsx:217` (`cloudNavItemIds`) for the canonical allowlist.

---

## How to challenge a "local" decision

If you think a feature listed here should become cloud, the bar is:

1. **Identifiable customer demand.** Not "would be cool" — actual user requests in support tickets, sales calls, or community discussion.
2. **Differentiation vs. existing managed services.** "What does MockForge do better than $managed_service for this?" If the answer is "nothing," don't build it.
3. **Pricing-dial story.** Every cloud feature should have a metered or tiered angle. If it can only be free or flat-fee, it's not pulling its operational weight.
4. **Integration with the existing pricing dials.** Use `usage_counters` columns; don't invent new metering systems unless the resource genuinely doesn't fit.

Open a design doc following the format in `docs/cloud/CLOUD_*_DESIGN.md` and propose flipping the decision before writing code.
