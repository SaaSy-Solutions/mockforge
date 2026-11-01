-- Internal organizations exempt from billing.
--
-- An exempt org needs no Stripe subscription and gets no plan limits, no
-- hosted-mock overage cutoff and no metered AI-credit drawdown. Only a site
-- admin (users.is_admin) can set or clear it, through
-- PUT /api/v1/admin/organizations/{org_id}/billing-exempt. Additive: existing
-- orgs default to not exempt.
ALTER TABLE organizations ADD COLUMN IF NOT EXISTS billing_exempt BOOLEAN NOT NULL DEFAULT FALSE;
