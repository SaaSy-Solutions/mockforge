# Contracts-First Onboarding

**Pillars:** [Contracts]

[Contracts] - Schema, drift, validation, and safety nets

## Start Here If...

You're a **Platform/API team**. You need to ensure API contracts are correct, validated, and stay in sync with real backends. You want to catch breaking changes before they reach production.

Perfect for:
- API teams managing contract evolution
- Platform teams ensuring contract consistency
- Teams needing automatic API sync and change detection
- Organizations requiring contract validation and drift monitoring

## Quick Start: 5 Minutes

Let's set up contract validation and drift detection:

```bash
# Install MockForge
cargo install mockforge-cli

# Create a config with contract validation
cat > mockforge.yaml <<EOF
workspaces:
  - name: api-contracts
    validation:
      mode: enforce  # disabled, warn, or enforce
      openapi_spec: https://api.example.com/openapi.json
    drift:
      enabled: true
      budgets:
        - endpoint: /api/users
          breaking_changes: 0
          non_breaking_changes: 5
    endpoints:
      - path: /api/users
        method: GET
        openapi_operation: getUsers
        response:
          body: |
            {
              "users": []
            }
EOF

# Start the server with validation
mockforge serve --config mockforge.yaml --validate
```

Now test validation:

```bash
# Valid request - should succeed
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{"name": "John", "email": "john@example.com"}'

# Invalid request - should return 422 with validation errors
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{"invalid": "field"}'
```

## Key Contracts Features

### 1. Request/Response Validation

Validate requests and responses against OpenAPI schemas:

```yaml
validation:
  mode: enforce  # disabled, warn, or enforce
  openapi_spec: ./api-spec.yaml
  strict_mode: true
```

**Why it matters:** Catch contract violations early, before they reach production. Get detailed 422 error responses that help developers fix issues quickly.

**Learn more:** [Validation Guide](../../user-guide/http-mocking/openapi.md)

### 2. Contract Drift Detection

Monitor contract changes and detect breaking changes:

```yaml
drift:
  enabled: true
  sync_interval: 3600  # Check every hour
  upstream_spec: https://api.example.com/openapi.json
  budgets:
    - endpoint: /api/users
      breaking_changes: 0  # No breaking changes allowed
      non_breaking_changes: 10  # Max 10 non-breaking changes
```

**Why it matters:** Stay informed about contract changes. Get alerts when drift budgets are exceeded. Prevent breaking changes from reaching consumers.

**Learn more:** [Drift Budgets Guide](../../docs/DRIFT_BUDGETS.md)

### 3. Automatic API Sync

Automatically sync contracts from upstream APIs:

```yaml
sync:
  enabled: true
  sources:
    - url: https://api.example.com/openapi.json
      interval: 3600
      on_change: alert  # alert, update, or ignore
```

**Why it matters:** Keep mocks in sync with real APIs automatically. Get notified when upstream contracts change.

