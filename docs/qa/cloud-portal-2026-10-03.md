# Cloud portal re-QA follow-up — 2026-10-03

The owner confirmed that the original blockers and majors are fixed. This follow-up addresses the newly reported flow creation HTTP 422, the services route-counter space, federation Back visibility, and portal text/action readability.

## Findings and changes

- Reproduced flow creation against the live registry: HTTP 422, `missing field config`. The request type and all five editors used `initial_config`; they now send `config` without changing the stored flow definition. Follow-through testing exposed a second mismatch: version reads used a nonexistent nested URL instead of `/api/v1/flow-versions/{version_id}`. The endpoint is corrected, the editor stays on its newly saved version, and load/save errors are visible. Unsupported changelog/set-current request fields and the changelog input were removed; registry saves always make the new version current.
- Federation details use a visible, labeled outline Back button. Snapshot and flow list pages have no Back control to repair. Flow editor dialogs have a labeled Back to flows action.
- Shared and legacy dialogs use readable neutral text and placeholders. Disabled buttons remain opaque with a muted surface and readable label; selected accent colors use contrast-safe primary foreground/background pairs in both themes. Dialog action rows and flow editor history wrap at small widths.
- The services route counter emits a single spaced text string.
- Protocol brokers remain intentionally local-only. No email, notification, billing, support, or runner mutations are part of verification.

## Validation

Pending final checks and authenticated browser verification. Temporary flow resources will be deleted after verification; existing E2E artifacts will be preserved.
