# Performance Page — Cloud Strategy Decision

Decision record for task #11. Unlike #1-#10, this is not a design doc for a new cloud feature — it's a decision about whether `performance` should become its own cloud surface or be folded into existing cloud features.

## Context

The local **`performance`** page (`PerformancePage.tsx`) surfaces metrics from `mockforge-performance`:
- Latency injection profiles.
- Traffic-shaping config.
- Bottleneck simulation.
- A live metrics view of those simulations running in-process.

It's a *configuration + observation* page for in-process performance simulation, not a benchmarking page (that's `mockforge-bench`, covered by #4 Test Execution).

## The decision

**Fold the performance page into `hosted-mocks` rather than building a parallel cloud surface for it.**

## Rationale

1. **The configurable side belongs to the mock that's running.** Latency profiles, traffic shaping, and bottleneck simulation are properties of a specific mock instance. The natural cloud-mode UX is "I'm editing hosted-mock X → tune its performance characteristics." Breaking that into a separate top-level page would force users to context-switch.

2. **The observation side belongs to Observability/Metrics (#2).** Live metric views of latency, throughput, and error rates are exactly what the cloud Metrics page covers once #2 lands. Building a second metrics view would duplicate work.

3. **A standalone "Performance" cloud page would be valueless without a target.** Cloud-mode users don't have an in-process MockForge to configure — they have hosted mocks. So the page would either be empty or be a thin wrapper over hosted-mocks settings, which is exactly what folding accomplishes.

4. **Effort: very low.** ~1 day to:
   - Add a "Performance" tab/section to the hosted-mock detail page (already exists in cloud).
   - Move the latency/traffic-shaping form components from `PerformancePage.tsx` into a new `HostedMockPerformanceTab.tsx`.
   - Wire it to the existing hosted-mocks PATCH endpoint with a `performance_config` field.

## What this means for `performance` nav item

- **Remove from sidebar in cloud mode.** It already shows as "Local only" with a lock badge, but the better answer is to omit it entirely from `navSections` in cloud mode. Local-mode users keep the standalone page (since they're configuring their own in-process MockForge).
- **Keep `mockforge-performance` crate as-is.** It's library code; the move is purely UI-level.

## Schema/API changes

Just one: extend the hosted-mock config to include performance settings.

```sql
-- New field on existing hosted_deployments
ALTER TABLE hosted_deployments ADD COLUMN performance_config JSONB;
```

Or add it inside the existing `config JSONB` if there's already one (likely — `hosted_mocks` has been actively developed). Check before migrating.

## UI changes

1. `AppShell.tsx` — exclude `'performance'` from `navSections` when `isCloudMode()` is true. Currently it shows as Local-only; change it to be hidden entirely in cloud builds. (Or leave the local-only badge — both are fine; hiding is cleaner.)
2. **HostedMocksPage** detail view — add Performance tab with latency / traffic-shaping / bottleneck controls.
3. **Live metrics** for performance live in #2 Observability — no cross-link work needed beyond the standard metrics filtering.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Hide `performance` nav item in cloud mode | ~0.5 hour |
| 2 | Add performance_config to hosted-mock schema (or extend existing config) | ~1 hour |
| 3 | Move performance form components into HostedMockPerformanceTab | ~3 hours |
| 4 | Wire saves to hosted-mock PATCH endpoint | ~2 hours |
| 5 | Smoke test | ~1 hour |

Total: **~1 working day.** This is the smallest task in the cloud-enablement plan.

## Out of scope

- Standalone "Performance" cloud page.
- Cross-deployment performance comparisons (could be useful, but #2 Metrics covers it generically).
- AI-driven performance tuning suggestions (defer; AI Studio #1 can host this if customers ask).

## Verification

When this is done, the cloud user's mental model is:
- "I want to configure how my mock behaves" → hosted-mocks detail → Performance tab.
- "I want to see how my mock is behaving" → Metrics page (#2).
- "I want to load-test my mock" → Test Execution (#4).

No standalone Performance page is necessary or desirable.
