# Cloud Collab Layer for State Machine / Graph / World State — Design

Cloud-enablement plan for `state-machine-editor`, `graph`, `world-state` nav items. Tracks task #14 in the cloud-enablement plan.

## Goal

Add a sharing/collab layer to the three editor surfaces. Local mode keeps the standalone editors; cloud mode adds workspace-scoped persistence, presence indicators, and basic version history (Figma-shaped). No real-time CRDT collaboration in v1 — just save/load + presence + version history.

## What exists

- **Pages**: `ScenarioStateMachineEditor`, `GraphPage`, `WorldStatePage`.
- **`mockforge-world-state` crate**: engine, model, query, aggregators.
- **`mockforge-collab` crate**: `client.rs`, `auth.rs`, `events.rs`, `conflict.rs`, `backup.rs` — substantial collab infrastructure already exists. Likely the right plumbing to lean on.
- **Overlap with #9**: state machines are already covered by #9 Scenario/Orchestration as one of the four `flows.kind` values. **State machine editor cloud work is therefore subsumed by #9.**

## Scope after merging with #9

Once #9 lands, the state machine editor is already cloud-shaped. That leaves only:

- **Graph page** (the dependency / orchestration graph view).
- **World State page** (the runtime state inspector).

Both are smaller scope than the original framing suggested. This task should be reframed as "Cloud Graph + World State" rather than three editors.

## What's missing for Graph page

The graph view is mostly read-only — it visualizes relationships between mocks, scenarios, and federation links. Cloud version needs:

1. **Cross-deployment data source.** Today it reads from local in-process state. Cloud needs to query the registry for relationships across the org's workspaces.
2. **Workspace/federation filter.** Default to current workspace; allow zooming out to org-wide or federation-wide views.
3. **Drill-down links.** Click a node → open the underlying resource.

No persistence needed (it's a derived view), so this is mostly a query-aggregation handler.

## What's missing for World State page

The world-state page surfaces runtime state of a *running* MockForge. In cloud, the running thing is a hosted mock.

1. **Per-deployment state inspection.** New endpoint: `GET /api/v1/hosted-mocks/{deployment_id}/world-state` returning the current world-state snapshot.
2. **Live subscription** for state changes (SSE or WebSocket).
3. **State mutation from cloud UI.** Set/unset state values from the cloud (debugging tool). Must be gated behind admin role.

Again, no persistence — world state is a runtime view. The page becomes a viewer for hosted-mock runtime state.

## Cloud architecture

### Proposed routes

```
# Graph
GET    /api/v1/organizations/{org_id}/graph                       # nodes + edges
GET    /api/v1/workspaces/{workspace_id}/graph                    # workspace-scoped subgraph

# World State
GET    /api/v1/hosted-mocks/{deployment_id}/world-state            # full snapshot
GET    /api/v1/hosted-mocks/{deployment_id}/world-state/stream     # SSE updates
PATCH  /api/v1/hosted-mocks/{deployment_id}/world-state            # mutate (admin only)
DELETE /api/v1/hosted-mocks/{deployment_id}/world-state/{key}
```

No new tables — both pages are runtime/derived views.

## Plan tiers

- **Free**: graph view only (no world-state inspection — no hosted mocks on free tier).
- **Pro**: graph + world-state read.
- **Team**: + world-state mutation.
- **Enterprise**: + cross-federation graph view, world-state history (timeline of state changes).

## UI changes

1. `AppShell.tsx:217` — add `'graph'`, `'world-state'` to `cloudNavItemIds`. (`'state-machine-editor'` is added by #9.)
2. **GraphPage rewrite for cloud mode**: workspace/federation filter, node detail drawer with deep links.
3. **WorldStatePage rewrite for cloud mode**: deployment selector, live SSE updates, optional mutation panel for admins.
4. **`mockforge-collab` evaluation**: figure out what's already wired. The crate has substantial code (`client.rs`, `conflict.rs`, `auth.rs`); if it's a complete in-process collab layer, the cloud work might be smaller than expected. Worth a 0.5-day spike before estimating tightly.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 0 | Spike: audit `mockforge-collab` to see what's reusable | ~0.5 day |
| 1 | Graph aggregation handler (nodes + edges from registry data) | ~2 days |
| 2 | World-state read endpoint + hosted-mock integration | ~1.5 days |
| 3 | World-state SSE stream | ~1 day |
| 4 | World-state mutation endpoint with admin role gate | ~1 day |
| 5 | UI rewrites (Graph + WorldState) | ~3 days |
| 6 | E2E (graph render + world-state read/mutate) | ~1.5 days |

Total: ~10.5 working days for v1 (after the spike clarifies collab-crate reuse).

## Decisions

### Subsume state-machine-editor under #9

**Decision: yes.** State machines are flow-kind in #9. Don't build cloud surface twice. This task ships only Graph + World State.

### No real-time CRDT collaboration in v1

**Decision: defer.** Cloud-mode users get save/load + version history (via #9 for flow editors); concurrent editing falls back to last-write-wins with a warning if someone else has the page open. Real-time collab is hard, expensive, and not the differentiating feature. Revisit when customers ask.

### Graph is a derived view, not a stored resource

**Decision: query-time aggregation.** Don't materialize the graph. Workspace data is already authoritative; the graph is a read.

## Out of scope for v1

- Real-time collaborative editing (CRDT or OT).
- Presence indicators (who's looking at this page).
- Comment threads on graph nodes.
- World-state history / time-travel for hosted-mock runtime state (could come from #10 Time Travel snapshots).
- Graph layout customization (use auto-layout only).

## Open questions

1. `mockforge-collab` looks substantial — is it already wired into the local UI, or vestigial? Spike phase 0 answers this. If it's wired, we're closer to real-time collab than this doc assumes.
2. World-state mutation from cloud is a powerful debugging tool but also dangerous. Should it be limited to non-prod environments only? Probably yes — gate by `hosted_deployments.environment != 'production'`.
3. Graph view for very large orgs (>100 workspaces) may need pagination / clustering. Start with a hard limit of 500 nodes; render an "Org too large for full graph; filter by workspace" message above that.
