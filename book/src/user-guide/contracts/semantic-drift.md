# Semantic Drift Notifications

**Pillars:** [Contracts]

Semantic Drift Notifications detect when the *meaning* of an API changes, not just its structure. This is where AI Contract Diff goes from "nice" to "indispensable."

## Overview

Structural diffs catch obvious breaking changes. Semantic drift detection catches subtle changes that break consumers even when the structure appears compatible:

- **Description changes** that alter meaning
- **Enum narrowing** (values removed)
- **Soft-breaking changes** hidden behind oneOf/anyOf
- **Nullable → non-nullable** changes
- **Error codes removed**

## How It Works

### Layer 1: Structural Diff

Traditional contract diffing compares:
- Field types
- Required vs optional
- Schema structure
- HTTP methods

### Layer 2: Semantic Analysis

Semantic drift detection adds:
- **LLM-Powered Analysis**: Understands meaning, not just structure
- **Rule-Based Detection**: Fast detection of common patterns
- **Soft-Breaking Scoring**: Quantifies likelihood of breaking consumers
- **Confidence Scoring**: Indicates how certain the detection is

## Detection Types

### Description Changes

Detects when field/endpoint descriptions change meaning:

**Before:**
```json
{
  "status": {
    "type": "string",
    "description": "Order status: pending, processing, shipped"
  }
}
```

**After:**
```json
{
  "status": {
    "type": "string",
    "description": "Order status: pending, processing, shipped, cancelled"
  }
}
```

**Detection:** New value added, but description change may indicate behavior change.

### Enum Narrowing

Detects when enum values are removed:

**Before:**
```json
{
  "status": {
    "type": "string",
    "enum": ["pending", "processing", "shipped", "cancelled"]
  }
}
```

**After:**
```json
{
  "status": {
    "type": "string",
    "enum": ["pending", "processing", "shipped"]
  }
}
```

**Detection:** "cancelled" value removed—breaking change for consumers using it.

### Nullable Changes

Detects nullable → non-nullable changes hidden behind oneOf:

**Before:**
```json
{
  "email": {
    "oneOf": [
      {"type": "string"},
      {"type": "null"}
    ]
  }
}
```

**After:**
```json
{
  "email": {
    "type": "string"
  }
}
```

**Detection:** Field is no longer nullable—breaking for consumers expecting null.

### Error Code Removals

Detects when error codes are removed:

**Before:**
```json
{
  "responses": {
    "400": {...},
    "404": {...},
    "409": {...}
  }
}
```

**After:**
```json
{
  "responses": {
    "400": {...},
    "404": {...}
  }
}
```

**Detection:** 409 error code removed—breaking for consumers handling conflicts.

### Soft-Breaking Changes

Detects changes that may break consumers but aren't structurally breaking:

- **Format changes**: Email format validation tightened
- **Constraint changes**: Min/max values changed
- **Pattern changes**: Regex pattern modified
- **Example changes**: Examples suggest different behavior

## Usage

### CLI Commands

```bash
# Analyze semantic drift
mockforge governance semantic-drift analyze

# Analyze specific endpoint
mockforge governance semantic-drift analyze --endpoint /api/users/{id}

# Get semantic drift incidents
mockforge governance semantic-drift incidents

# Resolve incident
mockforge governance semantic-drift resolve <incident-id>
```

### API Usage

```bash
# Analyze semantic drift
POST /api/v1/semantic-drift/analyze
{
  "before_spec": {...},
  "after_spec": {...}
}

# Get incidents
GET /api/v1/semantic-drift/incidents?workspace_id=workspace-123

# Get incident details
GET /api/v1/semantic-drift/incidents/{id}
```

## Semantic Drift Results

### Example Result

```json
{
  "semantic_confidence": 0.85,
  "soft_breaking_score": 0.7,
  "change_type": "enum_narrowing",
  "semantic_mismatches": [
    {
      "type": "SemanticEnumNarrowing",
      "severity": "high",
      "location": "/api/orders/{id}",
      "field": "status",
      "description": "Enum value 'cancelled' removed",
      "before": ["pending", "processing", "shipped", "cancelled"],
      "after": ["pending", "processing", "shipped"],
      "confidence": 0.9
    }
  ],
  "llm_analysis": {
    "reasoning": "Removing 'cancelled' status breaks consumers that rely on this value for order cancellation flows.",
    "impact": "High - affects order cancellation workflows",
    "recommendation": "Deprecate 'cancelled' first, then remove in next major version"
  }
}
```

### Understanding Scores

- **semantic_confidence**: 0.0-1.0, how certain the semantic analysis is
- **soft_breaking_score**: 0.0-1.0, likelihood this breaks consumers
- **change_type**: Type of semantic change detected
- **confidence**: Individual mismatch confidence

## Configuration

### Enable Semantic Drift Detection

```yaml
# mockforge.yaml
contract_drift:
  semantic_drift:
    enabled: true
    confidence_threshold: 0.65  # Minimum confidence to report
    use_llm_analysis: true  # Enable LLM-powered analysis
    rule_based_detection: true  # Enable rule-based detection
```

### LLM Configuration

```yaml
contract_drift:
  semantic_drift:
    enabled: true
    llm:
      provider: openai  # or anthropic, local
      model: gpt-4
      temperature: 0.3  # Lower = more deterministic
```

## Integration with Drift Budgets

Semantic drift incidents integrate with drift budgets:

```yaml
contract_drift:
  drift_budget:
    max_breaking_changes: 2
    semantic_drift:
      enabled: true
      # Semantic drift incidents count toward budget
```

## Webhooks

Semantic drift detection can trigger webhooks:

```yaml
webhooks:
  - url: https://slack.com/hooks/...
    events:
      - semantic_drift.detected
      - semantic_drift.high_confidence
      - semantic_drift.soft_breaking
```

## Best Practices

1. **Enable LLM Analysis**: More accurate than rule-based alone
2. **Review High Confidence**: Focus on high-confidence detections first
3. **Track Soft-Breaking**: Monitor soft-breaking scores
4. **Document Changes**: Explain why semantic changes were made
5. **Deprecate First**: Use deprecation before removing features

## Real-World Examples

### Example 1: Description Change

**Change:**
- Before: "User email address"
- After: "User email address (must be verified)"

**Detection:** Semantic change—implies new validation requirement.

**Impact:** Consumers may need to handle verification errors.

### Example 2: Enum Narrowing

**Change:**
- Before: `["active", "inactive", "suspended"]`
- After: `["active", "inactive"]`

**Detection:** "suspended" removed—breaking for consumers using it.

**Impact:** High—consumers relying on "suspended" will break.

### Example 3: Soft-Breaking Format Change

**Change:**
- Before: Email format: any string
- After: Email format: must match RFC 5322

**Detection:** Soft-breaking—may break consumers with invalid emails.

**Impact:** Medium—only affects consumers with invalid data.

## Troubleshooting

### False Positives

- **Low Confidence**: Check confidence scores
- **Context Missing**: LLM may need more context
- **Tuning Needed**: Adjust confidence thresholds

### Missed Detections

- **Enable LLM**: Rule-based may miss subtle changes
- **Lower Threshold**: May be filtering valid detections
- **Review Manually**: Some changes need human review

## Related Documentation

- [AI Contract Diff](ai-contract-diff.md) - Contract comparison
- [API Change Forecasting](api-change-forecasting.md) - Predicting changes
- [Drift Budgets](../../docs/DRIFT_BUDGETS.md) - Budget management

