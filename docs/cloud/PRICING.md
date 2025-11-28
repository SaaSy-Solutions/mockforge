# MockForge Cloud Pricing

Transparent, predictable pricing for teams of all sizes. Start free, scale as you grow.

## Overview

MockForge Cloud offers three pricing tiers designed to meet the needs of individual developers, small teams, and large organizations. All plans include access to the marketplace, API access, and core mocking features.

---

## Pricing Plans

### Free Plan

**$0/month** - Perfect for getting started

**Included:**
- ✅ **10,000 API requests** per month
- ✅ **1 GB storage** for mock definitions and assets
- ✅ **1 project** to organize your mocks
- ✅ **1 collaborator** (just you)
- ✅ **1 environment** (dev/staging/prod)
- ✅ **1 plugin** publish limit
- ✅ **3 templates** publish limit
- ✅ **1 scenario** publish limit
- ✅ **BYOK (Bring Your Own Key)** for AI features
- ✅ **Basic plugin marketplace** access
- ✅ **Community support** (best effort)

**Limitations:**
- ❌ No hosted mock deployments
- ❌ No included AI tokens (BYOK only)
- ❌ Limited publishing to marketplace

**Best for:**
- Individual developers
- Learning and experimentation
- Small personal projects
- Testing MockForge before committing

---

### Pro Plan

**$19/month** - For professional developers and small teams

**Everything in Free, plus:**
- ✅ **250,000 API requests** per month (25x more)
- ✅ **20 GB storage** (20x more)
- ✅ **10 projects** to organize multiple APIs
- ✅ **5 collaborators** for team collaboration
- ✅ **3 environments** per project
- ✅ **10 plugins** publish limit
- ✅ **50 templates** publish limit
- ✅ **20 scenarios** publish limit
- ✅ **100,000 AI tokens** per month (included)
- ✅ **Hosted mock deployments** with public URLs
- ✅ **Advanced analytics** and usage tracking
- ✅ **Priority support** (48-hour SLA)
- ✅ **Email notifications** for billing and usage

**Best for:**
- Professional developers
- Small teams (2-5 people)
- Production API development
- CI/CD integration
- Client projects

---

### Team Plan

**$79/month** - For growing teams and organizations

**Everything in Pro, plus:**
- ✅ **1,000,000 API requests** per month (100x Free)
- ✅ **100 GB storage** (100x Free)
- ✅ **Unlimited projects**
- ✅ **20 collaborators**
- ✅ **10 environments** per project
- ✅ **Unlimited plugins** publishing
- ✅ **Unlimited templates** publishing
- ✅ **Unlimited scenarios** publishing
- ✅ **1,000,000 AI tokens** per month
- ✅ **SSO (Single Sign-On)** support
- ✅ **Team collaboration features**
  - Role-based access control
  - Team workspaces
  - Shared resources
- ✅ **Dedicated support** (24-hour SLA)
- ✅ **Custom domain** support (coming soon)
- ✅ **SLA guarantee** (99.9% uptime)

**Best for:**
- Growing teams (5-20 people)
- Multiple projects
- Enterprise development
- High-volume usage
- Organizations requiring SSO

---

## Feature Comparison

| Feature | Free | Pro | Team |
|---------|------|-----|------|
| **Monthly Requests** | 10,000 | 250,000 | 1,000,000 |
| **Storage** | 1 GB | 20 GB | 100 GB |
| **Projects** | 1 | 10 | Unlimited |
| **Collaborators** | 1 | 5 | 20 |
| **Environments** | 1 | 3 | 10 |
| **Hosted Mocks** | ❌ | ✅ | ✅ |
| **AI Tokens (Included)** | 0 (BYOK only) | 100,000 | 1,000,000 |
| **Plugin Publishing** | 1 | 10 | Unlimited |
| **Template Publishing** | 3 | 50 | Unlimited |
| **Scenario Publishing** | 1 | 20 | Unlimited |
| **SSO Support** | ❌ | ❌ | ✅ |
| **Support SLA** | Best effort | 48 hours | 24 hours |
| **Advanced Analytics** | ❌ | ✅ | ✅ |
| **Custom Domains** | ❌ | ❌ | ✅ (coming soon) |

