# Cloud AI Studio + MockAI — Design

Cloud-enablement plan for the five AI nav items (`ai-studio`, `mockai`, `mockai-openapi-generator`, `mockai-rules`, `voice`). Tracks task #1 in the cloud-enablement plan.

## Goal

Move the AI surface from local-only to cloud, billed per token consumed, with BYOK as a pricing tier and platform-key access on paid plans. Lowest-lift / highest-revenue starting point because:

- LLM calls are already remote — there is no local-only compute to relocate.
- Most billing plumbing exists: `usage_counters.ai_tokens_used`, `/api/v1/usage/ai-tokens`, `byok` org_setting, BYOK page in nav.
- `/api/v1/organizations/{org_id}/settings/ai` already provides per-org AI controls.

## Current local architecture

UI talks to the embedded admin server:

- `crates/mockforge-ui/ui/src/services/api/mockai.ts` — `/__mockforge/api/mockai/{generate-openapi,rules,learn,...}`
- Pages: `AIStudioPage`, `MockAIPage`, `MockAIOpenApiGeneratorPage`, `MockAIRulesPage`, `VoicePage`
- Backend: `mockforge-ai-core` (`llm_client`, `embedding_client`, `rule_generator`, `validation_generator`, `behavior`, `cache`, `history`, `session`) + `mockforge-core::ai_studio`
- Local config picks the LLM provider (OpenAI / Anthropic / Ollama) from env vars or `mockforge.yaml`.

## Cloud architecture

### What's missing

1. **Registry-server handlers** for the AI surface. None of `crates/mockforge-registry-server/src/handlers/` covers AI today — `routes.rs` only exposes `organization_settings::ai` and `usage::report_ai_tokens`.
2. **An LLM proxy layer** in the registry that:
   - Looks up org's plan and BYOK config.
   - Routes to BYOK key (if configured) or platform key (Pro/Team plans).
   - Enforces `ai_tokens_per_month` quota *before* the call (read `usage_counters`).
   - Records usage *after* the call (reuse `UsageCounter::increment_ai_tokens`).
   - Streams responses back to the UI.
3. **UI cloud-mode wiring.** `mockai.ts` and the AI Studio service layer hardcode `/__mockforge/api/mockai/*`. Need a base-URL switch driven by `isCloudMode()` from `utils/cloudMode.ts`, and the cloud paths should live under `/api/v1/ai-studio/*` and `/api/v1/mockai/*` to match other cloud handlers.
4. **Nav allowlist.** Add the five IDs to `cloudNavItemIds` in `AppShell.tsx:217`.
5. **Voice page transport.** `VoicePage` uses Web Speech API client-side, but any LLM round-trips need to flow through the cloud proxy too.

### Proposed registry routes

```
POST /api/v1/ai-studio/chat                    # streaming completion
POST /api/v1/ai-studio/debug/analyze-with-context
POST /api/v1/mockai/generate-openapi
POST /api/v1/mockai/learn
GET  /api/v1/mockai/rules/explanations
GET  /api/v1/mockai/rules/{id}/explanation
POST /api/v1/voice/transcribe                  # if we host STT; otherwise client-side
```

All gated by the existing auth middleware + a new `require_ai_quota` middleware that:
- Loads the org's plan limits.
- Compares `usage_counters.ai_tokens_used` against `ai_tokens_per_month`.
- Returns `429 Too Many Requests` with a `quota_exceeded` body when over limit.
- Lets the request through and lets the handler post back actual usage on completion.

### Provider routing

Implemented as a thin `LlmRouter` in a new `mockforge-registry-server::ai` module:

```
fn pick_provider(org: &Org, plan: &Plan) -> Provider {
    match (plan.tier, &org.byok_config) {
        (Tier::Free,  Some(cfg)) => Provider::Byok(cfg),     // free + BYOK = allowed
        (Tier::Free,  None)      => Provider::Disabled,      // free w/o BYOK = blocked
        (_,           Some(cfg)) => Provider::Byok(cfg),     // paid prefers BYOK
        (_,           None)      => Provider::Platform,      // paid falls back to platform key
    }
}
```

Free + no BYOK gets a clear "Upgrade or add BYOK" UI prompt rather than silent failure.

### Token metering

Reuse what's there:
- After each LLM call, the handler calls the same code path as `report_ai_tokens` (the SQL increment in `UsageCounter::increment_ai_tokens`).
- Embeddings count too — split into `prompt_tokens` + `completion_tokens` + `embedding_tokens` if we want pricing differentiation, otherwise sum them as `ai_tokens`.
- The `usage_counters` row already aggregates monthly; the existing `usage` page renders the bar.

