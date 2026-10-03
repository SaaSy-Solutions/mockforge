# Cloud portal re-QA follow-up — 2026-10-03

The owner confirmed that the original blockers and majors are fixed. This follow-up addresses the newly reported flow creation HTTP 422, the services route-counter space, federation Back visibility, and portal text/action readability.

## Findings and changes

- Reproduced flow creation against the live registry: HTTP 422, `missing field config`. The request type and all five editors used `initial_config`; they now send `config` without changing the stored flow definition. Follow-through testing exposed a second mismatch: version reads used a nonexistent nested URL instead of `/api/v1/flow-versions/{version_id}`. The endpoint is corrected, the editor stays on its newly saved version, and load/save errors are visible. Unsupported changelog/set-current request fields and the changelog input were removed; registry saves always make the new version current.
- Federation details use a visible, labeled outline Back button. Snapshot and flow list pages have no Back control to repair. Flow editor dialogs have a labeled Back to flows action.
- Shared and legacy dialogs use readable neutral text and placeholders. Disabled buttons remain opaque with a muted surface and readable label; selected accent colors use contrast-safe primary foreground/background pairs in both themes. Tailwind dark variants now follow the portal class toggle instead of the operating system, correcting white dialogs with dark-theme text. Hosted-mock plan badges use explicit foreground/background pairs; colored status controls and cyan/amber actions use readable shades. Dialog actions, federation routing controls and flow history wrap at small widths.
- The services route counter emits a single spaced text string.
- Protocol brokers remain intentionally local-only. No email, notification, billing, support, or runner mutations are part of verification.

## Validation

- UI suite: 976 tests across 93 files pass. Type checking, lint (no errors), and cloud production build pass.
- Authenticated Playwright: scenario, orchestration, state-machine and chain creation, edited configuration save, version history and deletion pass against the live registry. All temporary flows were removed. Existing E2E data was preserved.
- Readability: 44 inspected views across ten creation dialogs, the flow editor and federation details, using 1440×1000 and 390×844 viewports and both themes. No measured text-contrast failures, button collisions or horizontal overflow remain. This is representative browser coverage of the shared dialog implementations, plus source inspection of legacy neutral/status styles; it is not a claim that every conditional dialog state was opened.
- Legacy time-travel and request-log actions now share the same readable primary pair. Primary controls meet 4.5:1 for all five accents in both themes; a store-level regression checks that a theme round trip preserves the selected accent and readable foreground.
- Browser JavaScript errors: none. The only failed API requests were expected unauthenticated session probes before sign-in.
- Documentation build and docs/config/CLI drift gates pass locally. CI's original pinned mdBook download repeatedly returned HTTP 503; a fresh URL returned the same artifact successfully. Both docs workflows now use a shared installer with SHA-256 verification and a URL unique to each run attempt. The pinned 0.4.40 build also passes locally.
- Impeccable detector findings concerned existing typography/decorative choices; these were retained to preserve the portal identity.

Protocol gating and the absence of Back buttons on list pages are intentional. Real mobile devices, runner execution and the previously excluded email/billing/support mutations remain outside this verification. Deployment and live post-deployment verification are tracked in PR #1146.
