# Cloud portal authenticated QA fixes — 2026-10-02

Account: `admin@mockforge.dev`, owner/admin. Browser: headless Chromium, desktop 1440×1000, mobile spot check 390×844.

## Fixes and verification

| Punch-list items | Resolution | Verification |
| --- | --- | --- |
| 1 Resilience crash | Preserve `runtime_state` and `data` in resilience responses; validate arrays and display request errors. | Browser load/navigation; response-envelope regression. |
| 2 Stale boundary/view | Reset boundaries on pathname changes and remount routed content. | Boundary regression; SPA navigation through all enabled pages. |
| 3 Snapshot diff crash | Remove the legacy interception of real cloud snapshot API requests. | Actual snapshot comparison; network pass-through regression; capture/diff/delete round trip. |
| 4 Broken deletes | Replace suppressed native browser confirmations with visible app dialogs; keep deletion errors visible. | Named service, deployment, snapshot, and tunnel deleted through the UI. |
| 5–7 Organization, tokens, BYOK loading | Restore an access token along with cookie authentication; never send `Bearer null`; show query failures and retries. | Full reload into all three pages. |
| 8 Fixture creation | Reconcile workspace context; display server errors inside the dialog; use the same cache key for the fixture list and mutations. | Live create/delete; inline-error regression; cache refresh regression. |
| 9 Fitness creation | Cloud authoring now sends supported kinds: latency threshold, error rate, contract stability, with evaluator settings and a valid target. | Live contract-stability create/delete; form regressions cover all three kinds. |
| 10 Federation route testing | Keep typed request paths, send the request, and show server routing errors in the page. | Typed path remains enabled and triggers POST; empty federation reports no matching route; a temporary federation routes a matching service successfully. |
| 11 Theme | Synchronize mode aliases, use the resolved mode for toggling, and share the palette store with preferences. | Repeated light/dark toggles and reload; store and toggle tests. |
| 12 Search | Add a page command palette with Ctrl+K/⌘K, arrow selection, Enter navigation, and visible empty results. Explicit logs/services scopes navigate to their filtered pages. | Ctrl+K and Enter workflows. |
| 13 Deleted workspace | Reconcile persisted selection with the server list, clear unavailable selections, refresh metadata, and ignore obsolete load responses. | Simulated deleted persisted workspace on reload; store regressions. |
| 14 Spacing | Consolidate counters into complete text strings and add explicit separation to metadata labels. | Browser text checks and rendering sweep. |
| 15 Wrong editor/breadcrumb | Preserve the distinct existing pages and reset route state. | State Machines, Scenario Studio, Orchestration Builder/Execution, and Cloud Flows all render their own content. |
| 16 TODO copy | Replace development-roadmap copy with actual supported behavior; retain the behavioral-cloning simulation limitation. | Rendered page inspection. |
| 17 Accessible actions | Label fixture, hosted deployment, and repository actions; make FAQ toggles keyboard accessible with expanded state; mark decorative support icons. | Browser action selectors and existing fixture/toggle tests. |
| 18 Activity users | Display the current user's name, other full user IDs, or System rather than ambiguous eight-character prefixes. | Dashboard activity feed. |
| 19 Job workspace | Display the selected workspace name and expose its complete ID in the tooltip; requests use the same active ID. | Workspace recovery and generator page. |
| 20 Deployment slug | Continue generating from the complete name until the slug is manually edited. | Character-by-character entry plus custom-slug preservation. |
| 21 Group folding | Add accessible group toggles and persist folded groups. | Fold/unfold browser check. |
| 22 Federation totals | Render zero service counts explicitly and show “None configured” for empty reality levels. | Empty federation detail. |
| 23 Local-only gating | Confirmed intentional for Proxy Inspector and SMTP/MQTT/Kafka/AMQP. | Cloud allowlist and disabled sidebar controls. |

## Cleanup

Deleted the four requested artifacts through the UI: QA-TEST Service, QA-TEST Mock, QA-TEST Snapshot, and QA-TEST Reservation (`qa-test-reservation-12345`). The previously deleted QA-TEST Workspace remains absent. Temporary fixtures, snapshot, fitness function, monitored-service, and federation records used for verification are also cleaned up.

## Checks

- 89 targeted tests passed across 12 files.
- Cloud production build passed.
- 22 focused live workflow checks passed; see `cloud-portal-2026-10-02-workflows.json`.
- All 59 enabled navigation pages passed on both the compiled candidate and live deployment, with zero uncaught JavaScript errors. See `cloud-portal-2026-10-02-navigation.json`.
- A local-only snapshot query still ran from Virtual Backends during the first sweep. It is now disabled in cloud mode, so cloud operations use the workspace snapshot endpoints. A final live check confirmed the local request is absent.
- Fresh signed-out browsers may return 401 from `/auth/me` and 400 “Missing refresh token”; these are expected authentication probes, not failed logged-in requests.
- Strict application type checking is still blocked by existing repository diagnostics, including the unsupported `erasableSyntaxOnly` option under TypeScript 5.6. The final check adds no diagnostics relative to the earlier check. The root `tsc --noEmit` script has no source files, so it should not be treated as application type-check evidence.
- No emails/invitations, billing changes, support submissions, or test-runner jobs were triggered.
- Impeccable's copied user installation was updated to v4.5.0 for future sessions.

Deployment: qa-fix-20261003T014215Z (atomic static release; previous release retained; existing hashed JS/CSS chunks retained for already-open tabs).
