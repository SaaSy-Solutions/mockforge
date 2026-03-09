# MockForge Production Setup Guide

Step-by-step guide to deploy the MockForge registry server, admin UI, and supporting infrastructure.

**Architecture:** Fly.io (compute + hosted mocks) + Neon (Postgres) + Vercel (admin UI)

**Estimated monthly cost at launch:** ~$8-10/mo

## Prerequisites

- [flyctl](https://fly.io/docs/flyctl/install/) installed and authenticated (`fly auth login`)
- [neonctl](https://neon.com/docs/reference/neon-cli) installed (`npm install -g neonctl`) and authenticated (`neonctl auth`)
- [Vercel CLI](https://vercel.com/docs/cli) installed and authenticated
- [Stripe CLI](https://stripe.com/docs/stripe-cli) installed (for webhook testing)
- [Cloudflare account](https://dash.cloudflare.com/) with R2 enabled (for spec/plugin storage)
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

| Record   | Type  | Value                        | Purpose           |
|----------|-------|------------------------------|--------------------|
| `api`    | CNAME | `mockforge-registry.fly.dev` | Registry API       |
| `*.mocks`| CNAME | `mockforge-registry.fly.dev` | Customer mock subdomains |
| `app`    | CNAME | `cname.vercel-dns.com`       | Admin UI           |

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

## 4. Admin UI on Vercel

```bash
cd crates/mockforge-ui/ui

# Set the API base URL for the production build
echo "VITE_API_BASE_URL=https://api.mockforge.dev" > .env.production

# Deploy to Vercel
vercel --prod

# Set custom domain
vercel domains add app.mockforge.dev
```

The `vercel.json` in `crates/mockforge-ui/ui/` proxies `/api/*` requests to `https://api.mockforge.dev`.

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
  --events checkout.session.completed,customer.subscription.updated,customer.subscription.deleted,invoice.payment_succeeded,invoice.payment_failed
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
  -d '{"username":"test","password":"SecurePass123!"}'

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

| Component                          | Monthly Cost |
|------------------------------------|-------------|
| Fly.io registry server (shared-cpu-1x, 512MB) | ~$3.50 |
| Fly.io shared IPv4                 | ~$2.00      |
| Neon Postgres (Free tier)          | $0.00       |
| Vercel (Hobby)                     | $0.00       |
| Customer mocks (auto-stopped)      | ~$1-3       |
| **Total**                          | **~$7-9/mo** |

### At 50 paying customers

| Component                          | Monthly Cost |
|------------------------------------|-------------|
| Fly.io registry (shared-cpu-2x, 1GB) | ~$7        |
| Fly.io shared IPv4                 | ~$2          |
| Neon Launch (usage-based)          | ~$10-15      |
| Vercel (Hobby or Pro)              | $0-20        |
| 50 customer mocks (most auto-stopped) | ~$20-30   |
| **Total**                          | **~$40-74/mo** |

### Scaling notes

- Neon Free → Launch is seamless, no migration needed (same connection string)
- If Neon latency becomes an issue, you can switch to Fly.io Managed Postgres ($38/mo) with zero code changes — just update `DATABASE_URL`
- Customer mock costs stay low because Fly.io auto-stops idle machines (you pay only $0.15/GB/month for stopped machine storage)