## Data model

Likely no new tables needed for v1. Optional additions if we want them:

- `ai_request_log` — per-request audit (org_id, user_id, route, model, prompt_tokens, completion_tokens, latency_ms, status, created_at). Useful for the AI Studio history UI and for debugging customer issues. Add only if we surface it in the UI; otherwise skip.
- Extend `byok` org_setting to support per-feature provider routing (e.g., chat = Anthropic, embeddings = OpenAI). Defer until a customer asks.

## UI changes

1. `crates/mockforge-ui/ui/src/components/layout/AppShell.tsx:217` — add `'ai-studio'`, `'mockai'`, `'mockai-openapi-generator'`, `'mockai-rules'`, `'voice'` to `cloudNavItemIds`.
2. `services/api/mockai.ts` — switch base URL using `isCloudMode()`. Keep the local fallback for OSS users.
3. New `services/api/aiStudio.ts` — a cloud-aware client for the AI Studio endpoints currently called from `pages/AIStudioPage.tsx`.
4. **Quota-exceeded handling.** Surface the 429 response as a non-dismissable banner on the affected pages with an "Upgrade" link to `/billing`.
5. **BYOK status badge.** Show "Using your key" vs "Using platform credits" near the AI Studio header so users know what's billing.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Registry handlers (chat, generate-openapi, learn, rules) + LlmRouter + quota middleware | ~3-4 days |
| 2 | Token metering integration with `usage_counters` (mostly reuse) | ~0.5 day |
| 3 | UI cloud-mode wiring + nav allowlist + 429 handling | ~1.5 days |
| 4 | Voice page cloud transport (defer if Web Speech is sufficient) | ~1 day |
| 5 | E2E test coverage on the cloud path (BYOK + platform + quota) | ~1 day |

Total: ~7-8 working days for v1.

## Out of scope for v1

- Per-feature provider routing in BYOK (single provider for everything).
- Streaming usage updates (poll the existing usage endpoint).
- Cross-org rate limiting beyond the monthly quota.
- AI request log UI (add later if support tickets demand it).
- Fine-tuning / custom-model upload.

## Decisions

### 1. Platform-key access on Pro

**Decision: include a metered monthly AI quota in Pro by default; sell overage as token top-up packs; Team/Enterprise gets a higher baseline quota. BYOK bypasses platform metering entirely.**

Rationale:
- "AI included" is the marketing line that wins against competitors who gate it behind add-ons (Postman, Insomnia have done the gated-AI thing and customers complain).
- A baseline quota protects unit economics — runaway usage hits the cap and users either upgrade, top up, or attach BYOK.
- Top-up packs (e.g., "+1M tokens for $X") give a clean overage path without forcing a plan upgrade for a single bursty month.
- BYOK becomes the answer for compliance buyers (we never see their prompts) and for power users who already have provider credits — neither group should be paying us per token on top of their provider bill.

Suggested initial quotas (tune after launch with real data):
- Free: 0 platform tokens (BYOK only).
- Pro: 1M platform tokens / month included.
- Team: 5M platform tokens / month included, pooled across the org.
- Top-up: $X per 1M additional tokens, valid until end of billing period.

Pricing page copy: *"AI Studio is included on Pro and above. Bring your own provider key any time to skip the quota."*

### 2. Embeddings cache

**Decision: keep per-org in Redis.** Reuses the existing `mockforge-registry-server::redis` import; cache key is `embedding:{org_id}:{sha256(text)}`. Per-org keying keeps cache hits accurate and avoids cross-tenant leakage. TTL: 30 days, configurable.

### 3. Free-tier BYOK abuse cap

**Decision: apply a request-rate cap instead of a token quota for free + BYOK.**

Rationale: with BYOK, the user pays their own provider bill, so a token cap is the wrong knob — it just punishes legitimate use. What we actually need to prevent is someone routing arbitrary LLM traffic through our proxy as a free relay/load-balancer.

Caps for free + BYOK:
- 60 requests / minute / org (short-burst protection).
- 10,000 requests / day / org (daily ceiling).
- Soft warn at 80%; hard 429 at 100%.

These are enforced by the same `require_ai_quota` middleware, just in a different code branch when `provider == Byok && plan.tier == Free`.

Paid + BYOK (Pro/Team) gets the same rate caps but at higher ceilings (e.g., 600 rpm, 100k/day) since they're paying for the relationship.