---

## Usage and Overage

### How Usage is Calculated

**API Requests:**
- Each HTTP request to your hosted mocks counts as 1 request
- WebSocket connections count as 1 request per connection
- gRPC calls count as 1 request per call
- Health checks and internal monitoring do not count

**Storage:**
- Mock definitions (OpenAPI specs, configs)
- Uploaded files (fixtures, test data)
- Plugin WASM files
- Template and scenario packages
- Deployment artifacts

**AI Tokens:**
- Used for AI-powered mock generation
- AI-driven data generation
- Smart response suggestions
- Token usage is tracked per request

### Overage Policy

**Current Policy:**
- Requests: Soft limit with warnings at 80% and 95%
- Storage: Hard limit - cannot upload beyond limit
- AI Tokens: Hard limit - features disabled when exceeded

**Future Overage Billing (Coming Soon):**
- Additional requests: $0.001 per 1,000 requests
- Additional storage: $0.10 per GB per month
- Additional AI tokens: $0.01 per 1,000 tokens

**Note:** Overage billing is not currently active. We'll provide 30 days notice before enabling overage charges.

---

## Plan Limits Explained

### Projects
Organize your mocks into projects. Each project can have multiple mock deployments and environments.

### Collaborators
Team members who can access your organization's resources. Admins can manage collaborators and assign roles.

### Environments
Separate configurations for dev, staging, and production. Each environment can have different mock configurations.

### Hosted Mocks
Deploy your mock definitions as live, accessible HTTP endpoints in the cloud. Each deployment gets a unique URL.

### AI Tokens
Used for AI-powered features like intelligent mock generation and data synthesis. Free plan requires BYOK (Bring Your Own Key).

### Marketplace Publishing
Publish your plugins, templates, and scenarios to the MockForge Marketplace for others to use (or keep them private to your organization).

---

## BYOK (Bring Your Own Key) - Free Plan

On the Free plan, AI features require you to provide your own API keys for AI providers:

**Supported Providers:**
- OpenAI (GPT-3.5, GPT-4)
- Anthropic (Claude)
- More providers coming soon

**How It Works:**
1. Go to **Settings** → **BYOK Configuration**
2. Enter your API key for your preferred provider
3. AI features will use your key
4. You're responsible for costs incurred with your key

**Benefits:**
- Full control over AI costs
- Use your existing API key
- No additional charges from MockForge

---

## Billing and Payment

### Payment Methods
- Credit card (Visa, Mastercard, American Express)
- Debit card
- Stripe handles all payments securely

### Billing Cycle
- **Monthly billing**: Charged on the same day each month
- **Annual billing**: Coming soon (save 20% with annual plans)

### Invoicing
- Automatic invoices sent via email
- Download invoices from billing dashboard
- Stripe Tax integration for automatic tax calculation

### Refunds
- **14-day money-back guarantee** for Pro and Team plans
- No questions asked refund policy
- Contact support@mockforge.dev for refunds

### Cancellation
- Cancel anytime from billing dashboard
- Access continues until end of billing period
- No cancellation fees
- Data export available before cancellation

---

## Upgrading and Downgrading

### Upgrading
- **Instant activation**: Upgrades take effect immediately
- **Prorated billing**: Pay only for remaining days in billing cycle
- **No downtime**: Seamless transition between plans

### Downgrading
- **End of billing period**: Downgrades take effect at end of current billing cycle
- **Data preservation**: Your data is preserved (within new plan limits)
- **Grace period**: 7 days to upgrade back if you hit limits

### Plan Changes
1. Go to **Billing** → **Plans**
2. Select new plan
3. Confirm changes
4. Billing updated automatically

---

## Enterprise Plans

