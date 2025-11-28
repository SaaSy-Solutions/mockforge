# B2C SaaS Blueprint

Complete B2C SaaS blueprint with authentication, subscription management, and billing.

## What's Included

This blueprint provides a complete setup for B2C SaaS applications:

### Personas

- **Premium User**: Active premium subscriber with consistent payment history
- **Trial User**: New user exploring the product during trial period
- **Churned User**: Former customer who cancelled, potential win-back candidate

### Sample Flows

1. **Signup Flow**: Complete user registration → trial activation → onboarding
2. **Upgrade Flow**: Trial to paid conversion with payment processing
3. **Billing Cycle Flow**: Monthly billing with invoice generation and payment

### Features

- User authentication and authorization
- Multi-tier subscription plans (trial, free, premium)
- Payment processing and invoicing
- Subscription lifecycle management
- Usage tracking and analytics
- Billing-specific chaos patterns (payment failures, subscription issues)

## Quick Start

```bash
# Create a new project from this blueprint
mockforge blueprint create my-saas-app --blueprint b2c-saas

# Navigate to the project
cd my-saas-app

# Start the mock server
mockforge serve
```

## API Endpoints

### Authentication
- `POST /api/auth/signup` - Create new account
- `POST /api/auth/login` - User login
- `POST /api/auth/logout` - User logout
- `GET /api/users/me` - Get current user

### Subscriptions
- `GET /api/subscriptions/current` - Get current subscription
- `POST /api/subscriptions/trial` - Start trial
- `POST /api/subscriptions/{id}/upgrade` - Upgrade subscription
- `POST /api/subscriptions/{id}/cancel` - Cancel subscription

### Billing
- `GET /api/billing/invoices` - List invoices
- `POST /api/billing/invoices` - Generate invoice
- `GET /api/billing/invoices/{id}` - Get invoice details

### Payments
- `POST /api/payments/methods` - Add payment method
- `POST /api/payments/process` - Process payment
- `GET /api/payments/methods` - List payment methods

## Reality Configuration

This blueprint uses **moderate realism** with:
- 30% reality blend for most endpoints
- 50% reality for billing endpoints
- 20% reality for payment endpoints (safer for testing)

## Chaos Patterns

Billing-specific chaos patterns are configured:
- 5% error rate for billing operations (payment failures)
- 3% error rate for subscription operations
- 1% error rate for authentication

## Persona Usage

Use personas to test different user scenarios:

```bash
# Test as premium user
curl -H "X-Persona-Id: premium-user" http://localhost:3000/api/subscriptions/current

# Test as trial user
curl -H "X-Persona-Id: trial-user" http://localhost:3000/api/subscriptions/current

# Test as churned user
curl -H "X-Persona-Id: churned-user" http://localhost:3000/api/subscriptions/current
```

## Next Steps

1. Review and customize `mockforge.yaml` configuration
2. Add your OpenAPI specification to `openapi.yaml`
3. Customize personas in the `personas/` directory
4. Add more flows in the `flows/` directory
5. Test endpoints using the playground collection

## Documentation

For more information, visit: https://docs.mockforge.dev