**Learn more:** [API Sync Guide](../../docs/DRIFT_BUDGETS.md#automatic-api-sync)

### 4. AI Contract Diff

Intelligently compare and analyze contract changes:

```yaml
ai_contract_diff:
  enabled: true
  llm_provider: openai
  analysis_depth: detailed
```

**Why it matters:** Understand the impact of contract changes. Get AI-powered recommendations for handling breaking changes.

**Learn more:** [AI Contract Diff Guide](../../docs/DRIFT_BUDGETS.md#ai-contract-diff)

### 5. Multi-Protocol Contracts

Manage contracts across HTTP, gRPC, WebSocket, MQTT, and Kafka:

```yaml
protocols:
  grpc:
    proto_files:
      - ./api.proto
    validation: true
  websocket:
    message_schemas:
      - type: user_message
        schema: ./schemas/user.json
```

**Why it matters:** Ensure contract consistency across all transport layers, not just HTTP/REST.

**Learn more:** [Protocol Contracts Guide](../../docs/PROTOCOL_CONTRACTS.md)

## Next Steps

### Explore Contracts Features

1. **Validation**: [Complete Guide](../../user-guide/http-mocking/openapi.md)
   - Configure validation modes
   - Set up OpenAPI specs
   - Understand validation errors

2. **Drift Budgets**: [Complete Guide](../../docs/DRIFT_BUDGETS.md)
   - Define drift budgets
   - Configure GitOps integration
   - Set up alerts

3. **API Sync**: [Complete Guide](../../docs/DRIFT_BUDGETS.md#automatic-api-sync)
   - Configure automatic sync
   - Set up change detection
   - Handle sync events

4. **Protocol Contracts**: [Complete Guide](../../docs/PROTOCOL_CONTRACTS.md)
   - Manage gRPC contracts
   - Configure WebSocket schemas
   - Set up MQTT/Kafka contracts

5. **AI Contract Diff**: [Complete Guide](../../docs/DRIFT_BUDGETS.md#ai-contract-diff)
   - Enable AI analysis
   - Understand recommendations
   - Handle breaking changes

### Related Pillars

Once you've mastered Contracts, explore these complementary pillars:

- **[Reality]** - Add realistic behavior to your validated mocks
  - [Reality-First Onboarding](reality-first.md)
  - [Reality Continuum Guide](../../docs/REALITY_CONTINUUM.md)

- **[DevX]** - Improve your workflow with SDKs and developer tools
  - [DevX Features](../../user-guide/http-mocking.md)
  - [CLI Reference](../../reference/cli.md)

- **[AI]** - Enhance contract analysis with AI-powered insights
  - [AI-First Onboarding](ai-first.md)
  - [AI Contract Diff Guide](../../docs/DRIFT_BUDGETS.md#ai-contract-diff)

## Examples

### Example 1: Strict Validation

```yaml
workspaces:
  - name: strict-api
    validation:
      mode: enforce
      openapi_spec: ./api-spec.yaml
      strict_mode: true
      validate_responses: true
    endpoints:
      - path: /api/users
        method: POST
        openapi_operation: createUser
        response:
          body: |
            {
              "id": "{{uuid}}",
              "name": "{{faker.name}}",
              "email": "{{faker.email}}"
            }
```

### Example 2: Drift Monitoring

```yaml
drift:
  enabled: true
  sync_interval: 1800  # Check every 30 minutes
  upstream_spec: https://api.example.com/openapi.json
  budgets:
    - endpoint: /api/users
      breaking_changes: 0
      non_breaking_changes: 5
      field_churn_percent: 10
    - endpoint: /api/orders
      breaking_changes: 0
      non_breaking_changes: 10
  gitops:
    enabled: true
    on_violation: create_pr
    branch_prefix: contract-update
```

### Example 3: Multi-Protocol Contracts

```yaml
protocols:
  grpc:
    proto_files:
      - ./api.proto
    validation: true
    drift:
      enabled: true
  websocket:
    message_schemas:
      - type: user_message
        schema: ./schemas/user.json
    validation: true
```

## Troubleshooting

### Validation Not Working

Ensure validation mode is set correctly:

```yaml
validation:
  mode: enforce  # Must be 'enforce' or 'warn', not 'disabled'
  openapi_spec: ./api-spec.yaml  # Must be valid OpenAPI spec
```

### Drift Detection Not Triggering

Check your sync configuration:

```yaml
drift:
  enabled: true
  sync_interval: 3600  # Must be > 0
  upstream_spec: https://api.example.com/openapi.json  # Must be accessible
```

### Contract Sync Failing

Verify your upstream URL and network access:

```bash
# Test upstream accessibility
curl https://api.example.com/openapi.json
```

## Resources

- [Complete Pillars Documentation](../../docs/PILLARS.md)
- [Contracts Features Overview](../../docs/PILLARS.md#contracts--schema-drift-validation-and-safety-nets)
- [API Reference](../../api/rust.md)
- [Examples Repository](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)

---

**Ready to dive deeper?** Continue to the [Drift Budgets Guide](../../docs/DRIFT_BUDGETS.md) or explore [all Contracts features](../../docs/PILLARS.md#contracts--schema-drift-validation-and-safety-nets).