Need more than Team plan offers? Contact us for custom enterprise solutions:

**Enterprise Features:**
- Custom request limits
- Custom storage limits
- Unlimited collaborators
- Dedicated infrastructure
- Custom SLA (99.99% available)
- On-premise deployment options
- Custom integrations
- Dedicated account manager
- Priority feature requests

**Contact:** enterprise@mockforge.dev

---

## Frequently Asked Questions

### Can I change plans later?

**Yes!** You can upgrade or downgrade at any time. Upgrades take effect immediately with prorated billing. Downgrades take effect at the end of your billing cycle.

### What happens if I exceed my limits?

**Requests:**
- Soft limit: Warnings at 80% and 95%
- Hard limit: Requests are rate-limited (429 responses)
- Upgrade to increase limits

**Storage:**
- Hard limit: Cannot upload beyond limit
- Delete unused files or upgrade plan

**AI Tokens:**
- Hard limit: AI features disabled when exceeded
- Upgrade plan or wait for next billing cycle

### Do unused requests/storage roll over?

**No.** Limits reset each billing cycle. Unused capacity does not roll over to the next month.

### Can I use MockForge Cloud for free forever?

**Yes!** The Free plan has no expiration. Use it as long as you need, as long as you stay within limits.

### What payment methods do you accept?

We accept all major credit and debit cards through Stripe. We're working on adding more payment methods (PayPal, bank transfer for enterprise).

### Is there a free trial for paid plans?

**Yes!** All paid plans include a 14-day free trial. No credit card required for trial. Cancel anytime during trial with no charges.

### Can I get a refund?

**Yes!** We offer a 14-day money-back guarantee for Pro and Team plans. Contact support@mockforge.dev for refunds.

### Do you offer discounts for students/non-profits?

**Yes!** We offer:
- **Student discount**: 50% off Pro plan (valid student email required)
- **Non-profit discount**: 25% off Team plan (501(c)(3) verification required)
- **Open source discount**: Free Team plan for qualifying open source projects

Contact support@mockforge.dev with proof of eligibility.

### What's included in support?

**Free Plan:**
- Community support (GitHub, Discord)
- Documentation and FAQ
- Best effort email support

**Pro Plan:**
- Priority email support (48-hour SLA)
- Technical assistance
- Feature requests prioritized

**Team Plan:**
- Dedicated support channel (24-hour SLA)
- Priority technical assistance
- Feature request prioritization
- Direct access to engineering team

### Can I host MockForge on-premise?

**Yes!** MockForge is open source and can be self-hosted. For enterprise on-premise deployments with support, contact enterprise@mockforge.dev.

### How do I estimate which plan I need?

**Consider:**
1. **Monthly API requests**: Count your expected traffic
2. **Team size**: Number of collaborators needed
3. **Number of projects**: How many APIs you're mocking
4. **Storage needs**: Size of mock definitions and assets
5. **AI usage**: If you need AI features (Pro+)

**Quick Guide:**
- **Free**: < 10K requests/month, 1 person, 1 project
- **Pro**: 10K-250K requests/month, 2-5 people, multiple projects
- **Team**: 250K+ requests/month, 5+ people, many projects

---

## Getting Started

1. **Sign up** for a free account at [app.mockforge.dev](https://app.mockforge.dev)
2. **Start with Free plan** - no credit card required
3. **Upgrade when needed** - seamless transition to paid plans
4. **Cancel anytime** - no long-term commitments

---

## Need Help Choosing?

Not sure which plan is right for you? We're here to help:

- **Email**: support@mockforge.dev
- **Schedule a call**: [book a demo](https://calendly.com/mockforge)
- **Chat**: Join our [Discord community](https://discord.gg/mockforge)

---

## Legal

- [Terms of Service](/terms)
- [Privacy Policy](/privacy)
- [Data Processing Agreement](/dpa)

---

**Last Updated:** January 2025

**Note:** Pricing and features are subject to change. We'll provide 30 days notice for any pricing changes to existing customers.
