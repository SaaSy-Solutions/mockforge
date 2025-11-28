# Contract Threat Modeling

**Pillars:** [Contracts]

Contract Threat Modeling is a new category: **contract security posture**. MockForge becomes not only a contract tool, but an **API safety platform**.

## Overview

Beyond structural validation, Contract Threat Modeling analyzes APIs for security risks:

- **PII Exposure**: APIs returning too much personally identifiable information
- **DoS Risk**: Unbounded arrays and missing pagination
- **Error Leakage**: Stack traces and internal details in error responses
- **Schema Design Issues**: Excessive optional fields, inconsistent patterns

## Threat Categories

### PII Exposure

Detects when APIs return sensitive personal information:

**Example:**
```json
{
  "user": {
    "id": "123",
    "email": "user@example.com",
    "ssn": "123-45-6789",  // ⚠️ PII exposure
    "credit_card": "****-****-****-1234"  // ⚠️ PII exposure
  }
}
```

**Detection:** Fields like `ssn`, `credit_card`, `passport_number` are flagged.

**Remediation:** Mask or remove PII from responses.

### DoS Risk (Unbounded Arrays)

Detects arrays without size limits:

**Example:**
```json
{
  "users": {
    "type": "array",
    "items": {...}
    // ⚠️ No maxItems constraint
  }
}
```

**Risk:** Attackers can request unbounded arrays, causing DoS.

**Remediation:** Add `maxItems` constraint:

```json
{
  "users": {
    "type": "array",
    "items": {...},
    "maxItems": 100  // ✅ Bounded
  }
}
```

### Error Leakage

Detects stack traces and internal details in error responses:

**Example:**
```json
{
  "error": {
    "message": "Internal server error",
    "stack_trace": "at com.example.Service.handle()...",  // ⚠️ Leakage
    "internal_id": "uuid-123",  // ⚠️ Internal details
    "database_query": "SELECT * FROM users..."  // ⚠️ SQL leak
  }
}
```

**Risk:** Exposes internal implementation details.

**Remediation:** Sanitize error messages:

```json
{
  "error": {
    "message": "An error occurred",
    "code": "ERROR_CODE"  // ✅ Sanitized
  }
}
```

### Schema Design Issues

Detects problematic schema patterns:

**Excessive Optional Fields:**
```json
{
  "user": {
    "id": "required",
    "name": "optional",  // ⚠️ Too many optional fields
    "email": "optional",
    "phone": "optional",
    "address": "optional",
    // ... 20 more optional fields
  }
}
```

**Risk:** Inconsistent responses, unclear contracts.

**Remediation:** Split into separate schemas or make more fields required.

## Usage

### CLI Commands

```bash
# Analyze contract for threats
mockforge governance threat-model analyze

# Analyze specific service
mockforge governance threat-model analyze --service payments

# Analyze specific endpoint
mockforge governance threat-model analyze --endpoint /api/users/{id}

# Get threat assessments
mockforge governance threat-model assessments

# Get remediation suggestions
mockforge governance threat-model remediations <assessment-id>
```

### API Usage

```bash
# Analyze contract
POST /api/v1/threats/analyze
{
  "spec": {...},
  "workspace_id": "workspace-123"
}

# Get assessments
GET /api/v1/threats/assessments?workspace_id=workspace-123

# Get remediation
GET /api/v1/threats/assessments/{id}/remediations
```

## Threat Assessment Results

### Example Assessment

```json
{
  "workspace_id": "workspace-123",
  "service_name": "payments",
  "endpoint": "/api/payments",
  "threat_level": "high",
  "threat_score": 0.75,
  "threat_categories": ["pii_exposure", "dos_risk"],
  "findings": [
    {
      "finding_type": "PiiExposure",
      "severity": "high",
      "field_path": "body.card_number",
      "description": "Credit card number exposed in response",
      "confidence": 0.9
    },
    {
      "finding_type": "UnboundedArrays",
      "severity": "high",
      "field_path": "body.transactions",
      "description": "Transactions array has no maxItems constraint",
      "confidence": 1.0
    }
  ],
  "remediation_suggestions": [
    {
      "finding_id": "finding_body.card_number",
      "suggestion": "Mask or remove card_number from response",
      "code_example": {
        "before": "\"card_number\": \"1234-5678-9012-3456\"",
        "after": "\"card_number\": \"****-****-****-3456\""
      },
      "confidence": 0.8,
      "priority": "high"
    }
  ]
}
```

