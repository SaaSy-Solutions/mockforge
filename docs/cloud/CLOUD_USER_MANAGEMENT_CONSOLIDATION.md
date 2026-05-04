# User Management Consolidation — Design

Decision + plan for consolidating the local `user-management` page into the cloud `organization` page. Tracks task #15 in the cloud-enablement plan.

## Goal

Eliminate the duplicate surface. Today both `user-management` (local) and `organization` (cloud) cover roughly the same functionality with different shapes. Pick one (organization), fold any unique features in, retire the other.

## What exists

- **Local `UserManagementPage`** (`pages/UserManagementPage.tsx`):
  - Users tab: list, role (admin/editor/viewer), status (active/inactive/pending).
  - Teams tab: name, description, member count, quota.
  - Invitations tab: pending/accepted/expired.
  - Analytics tab: active users, new this month, invitation funnel, daily activity chart.
  - Quotas tab: max users/teams/requests/storage with current usage.
- **Cloud `OrganizationPage`** + cloud routes:
  - `/api/v1/organizations/{org_id}/members` — list, add, update role, remove.
  - `/api/v1/organizations/{org_id}/settings/ai` — org-level AI settings.
  - Likely also has billing/usage pages already (separate cloud nav items).

The cloud version is leaner and aligned with the SaaS shape (orgs > teams > users); the local version is fuller but non-canonical.

## What's missing in the cloud organization page

To absorb everything useful from `UserManagementPage`:

1. **Teams (sub-groups within an org).** Cloud only models org members today; teams would be a sub-grouping (e.g., `team_id` on workspace ACLs). Decide whether to import this concept.
2. **Pending invitations UI.** Cloud has email-based invites somewhere (since auth handles new accounts), but the org page may not surface invitation state.
3. **Analytics tab.** User activity charts. Could leverage `pillar_analytics` (already cloud).
4. **Quotas display.** Effectively the existing `usage` cloud page, but framed as an org-overview tab.

## Decision

**Fold the analytics, quotas, and invitations features into `OrganizationPage` as new tabs. Drop the Teams concept** unless customer-driven; orgs + workspace-scoped roles are usually enough. Retire `UserManagementPage` from the sidebar (both modes).

## Specifically

| Local feature | Where it goes in cloud |
|---------------|------------------------|
| Users list + roles | OrganizationPage > Members tab (already exists) |
| Pending invitations | OrganizationPage > Invitations tab (new) |
| Teams | **Drop.** Use workspace-scoped roles instead. |
| Analytics | OrganizationPage > Activity tab (new) — reuses `pillar_analytics` data |
| Quotas | Link to existing `usage` cloud nav item; show summary card on Members tab |

## Routes

Mostly reuse existing endpoints. The few new ones:

```
GET    /api/v1/organizations/{org_id}/invitations
POST   /api/v1/organizations/{org_id}/invitations                # send invite
DELETE /api/v1/organizations/{org_id}/invitations/{id}           # cancel
POST   /api/v1/organizations/{org_id}/invitations/{id}/resend
GET    /api/v1/organizations/{org_id}/activity                   # daily user activity, request counts
```

`activity` is mostly a thin wrapper over `pillar_analytics` — group by user instead of pillar.

## UI changes

1. `AppShell.tsx:217` — **remove `'user-management'`** entirely from `navSections` (not just from the cloudNavItemIds allowlist; remove from the section so local also hides it). The page file can be deleted in a cleanup pass.
2. **OrganizationPage**:
   - Add Invitations tab (pending invites, send/resend/cancel).
   - Add Activity tab (charts from new `/activity` endpoint).
   - Members tab gets a small "Quota usage" summary card linking to `/usage`.
3. **Workspace-scoped roles**: ensure the existing workspace permission UI handles role assignment cleanly, since we're dropping Teams.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Invitations endpoints (CRUD + resend + email send) | ~1.5 days |
| 2 | Activity endpoint (aggregation over pillar_analytics) | ~1 day |
| 3 | OrganizationPage: add Invitations tab | ~1 day |
| 4 | OrganizationPage: add Activity tab | ~1 day |
| 5 | Members tab: quota summary card | ~0.5 day |
| 6 | Remove `user-management` from `navSections` and delete `UserManagementPage.tsx` + tests | ~0.5 day |
| 7 | E2E (invite → accept → see in members → activity tracked) | ~1 day |

Total: ~6.5 working days for v1.

## Decisions

### Drop Teams

**Decision: yes.** Teams add a layer (org > team > user) that most customers don't need. Workspace-scoped roles already give per-resource permissioning; teams would be redundant. If a future Enterprise customer asks for teams (e.g., "Mobile team can only edit mobile workspaces"), revisit — but ship without first.

### Invitations are email-based, not link-share

**Decision: email invites only.** Magic-link invites bypass admin oversight ("who added Bob?"); email forces explicit invitation events that show in the Invitations tab. Existing `email.rs` in registry handles SMTP.

### Activity charts use `pillar_analytics` data

**Decision: reuse, don't duplicate.** Pillar analytics already tracks per-user request counts. The Activity tab is a different presentation of the same source. Avoid building a parallel tracking pipeline.

## Out of scope for v1

- Teams / sub-org grouping.
- Custom roles beyond admin/editor/viewer.
- SSO group sync (existing SSO migration covers some of this; not in this task).
- Per-workspace user limits (org-level only).

## Open questions

1. Some local users in self-hosted mode might rely on `UserManagementPage`. Confirm the local admin server has equivalent endpoints, or accept that self-hosted users go through a config file. Recommend the latter — the cloud-first path is the primary one.
2. Removing `user-management` from `navSections` removes the page from local mode too. Are there enterprise self-hosted customers using this page actively? Worth a one-line note in the changelog so they know to migrate to the org page.
3. Activity tab privacy: do we expose per-user activity to all org members, or only admins? Recommend admins-only; viewers see only their own activity.
