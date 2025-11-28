# Banking Lite Blueprint

Banking blueprint with accounts, transactions, transfers, and statements.

## What's Included

This blueprint provides a complete setup for banking and fintech applications:

### Personas

- **High-Value Customer**: Customer with multiple accounts and high balances
- **Regular Customer**: Standard customer with typical banking activity
- **Business Account**: Business customer with commercial banking needs

### Sample Flows

1. **Account Creation Flow**: Create new account and initial deposit
2. **Transfer Flow**: Transfer money between accounts
3. **Transaction History Flow**: View account transactions and generate statement

### Features

- Account management (checking, savings, business)
- Transaction history and processing
- Money transfers between accounts
- Account statements and reports
- Balance inquiries and account details
- Strict contract compliance with drift budgets

## Quick Start

```bash
# Create a new project from this blueprint
mockforge blueprint create my-bank --blueprint banking-lite

# Navigate to the project
cd my-bank

# Start the mock server
mockforge serve
```

## API Endpoints

### Accounts
- `GET /api/accounts` - List all accounts
- `GET /api/accounts/{id}` - Get account details
- `POST /api/accounts` - Create new account
- `GET /api/accounts/{id}/balance` - Get account balance

### Transactions
- `GET /api/accounts/{id}/transactions` - List account transactions
- `GET /api/transactions/{id}` - Get transaction details
- `POST /api/transactions` - Create transaction

### Transfers
- `POST /api/transfers` - Create transfer
- `GET /api/transfers/{id}` - Get transfer details
- `GET /api/transfers` - List transfers

### Statements
- `POST /api/accounts/{id}/statements` - Generate statement
- `GET /api/statements/{id}` - Get statement

## Reality Configuration

This blueprint uses **high realism** with:
- 50% reality blend for most endpoints
- 60% reality for account data
- 70% reality for transactions (accuracy critical)
- 30% reality for transfers (safer for testing)

## Drift Budget

**STRICT** drift budgets are configured for banking:
- **NO breaking changes** allowed on any endpoint
- Very limited non-breaking changes (0-2 max)
- Balance endpoints have zero tolerance for changes
- Transfer API must remain stable

This ensures financial data accuracy and contract compliance.

## Chaos Patterns

Minimal chaos patterns for banking:
- 1% global error rate
- 2% error rate for transfers
- 1% error rate for transactions

## Persona Usage

Use personas to test different customer scenarios:

```bash
# Test as high-value customer
curl -H "X-Persona-Id: high-value-customer" http://localhost:3000/api/accounts

# Test as regular customer
curl -H "X-Persona-Id: regular-customer" http://localhost:3000/api/accounts

# Test as business account
curl -H "X-Persona-Id: business-account" http://localhost:3000/api/accounts
```

## Security

Enhanced security monitoring is enabled:
- Suspicious activity alerts
- Structured JSON logging
- Strict access controls

## Next Steps

1. Review and customize `mockforge.yaml` configuration
2. Add your OpenAPI specification to `openapi.yaml`
3. Customize personas in the `personas/` directory
4. Add more flows in the `flows/` directory
5. Test endpoints using the playground collection

## Documentation

For more information, visit: https://docs.mockforge.dev
