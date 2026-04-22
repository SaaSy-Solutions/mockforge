# MockForge Production Setup Guide

Step-by-step guide to deploy the MockForge registry server, admin UI, and supporting infrastructure.

**Architecture:** Fly.io (compute + hosted mocks) + Neon (Postgres) + Cloudflare Pages (admin UI) + Cloudflare R2 (storage)

**Estimated monthly cost at launch:** ~$7-9/mo

## Prerequisites

- [flyctl](https://fly.io/docs/flyctl/install/) installed and authenticated (`fly auth login`)
- [neonctl](https://neon.com/docs/reference/neon-cli) installed (`npm install -g neonctl`) and authenticated (`neonctl auth`)
- [Stripe CLI](https://stripe.com/docs/stripe-cli) installed (for webhook testing)
- [Cloudflare account](https://dash.cloudflare.com/) with R2 + Pages enabled, and an API token with permissions `Account > Cloudflare Pages: Edit` and `User > Memberships: Read` (exported as `CLOUDFLARE_API_TOKEN` for `make deploy-ui`). The wrangler CLI itself is not installed locally — `make deploy-ui` invokes it via `pnpm dlx`.
- [Brevo account](https://www.brevo.com/) with sending domain verified (for transactional email)
- DNS control over `mockforge.dev`

## 1. Neon Postgres Database

### Create project

```bash
# Create a Neon project in US East (same region as Fly.io iad)
neonctl projects create --name mockforge --region-id aws-us-east-1

# Get the connection string (use the pooled endpoint for production)
neonctl connection-string --pooled
# Output: postgresql://neondb_owner:...@ep-xxx-pooler.us-east-1.aws.neon.tech/neondb?sslmode=require
```

Save the pooled connection string — you'll need it for the `DATABASE_URL` secret.

### Free tier limits

| Resource          | Limit                          |
|-------------------|--------------------------------|
| Storage           | 0.5 GB per project             |
| Compute           | 100 CU-hours/month             |
| Max compute size  | 2 CU (8 GB RAM)               |
| Auto-suspend      | After 5 min idle (not configurable on free) |
| Projects          | 100                            |

This is enough for launch. Upgrade to Neon Launch (~$10-15/mo, usage-based) when you get paying customers.

### Notes for SQLx

The registry server uses `sqlx` with `runtime-tokio-rustls`, which is compatible with Neon. Key points:

- Connection string **must** include `?sslmode=require`
- Use the **pooled** endpoint (`-pooler` in hostname) and keep `max_connections` low (5-10) since Neon's PgBouncer handles pooling
- Scale-to-zero means the first query after idle has ~500ms cold start; this is fine for an API server that stays active

## 2. Fly.io Registry Server

### Create app

```bash
# Create the registry server app
fly apps create mockforge-registry

# Allocate a shared IPv4 (needed for HTTPS)
fly ips allocate-v4 --shared -a mockforge-registry

# Allocate IPv6 (needed for certificate validation)
fly ips allocate-v6 -a mockforge-registry
```

### Set secrets

```bash
# Generate a JWT secret
JWT_SECRET=$(openssl rand -hex 32)

# Core secrets
fly secrets set -a mockforge-registry \
  DATABASE_URL="postgresql://neondb_owner:PASSWORD@ep-xxx-pooler.us-east-1.aws.neon.tech/neondb?sslmode=require" \
  JWT_SECRET="$JWT_SECRET" \
  FLYIO_API_TOKEN="$(fly tokens create deploy -a mockforge-registry)" \
  FLYIO_ORG_SLUG="your-fly-org-slug" \
  CORS_ALLOWED_ORIGINS="https://app.mockforge.dev" \
  MOCKFORGE_BASE_URL="https://mocks.mockforge.dev"

# Cloudflare R2 storage (for spec/plugin uploads)
# Create an R2 bucket named "mockforge-storage" in Cloudflare Dashboard
# Then create an R2 API Token: R2 → Manage R2 API Tokens → Create API Token
#   - Permission: Object Read & Write
#   - Scope: mockforge-storage bucket only
# The token gives you an Access Key ID and Secret Access Key
fly secrets set -a mockforge-registry \
  S3_BUCKET="mockforge-storage" \
  S3_REGION="auto" \
  S3_ENDPOINT="https://<CLOUDFLARE_ACCOUNT_ID>.r2.cloudflarestorage.com" \
  AWS_ACCESS_KEY_ID="<R2_TOKEN_ACCESS_KEY_ID>" \
  AWS_SECRET_ACCESS_KEY="<R2_TOKEN_SECRET_ACCESS_KEY>"

# Stripe (configure after Step 5)
# Get your secret key from: https://dashboard.stripe.com/apikeys
# Get your webhook secret from: stripe listen --forward-to <url> (outputs whsec_...)
fly secrets set -a mockforge-registry \
  STRIPE_SECRET_KEY="sk_test_..." \
  STRIPE_WEBHOOK_SECRET="whsec_..."

# Brevo (transactional email — free: 300 emails/day)
# Get your SMTP key: Brevo Dashboard → your name (top right) → SMTP & API → SMTP tab → Generate
# Verify your sending domain in Brevo before sending
fly secrets set -a mockforge-registry \
  SMTP_HOST="smtp-relay.brevo.com" \
  SMTP_PORT="587" \
  SMTP_USERNAME="<YOUR_BREVO_LOGIN_EMAIL>" \
  SMTP_PASSWORD="<BREVO_SMTP_KEY>"
```

### Deploy

```bash
fly deploy --config fly.registry.toml

# Verify
curl https://mockforge-registry.fly.dev/metrics/health
```

### Check status

```bash
fly status -a mockforge-registry
fly logs -a mockforge-registry
```

## 3. DNS Setup

Add the following DNS records for `mockforge.dev`:

| Record   | Type  | Value                          | Purpose           |
|----------|-------|--------------------------------|--------------------|
| `api`    | CNAME | `mockforge-registry.fly.dev`   | Registry API       |
| `*.mocks`| CNAME | `mockforge-registry.fly.dev`   | Customer mock subdomains |
| `app`    | CNAME | `mockforge-admin-ui.pages.dev` | Admin UI (Cloudflare Pages) |

> If DNS is hosted on Cloudflare, you can skip the manual `app` CNAME and instead attach `app.mockforge.dev` to the `mockforge-admin-ui` project via the Pages dashboard — Cloudflare will create the record for you. The CNAME above is only needed when DNS lives elsewhere.

### Add TLS certificates on Fly.io

```bash
# API subdomain
fly certs add api.mockforge.dev -a mockforge-registry

# Wildcard for customer mocks (quote to prevent shell glob expansion)
fly certs add "*.mocks.mockforge.dev" -a mockforge-registry
```

For the wildcard cert, Fly.io uses the DNS-01 challenge. Check the output of `fly certs add` for the `_acme-challenge` CNAME record you need to create:

```bash
# Check cert status and get the ACME challenge CNAME value
fly certs show "*.mocks.mockforge.dev" -a mockforge-registry
```

Add the required DNS record:

| Record                          | Type  | Value                               |
|---------------------------------|-------|--------------------------------------|
| `_acme-challenge.mocks`        | CNAME | `<value from fly certs show output>` |

Wait for DNS propagation, then verify:

```bash
fly certs check "*.mocks.mockforge.dev" -a mockforge-registry
```

## 4. Admin UI on Cloudflare Pages

The admin UI ships as a static SPA built with Vite and deployed to Cloudflare Pages (project: `mockforge-admin-ui`). Deploys are local (no GitHub Actions runner) via the `make deploy-ui` target, which builds the bundle with `pnpm build` and uploads it with `pnpm dlx wrangler@latest pages deploy`.

```bash
# Set the API base URL for the production build
echo "VITE_API_BASE_URL=https://api.mockforge.dev" \
  > crates/mockforge-ui/ui/.env.production

# Export the Cloudflare API token (scoped to Pages:Edit + Memberships:Read)
export CLOUDFLARE_API_TOKEN=...

# Deploy to production (branch=main)
make deploy-ui

# …or deploy the current git branch as a preview URL
make deploy-ui-preview
```

The Makefile pins `CLOUDFLARE_ACCOUNT_ID` in-target, so you only need to supply the token. `pnpm dlx wrangler@latest` fetches wrangler on demand — no global install.

### SPA routing & security headers

`crates/mockforge-ui/ui/public/_redirects` contains `/*  /index.html  200` so React Router paths resolve on refresh, and `public/_headers` sets `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, and `Referrer-Policy: strict-origin-when-cross-origin`. Vite copies both files to `dist/` during `pnpm build` and Pages picks them up automatically.

### Custom domain

Custom-domain attachment is dashboard-only (the wrangler CLI doesn't expose it): **Cloudflare Dashboard → Pages → `mockforge-admin-ui` → Custom domains → Set up a custom domain**, enter `app.mockforge.dev`, and follow the prompts.

If the zone is on Cloudflare DNS, Pages creates the CNAME automatically. Otherwise, add the `app` CNAME from the DNS table above manually before attaching.

### CORS

Cross-origin requests from `app.mockforge.dev` to `api.mockforge.dev` are handled by the registry server's `CORS_ALLOWED_ORIGINS` secret (set in Section 2).

## 5. Stripe Products

### Create products in Stripe Dashboard

1. **MockForge Pro** — $29/month
   - Metadata: `plan=pro`, `max_hosted_mocks=3`
2. **MockForge Team** — $99/month
   - Metadata: `plan=team`, `max_hosted_mocks=-1`

### Configure webhook

```bash
# Create webhook endpoint via Stripe CLI
stripe webhook_endpoints create \
  --url https://api.mockforge.dev/api/v1/billing/webhook \
  --enabled-events checkout.session.completed,customer.subscription.updated,customer.subscription.deleted,invoice.payment_succeeded,invoice.payment_failed
```

Set the webhook secret and price IDs:

```bash
fly secrets set -a mockforge-registry \
  STRIPE_WEBHOOK_SECRET="whsec_..." \
  STRIPE_PRO_PRICE_ID="price_..." \
  STRIPE_TEAM_PRICE_ID="price_..."
```

### Test checkout flow

```bash
# Forward Stripe events to your local or production server
stripe listen --forward-to https://api.mockforge.dev/api/v1/billing/webhook
```

## 6. CI/CD

### GitHub Actions secrets

Set these in Settings > Secrets and variables > Actions:

| Secret            | Description              |
|-------------------|--------------------------|
| `FLY_API_TOKEN`   | Fly.io deploy token      |
| `DOCKER_USERNAME` | Docker Hub username      |
| `DOCKER_PASSWORD` | Docker Hub password/token|
| `CRATES_IO_TOKEN` | crates.io API token      |

The `release.yml` workflow automatically deploys to Fly.io on version tags (non-prerelease).

## 7. Verification Checklist

```bash
# 1. API health
curl https://api.mockforge.dev/metrics/health

# 2. Register a user
curl -X POST https://api.mockforge.dev/api/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"test","email":"test@example.com","password":"SecurePass123!"}'

# 3. Login
curl -X POST https://api.mockforge.dev/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"test@example.com","password":"SecurePass123!"}'

# 4. CLI login
mockforge login --service-url https://api.mockforge.dev

# 5. Deploy a mock
mockforge cloud deploy --spec examples/openapi-demo.json --name "Test API" --wait

# 6. Verify the deployed mock responds
curl https://<slug>.mocks.mockforge.dev/pets

# 7. Admin UI
open https://app.mockforge.dev

# 8. Waitlist signup
curl -X POST https://api.mockforge.dev/api/v1/waitlist/subscribe \
  -H 'Content-Type: application/json' \
  -d '{"email":"beta@example.com","source":"test"}'
```

## Cost Breakdown

### At launch (0-10 customers)

| Component                                     | Monthly Cost |
|-----------------------------------------------|--------------|
| Fly.io registry server (shared-cpu-1x, 512MB) | ~$3.50       |
| Fly.io shared IPv4                            | ~$2.00       |
| Neon Postgres (Free tier)                     | $0.00        |
| Cloudflare Pages (Free tier)                  | $0.00        |
| Cloudflare R2 (under 10 GB storage)           | $0.00        |
| Customer mocks (auto-stopped)                 | ~$1-3        |
| **Total**                                     | **~$7-9/mo** |

### At 50 paying customers

| Component                             | Monthly Cost  |
|---------------------------------------|---------------|
| Fly.io registry (shared-cpu-2x, 1GB)  | ~$7           |
| Fly.io shared IPv4                    | ~$2           |
| Neon Launch (usage-based)             | ~$10-15       |
| Cloudflare Pages (Free tier)          | $0.00         |
| Cloudflare R2 (~50 GB, $0.015/GB)     | ~$0.75        |
| 50 customer mocks (most auto-stopped) | ~$20-30       |
| **Total**                             | **~$40-55/mo** |

Cloudflare Pages Free tier covers unlimited requests and bandwidth for static assets, with a 500 builds/month limit — since deploys are manual and on-demand, this is effectively free forever.

### Scaling notes

- Neon Free → Launch is seamless, no migration needed (same connection string)
- If Neon latency becomes an issue, you can switch to Fly.io Managed Postgres ($38/mo) with zero code changes — just update `DATABASE_URL`
- Customer mock costs stay low because Fly.io auto-stops idle machines (you pay only $0.15/GB/month for stopped machine storage)
