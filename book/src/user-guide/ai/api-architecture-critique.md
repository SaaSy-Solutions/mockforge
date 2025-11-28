# API Architecture Critique

**Pillars:** [AI]

API Architecture Critique feeds entire API schemas into an LLM and produces comprehensive analysis including anti-pattern detection, redundancy detection, poor naming, emotional tone assessment, and recommended restructuring. This positions MockForge as an **API Architect AI**.

## Overview

Beyond structural validation, API Architecture Critique provides:

- **Anti-Pattern Detection**: REST violations, inconsistent naming, poor resource modeling
- **Redundancy Detection**: Duplicate endpoints, overlapping functionality
- **Naming Quality Assessment**: Inconsistent conventions, unclear names, abbreviations
- **Emotional Tone Analysis**: Error messages that are too vague, technical, or unfriendly
- **Restructuring Recommendations**: Better resource hierarchy, consolidation opportunities

## Usage

### CLI Commands

```bash
# Critique API schema
mockforge ai critique --spec openapi.json

# Critique with focus areas
mockforge ai critique --spec openapi.json --focus anti-patterns,naming

# Critique specific endpoint
mockforge ai critique --spec openapi.json --endpoint /api/users/{id}
```

### API Usage

```bash
# Critique API
POST /api/v1/ai-studio/critique
{
  "schema": {...},
  "schema_type": "openapi",
  "focus_areas": ["anti-patterns", "naming", "tone"]
}
```

### UI Usage

Access via AI Studio page:
1. Navigate to AI Studio
2. Select "API Critique"
3. Upload OpenAPI/GraphQL/Protobuf schema
4. Select focus areas
5. Review critique results

## Critique Results

### Example Critique

```json
{
  "anti_patterns": [
    {
      "pattern_type": "rest_violation",
      "severity": "high",
      "location": "/api/users/{id}/delete",
      "description": "DELETE endpoint should use DELETE method, not POST",
      "suggestion": "Change to DELETE /api/users/{id}",
      "example": "POST /api/users/{id}/delete → DELETE /api/users/{id}"
    }
  ],
  "redundancies": [
    {
      "redundancy_type": "duplicate_endpoint",
      "severity": "medium",
      "affected_items": [
        "/api/users/list",
        "/api/users"
      ],
      "description": "Both endpoints return user lists",
      "suggestion": "Consolidate to /api/users"
    }
  ],
  "naming_issues": [
    {
      "issue_type": "inconsistent_convention",
      "severity": "low",
      "location": "user_id",
      "current_name": "user_id",
      "description": "Inconsistent: some fields use 'id', others use 'Id'",
      "suggestion": "Standardize to 'id' or 'Id'"
    }
  ],
  "tone_analysis": {
    "overall_tone": "technical",
    "error_message_issues": [
      {
        "issue_type": "too_vague",
        "severity": "medium",
        "location": "400 error",
        "current_text": "Bad request",
        "description": "Error message is too vague",
        "suggestion": "Provide specific error details: 'Invalid user ID format'"
      }
    ],
    "recommendations": [
      "Make error messages more user-friendly",
      "Provide actionable error details"
    ]
  },
  "restructuring": {
    "hierarchy_improvements": [
      {
        "current": "/api/users/{id}/orders/{order_id}",
        "suggested": "/api/orders/{order_id}?user_id={id}",
        "rationale": "Orders are top-level resources, not nested under users",
        "impact": "medium"
      }
    ],
    "consolidation_opportunities": [
      {
        "items": ["/api/users/list", "/api/users"],
        "description": "Duplicate user listing endpoints",
        "suggestion": "Use single /api/users endpoint with query parameters",
        "benefits": ["Simpler API", "Less maintenance"]
      }
    ]
  },
  "overall_score": 72.5,
  "summary": "API has good structure but needs improvements in REST compliance and error messaging."
}
```

## Focus Areas

### Anti-Patterns

Detects REST violations and design issues:

- **REST Violations**: Wrong HTTP methods, non-RESTful patterns
- **Inconsistent Naming**: Mixed naming conventions
- **Poor Resource Modeling**: Incorrect resource hierarchy

### Redundancy

Detects duplicate or overlapping functionality:

- **Duplicate Endpoints**: Multiple endpoints doing the same thing
- **Overlapping Functionality**: Endpoints with significant overlap

### Naming Quality

Assesses naming consistency and clarity:

- **Inconsistent Conventions**: Mixed naming styles
- **Unclear Names**: Ambiguous or confusing names
- **Abbreviations**: Overuse of abbreviations

### Emotional Tone

Analyzes user-facing text quality:

- **Error Messages**: Too vague, technical, or unfriendly
- **Descriptions**: Unclear or unhelpful
- **User-Facing Text**: Tone and clarity issues

### Restructuring

Recommends structural improvements:

- **Hierarchy Improvements**: Better resource organization
- **Consolidation Opportunities**: Endpoints that can be merged
- **Resource Modeling**: Better resource design

## Real-World Examples

### Example 1: REST Violation

**Anti-Pattern Detected:**
```
POST /api/users/{id}/delete
```

**Issue:** DELETE operation should use DELETE method.

**Suggestion:**
```
DELETE /api/users/{id}
```

### Example 2: Redundancy

**Redundancy Detected:**
```
GET /api/users/list
GET /api/users
```

**Issue:** Both endpoints return user lists.

**Suggestion:** Consolidate to `GET /api/users` with query parameters.

### Example 3: Naming Inconsistency

**Issue Detected:**
- Some fields: `user_id`, `order_id`
- Other fields: `userId`, `orderId`

**Suggestion:** Standardize to one convention (e.g., `user_id`).

### Example 4: Error Message Tone

**Issue Detected:**
```json
{
  "error": "Bad request"
}
```

**Issue:** Too vague, not actionable.

**Suggestion:**
```json
{
  "error": "Invalid user ID format. Expected UUID.",
  "code": "INVALID_USER_ID"
}
```

## Configuration

### Enable API Critique

```yaml
# mockforge.yaml
ai_studio:
  api_critique:
    enabled: true
    default_focus_areas:
      - anti-patterns
      - redundancy
      - naming
      - tone
      - restructuring
```

### LLM Configuration

```yaml
ai_studio:
  api_critique:
    enabled: true
    llm:
      provider: openai
      model: gpt-4
      temperature: 0.3
```

## Best Practices

1. **Run Early**: Critique APIs during design phase
2. **Focus Areas**: Select relevant focus areas
3. **Review Recommendations**: AI suggestions are helpful but review manually
4. **Iterate**: Use critique to improve API design
5. **Track Scores**: Monitor overall scores over time

## Related Documentation

- [AI Studio](../../user-guide/llm-studio.md) - AI features overview
- [System Generation](system-generation.md) - NL to system generation
- [Behavioral Simulation](behavioral-simulation.md) - AI behavioral simulation