### Threat Levels

- **Low**: Minor issues, acceptable risks
- **Medium**: Issues that should be addressed
- **High**: Significant security risks
- **Critical**: Immediate security concerns

### Threat Score

0.0-1.0 score indicating overall threat level:
- **0.0-0.3**: Low risk
- **0.3-0.6**: Medium risk
- **0.6-0.8**: High risk
- **0.8-1.0**: Critical risk

## AI-Powered Remediation

MockForge provides AI-generated remediation suggestions:

### Example Remediation

**Finding:** Unbounded array detected

**Remediation:**
```json
{
  "suggestion": "Add maxItems constraint to array schema",
  "code_example": {
    "type": "array",
    "items": {...},
    "maxItems": 100
  },
  "confidence": 0.9,
  "priority": "high"
}
```

## Configuration

### Enable Threat Modeling

```yaml
# mockforge.yaml
contract_drift:
  threat_modeling:
    enabled: true
    pii_detection: true
    dos_analysis: true
    error_analysis: true
    schema_analysis: true
    ai_remediation: true  # Enable AI-powered suggestions
```

### Thresholds

```yaml
contract_drift:
  threat_modeling:
    enabled: true
    thresholds:
      high_threat_score: 0.7
      critical_threat_score: 0.9
      max_array_size: 1000  # Default maxItems recommendation
```

## Integration with Drift Budgets

Threat assessments can trigger drift budget violations:

```yaml
contract_drift:
  drift_budget:
    max_breaking_changes: 2
    threat_modeling:
      enabled: true
      # High-threat findings count toward budget
```

## Webhooks

Threat assessments can trigger webhooks:

```yaml
webhooks:
  - url: https://slack.com/hooks/...
    events:
      - threat_assessment.completed
      - threat_remediation.suggested
      - threat_critical_detected
```

## Real-World Examples

### Example 1: PII Exposure

**Finding:**
```json
{
  "user": {
    "ssn": "123-45-6789"  // ⚠️ PII
  }
}
```

**Remediation:**
- Remove SSN from response
- Or mask: `"ssn": "***-**-6789"`
- Or return only last 4: `"ssn_last4": "6789"`

### Example 2: DoS Risk

**Finding:**
```json
{
  "products": {
    "type": "array",
    "items": {...}
    // No maxItems
  }
}
```

**Remediation:**
```json
{
  "products": {
    "type": "array",
    "items": {...},
    "maxItems": 100  // ✅ Bounded
  }
}
```

### Example 3: Error Leakage

**Finding:**
```json
{
  "error": {
    "stack_trace": "at com.example..."  // ⚠️ Leakage
  }
}
```

**Remediation:**
```json
{
  "error": {
    "message": "An error occurred",
    "code": "INTERNAL_ERROR"  // ✅ Sanitized
  }
}
```

## Best Practices

1. **Run Regularly**: Assess contracts in CI/CD pipeline
2. **Fix High Priority**: Address high/critical findings immediately
3. **Review AI Suggestions**: AI suggestions are helpful but review manually
4. **Document Exceptions**: Document why certain risks are acceptable
5. **Track Over Time**: Monitor threat scores over time

## Troubleshooting

### False Positives

- **Field Names**: Some field names may trigger false PII detection
- **Context Missing**: AI may need more context
- **Tuning Needed**: Adjust detection thresholds

### Missed Threats

- **Enable All Analyzers**: Ensure all analyzers are enabled
- **Review Manually**: Some threats need human review
- **Update Patterns**: Keep PII patterns updated

## Related Documentation

- [API Change Forecasting](api-change-forecasting.md) - Predicting changes
- [Semantic Drift](semantic-drift.md) - Semantic analysis
- [Drift Budgets](../../docs/DRIFT_BUDGETS.md) - Budget management

