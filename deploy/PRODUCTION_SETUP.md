# MockForge Production Setup Guide

Step-by-step guide to deploy the MockForge registry server, admin UI, and supporting infrastructure.

## Prerequisites

- [flyctl](https://fly.io/docs/flyctl/install/) installed and authenticated
- [Vercel CLI](https://vercel.com/docs/cli) installed and authenticated
- [Stripe CLI](https://stripe.com/docs/stripe-cli) installed (for webhook testing)
- Access to an S3-compatible storage service
- A domain with DNS control (e.g., `mockforge.dev`)

## 1. Fly.io Registry Server

### Create app and database

```bash
# Create the registry server app
flyctl apps create mockforge-registry

# Create a managed Postgres database and attach it
flyctl postgres create --name mockforge-db --region iad --vm-size shared-cpu-1x --volume-size 10
flyctl postgres attach mockforge-db --app mockforge-registry
# This sets DATABASE_URL automatically
```

### Set secrets

```bash
# Generate a JWT secret
JWT_SECRET=$(openssl rand -hex 32)

flyctl secrets set -a mockforge-registry \
  JWT_SECRET="$JWT_SECRET" \
  FLYIO_API_TOKEN="$(flyctl tokens create deploy -a mockforge-registry)" \
  FLYIO_ORG_SLUG="your-org-slug" \
  CORS_ALLOWED_ORIGINS="https://app.mockforge.dev" \
  MOCKFORGE_BASE_URL="https://mocks.mockforge.dev"

# S3 storage (for plugin/spec uploads)
flyctl secrets set -a mockforge-registry \
  S3_BUCKET="mockforge-storage" \
  S3_REGION="us-east-1" \
  AWS_ACCESS_KEY_ID="..." \
  AWS_SECRET_ACCESS_KEY="..."

# Stripe (after Step 4)
flyctl secrets set -a mockforge-registry \
  STRIPE_SECRET_KEY="sk_live_..." \
  STRIPE_WEBHOOK_SECRET="whsec_..."

# SMTP (for transactional email)
flyctl secrets set -a mockforge-registry \
  SMTP_HOST="smtp.example.com" \
  SMTP_USERNAME="..." \
  SMTP_PASSWORD="..."

# Redis (optional, for 2FA and rate limiting)
# flyctl redis create --name mockforge-redis --region iad
# flyctl secrets set -a mockforge-registry REDIS_URL="redis://..."
```

### Deploy

```bash
flyctl deploy --config fly.registry.toml --remote-only

# Verify
curl https://mockforge-registry.fly.dev/metrics/health
```

## 2. DNS Setup

Add the following DNS records:

| Record | Type  | Value                            | Purpose          |
|--------|-------|----------------------------------|------------------|
| `api`  | CNAME | `mockforge-registry.fly.dev`     | Registry API     |
| `mocks`| CNAME | `mockforge-registry.fly.dev`     | Multitenant mocks|
| `app`  | CNAME | `cname.vercel-dns.com`           | Admin UI         |

```bash
# Add TLS certificates on Fly.io
flyctl certs add api.mockforge.dev -a mockforge-registry
flyctl certs add mocks.mockforge.dev -a mockforge-registry
```

After DNS propagates, update `CORS_ALLOWED_ORIGINS`:

```bash
flyctl secrets set -a mockforge-registry \
  CORS_ALLOWED_ORIGINS="https://app.mockforge.dev"
```

## 3. Admin UI on Vercel

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

## 4. Stripe Products

### Create products in Stripe Dashboard

1. **MockForge Pro** — $29/month
   - Metadata: `plan=pro`, `max_hosted_mocks=3`
2. **MockForge Team** — $99/month
   - Metadata: `plan=team`, `max_hosted_mocks=-1`

### Configure webhook

```bash
# Create webhook endpoint in Stripe Dashboard or via CLI
stripe webhook_endpoints create \
  --url https://api.mockforge.dev/api/v1/billing/webhook \
  --events checkout.session.completed,customer.subscription.updated,customer.subscription.deleted,invoice.payment_succeeded,invoice.payment_failed
```

Set the webhook secret and price IDs:

```bash
flyctl secrets set -a mockforge-registry \
  STRIPE_WEBHOOK_SECRET="whsec_..." \
  STRIPE_PRO_PRICE_ID="price_..." \
  STRIPE_TEAM_PRICE_ID="price_..."
```

### Test checkout flow

```bash
# Use Stripe test mode first
stripe listen --forward-to https://api.mockforge.dev/api/v1/billing/webhook
```

## 5. CI/CD

### GitHub Actions secrets

Set these in the repository's Settings > Secrets and variables > Actions:

| Secret              | Description                     |
|---------------------|---------------------------------|
| `FLY_API_TOKEN`     | Fly.io deploy token             |
| `DOCKER_USERNAME`   | Docker Hub username             |
| `DOCKER_PASSWORD`   | Docker Hub password/token       |
| `CRATES_IO_TOKEN`   | crates.io API token             |

The `release.yml` workflow automatically deploys to Fly.io on version tags (non-prerelease).

## 6. Verification Checklist

```bash
# API health
curl https://api.mockforge.dev/metrics/health

# Register a user
curl -X POST https://api.mockforge.dev/api/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"test","email":"test@example.com","password":"SecurePass123!"}'

# Login
curl -X POST https://api.mockforge.dev/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"test","password":"SecurePass123!"}'

# CLI login
mockforge login --service-url https://api.mockforge.dev

# Deploy a mock
mockforge cloud deploy --spec examples/openapi-demo.json --name "Test API" --wait

# Verify the deployed mock responds
curl https://mocks.mockforge.dev/mocks/<org-id>/<slug>/pets

# Admin UI
open https://app.mockforge.dev
```
