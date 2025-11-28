# MockAI Rule Explanations

Understand how MockAI generates behavioral rules from examples with detailed explanations, confidence scores, and source tracking.

## Overview

Rule explanations provide transparency into MockAI's rule generation process, showing:
- **Why** a rule was generated
- **How confident** the system is in the rule
- **What examples** triggered the rule
- **What patterns** were detected

## Features

- **Detailed reasoning**: Human-readable explanations for each rule
- **Confidence scoring**: 0.0 to 1.0 confidence scores for rule quality
- **Source tracking**: Links to the examples that generated each rule
- **Pattern matching**: Shows detected patterns and match counts
- **Rule categorization**: Rules grouped by type (consistency, validation, etc.)

## Rule Types

### Consistency Rules

Enforce logical behavior patterns across requests:

```json
{
  "rule_id": "consistency_rule_0",
  "rule_type": "consistency",
  "confidence": 0.85,
  "reasoning": "Inferred from 25 examples matching pattern: path starts_with '/api/cart'",
  "source_examples": ["example_1", "example_2", "example_3"]
}
```

### Validation Rules

Define field-level validation requirements:

```json
{
  "rule_id": "validation_rule_email",
  "rule_type": "validation",
  "confidence": 0.92,
  "reasoning": "Email field validation inferred from 18 error examples",
  "pattern_matches": [
    {
      "pattern": "email format validation",
      "match_count": 18,
      "example_ids": ["error_1", "error_2"]
    }
  ]
}
```

### State Transition Rules

Define resource lifecycle state machines:

```json
{
  "rule_id": "state_machine_order",
  "rule_type": "state_transition",
  "confidence": 0.88,
  "reasoning": "State machine for order with 5 states and 8 transitions inferred from CRUD patterns",
  "source_examples": ["crud_1", "crud_2", "crud_3"]
}
```

### Pagination Rules

Define list endpoint pagination behavior:

```json
{
  "rule_id": "pagination_rule_users",
  "rule_type": "pagination",
  "confidence": 0.75,
  "reasoning": "Pagination inferred from 12 list endpoint examples",
  "pattern_matches": [
    {
      "pattern": "page-based pagination",
      "match_count": 12,
      "example_ids": ["list_1", "list_2"]
    }
  ]
}
```

## Usage

### API Endpoints

#### List All Rule Explanations

```bash
GET /__mockforge/api/mockai/rules/explanations?rule_type=consistency&min_confidence=0.7
```

**Response:**
```json
{
  "explanations": [
    {
      "rule_id": "consistency_rule_0",
      "rule_type": "consistency",
      "confidence": 0.85,
      "source_examples": ["example_1", "example_2"],
      "reasoning": "Inferred from 25 examples...",
      "pattern_matches": [
        {
          "pattern": "path starts_with '/api/cart'",
          "match_count": 25,
          "example_ids": ["example_1", "example_2"]
        }
      ],
      "generated_at": "2025-01-15T10:30:00Z"
    }
  ],
  "total": 1
}
```

#### Get Specific Rule Explanation

```bash
GET /__mockforge/api/mockai/rules/{rule_id}/explanation
```

**Response:**
```json
{
  "explanation": {
    "rule_id": "consistency_rule_0",
    "rule_type": "consistency",
    "confidence": 0.85,
    "source_examples": ["example_1", "example_2"],
    "reasoning": "Inferred from 25 examples matching pattern: path starts_with '/api/cart'",
    "pattern_matches": [
      {
        "pattern": "path starts_with '/api/cart'",
        "match_count": 25,
        "example_ids": ["example_1", "example_2", "..."]
      }
    ],
    "generated_at": "2025-01-15T10:30:00Z"
  }
}
```

### UI

Access rule explanations through the **MockAI Rules** dashboard:

1. Navigate to **MockAI Rules** in the sidebar
2. Browse all generated rules with their explanations
3. Filter by rule type or confidence level
4. Search by rule ID, reasoning, or patterns
5. Click on a rule to view detailed explanation

### Programmatic Access

```typescript
// List all explanations
const response = await fetch('/__mockforge/api/mockai/rules/explanations');
const { explanations } = await response.json();

// Filter by type and confidence
const filtered = await fetch(
  '/__mockforge/api/mockai/rules/explanations?rule_type=consistency&min_confidence=0.8'
);

// Get specific explanation
const explanation = await fetch(
  '/__mockforge/api/mockai/rules/consistency_rule_0/explanation'
);
```

## Understanding Confidence Scores

Confidence scores indicate how reliable a generated rule is:

### High Confidence (0.8 - 1.0)

- Many examples support the rule
- Consistent patterns across examples
- Clear, unambiguous inference
- **Action**: Use with confidence, may need minor adjustments

### Medium Confidence (0.6 - 0.8)

- Some examples support the rule
- Mostly consistent patterns
- Some ambiguity in inference
- **Action**: Review and validate, may need refinement

### Low Confidence (0.0 - 0.6)

- Few examples support the rule
- Inconsistent or ambiguous patterns
- Uncertain inference
- **Action**: Manual review required, likely needs significant refinement

## Pattern Matching

Pattern matches show what patterns were detected and how many examples matched:

```json
{
  "pattern_matches": [
    {
      "pattern": "POST /api/users → 201 Created",
      "match_count": 15,
      "example_ids": ["create_1", "create_2", "..."]
    },
    {
      "pattern": "GET /api/users/{id} → 200 OK",
      "match_count": 23,
      "example_ids": ["read_1", "read_2", "..."]
    }
  ]
}
```

Patterns help you understand:
- **What** behavior was detected
- **How often** it occurred
- **Which examples** contributed

## Source Examples

Source examples link rules back to the original examples that generated them:

```json
{
  "source_examples": [
    "example_1",
    "example_2",
    "example_3"
  ]
}
```

Use source examples to:
- **Trace** rule generation back to original data
- **Validate** rule correctness
- **Debug** unexpected rule behavior
- **Improve** examples to generate better rules

## Best Practices

1. **Review explanations after learning**: Check explanations to understand generated rules
2. **Filter by confidence**: Focus on high-confidence rules first
3. **Check source examples**: Validate rules against original examples
4. **Use pattern matches**: Understand what patterns were detected
5. **Iterate on examples**: Improve examples based on explanation feedback

## Troubleshooting

### No explanations available

- Ensure rules have been generated using `generate_rules_with_explanations`
- Check that explanations are stored in the rule explanations storage
- Verify API endpoint is accessible

### Low confidence scores

- Provide more examples for the patterns you want to learn
- Ensure examples are consistent
- Check for typos or inconsistencies in examples

### Missing source examples

- Verify examples were provided during rule generation
- Check that example IDs are correctly tracked
- Ensure examples haven't been deleted

## Integration with Rule Generation

Rule explanations are generated alongside rules and can be stored automatically via the API:

### API Endpoint

```bash
POST /__mockforge/api/mockai/learn
Content-Type: application/json

{
  "examples": [
    {
      "request": {
        "method": "POST",
        "path": "/api/users",
        "body": {
          "name": "Alice",
          "email": "alice@example.com"
        }
      },
      "response": {
        "status_code": 201,
        "body": {
          "id": "user_123",
          "name": "Alice",
          "email": "alice@example.com"
        }
      }
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "rules_generated": {
    "consistency_rules": 2,
    "schemas": 1,
    "state_machines": 0,
    "system_prompt": true
  },
  "explanations": [
    {
      "rule_id": "consistency_rule_0",
      "rule_type": "consistency",
      "confidence": 0.8,
      "reasoning": "Inferred from 3 examples matching pattern: path starts_with '/api/users'"
    }
  ],
  "total_explanations": 3
}
```

### Programmatic Usage

```rust
use mockforge_core::intelligent_behavior::{RuleGenerator, ExamplePair};

let generator = RuleGenerator::new(config);
let examples = vec![/* ... */];

// Generate rules with explanations
let (rules, explanations) = generator
    .generate_rules_with_explanations(examples)
    .await?;

// Store explanations for API access
for explanation in explanations {
    // Store in rule_explanations storage
}
```

### TypeScript/JavaScript Usage

```typescript
const response = await fetch('/__mockforge/api/mockai/learn', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    examples: [
      {
        request: {
          method: 'POST',
          path: '/api/users',
          body: { name: 'Alice', email: 'alice@example.com' }
        },
        response: {
          status_code: 201,
          body: { id: 'user_123', name: 'Alice', email: 'alice@example.com' }
        }
      }
    ]
  })
});

const { success, rules_generated, explanations } = await response.json();
console.log(`Generated ${explanations.length} rule explanations`);
```

## See Also

- [MockAI Usage](./MOCKAI_USAGE.md) - General MockAI features
- [Intelligent Mock Behavior](./INTELLIGENT_MOCK_BEHAVIOR.md) - Behavior system
- [MockAI OpenAPI Generation](./MOCKAI_OPENAPI_GENERATION.md) - OpenAPI generation
