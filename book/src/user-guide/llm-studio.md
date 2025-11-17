# LLM Studio - Natural Language Workspace Creation

LLM Studio enables you to create complete MockForge workspaces from natural language descriptions. Simply describe your system, and MockForge builds a realistic mock backend with personas, behavioral scenarios, reality continuum configuration, and drift budget settings.

## Overview

LLM Studio provides:

- **Natural Language Workspace Creation**: Describe your system in plain English
- **Complete Workspace Generation**: Creates workspaces with endpoints, personas, and scenarios
- **Persona & Relationship Modeling**: Automatically generates personas with relationships
- **Behavioral Scenario Generation**: Creates happy path, failure, and slow path scenarios
- **Reality Continuum Configuration**: Configure mock-to-real blending from natural language
- **Drift Budget Configuration**: Set contract drift tolerance from natural language
- **Voice & Text Input**: Use voice commands or type your descriptions

## Quick Start

### CLI Usage

Create a complete workspace from a natural language description:

```bash
# Create workspace from text command
mockforge voice create-workspace \
  --command "Create an e-commerce workspace with customers, orders, and payments. I need a happy path checkout, a failed payment path, and a slow-shipping scenario."

# Or use interactive voice input
mockforge voice create-workspace
# Describe your workspace when prompted
```

### Web UI Usage

1. Navigate to **Voice** page in Admin UI
2. Enter your workspace description
3. Review the preview
4. Confirm to create the workspace

## Workspace Creation

### Basic Example

Create a simple e-commerce workspace:

```bash
mockforge voice create-workspace \
  --command "Create an e-commerce workspace with customers, orders, and payments"
```

This creates:
- A new workspace with a sanitized ID
- Endpoints for customers, orders, and payments (2-3 per entity)
- 2-3 personas with relationships (e.g., customer owns orders)
- 2-3 behavioral scenarios (happy path, failure, slow path)

### Complete Example

Create a workspace with reality continuum and drift budget:

```bash
mockforge voice create-workspace \
  --command "Create an e-commerce workspace with customers, orders, and payments. I need a happy path checkout, a failed payment path, and a slow-shipping scenario. Make this 80% mock, 20% real prod for catalog only, with strict drift budget."
```

This creates:
- All the basic workspace components
- Reality continuum: 80% mock, 20% real for catalog endpoints
- Drift budget: Strict tolerance (0 breaking changes allowed)

## Natural Language Patterns

### Entity Descriptions

Describe entities and their relationships:

```
Create a workspace with:
- Customers (with profiles and preferences)
- Orders (linked to customers)
- Payments (linked to orders)
- Products (catalog)
```

### Scenario Descriptions

Specify behavioral scenarios:

```
I need:
- A happy path checkout scenario
- A failed payment scenario
- A slow shipping scenario
```

### Reality Continuum

Configure mock-to-real blending:

```
Make this 80% mock, 20% real prod for catalog only
```

```
Use 50% real data for user endpoints, 100% mock for everything else
```

### Drift Budget

Set contract drift tolerance:

```
With strict drift budget
```

```
Allow moderate tolerance for changes
```

```
Lenient drift budget, allow up to 5 breaking changes
```

## Generated Components

### Workspaces

Each workspace includes:

- **Workspace ID**: Auto-generated from name (sanitized)
- **Name & Description**: From your natural language description
- **OpenAPI Specification**: Generated from entity endpoints
- **Personas**: With traits and relationships
- **Scenarios**: Behavioral workflows
- **Configuration**: Reality continuum and drift budget settings

### Endpoints

For each entity, 2-3 endpoints are created:

- **List**: GET endpoint to retrieve all items
- **Get by ID**: GET endpoint to retrieve a specific item
- **Create**: POST endpoint to create new items
- **Update**: PUT/PATCH endpoint to update items (if applicable)
- **Delete**: DELETE endpoint to remove items (if applicable)

### Personas

Personas are automatically generated with:

- **Unique IDs**: Based on persona names
- **Traits**: Extracted from entity context
- **Relationships**: Links between personas (e.g., customer owns orders)
- **Domain**: Inferred from workspace description (e-commerce, finance, healthcare, etc.)

Example personas:
- `premium-customer`: High-value customer with premium traits
- `regular-customer`: Standard customer with typical traits
- `vip-customer`: VIP customer with exclusive traits

### Behavioral Scenarios

Three types of scenarios are generated:

1. **Happy Path**: Successful flows (e.g., successful checkout)
2. **Failure Path**: Error scenarios (e.g., failed payment)
3. **Slow Path**: Latency scenarios (e.g., slow shipping)

Each scenario includes:
- **Ordered Steps**: Sequence of API calls
- **State Variables**: Data extracted between steps
- **Expected Outcomes**: Success/failure conditions
- **Delays**: For slow path scenarios (2 second delays)

## Reality Continuum Configuration

Configure how mock and real data are blended:

### Basic Configuration

```
Make this 80% mock, 20% real
```

Creates:
- Default blend ratio: 0.2 (20% real, 80% mock)
- Enabled: true
- Transition mode: manual

### Route-Specific Configuration

```
Make catalog 50% real, everything else 100% mock
```

Creates:
- Default blend ratio: 0.0 (100% mock)
- Route rule: `/api/catalog/*` with ratio 0.5 (50% real)

### Advanced Configuration

```
80% mock, 20% real prod for catalog only, with time-based transition
```

Creates:
- Default blend ratio: 0.2
- Route rule for catalog endpoints
- Transition mode: time-based

## Drift Budget Configuration

Set acceptable levels of contract changes:

### Strict Budget

```
With strict drift budget
```

Creates:
- Max breaking changes: 0
- Max non-breaking changes: 5
- Enabled: true

### Moderate Budget

```
With moderate tolerance
```

Creates:
- Max breaking changes: 1
- Max non-breaking changes: 10
- Enabled: true

### Lenient Budget

```
Lenient drift budget, allow up to 5 breaking changes
```

Creates:
- Max breaking changes: 5
- Max non-breaking changes: 20
- Enabled: true

## API Endpoints

### Preview Workspace Creation

Parse a workspace creation command and return a preview:

```http
POST /api/v2/voice/create-workspace-preview
Content-Type: application/json

{
  "description": "Create an e-commerce workspace with customers, orders, and payments"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "description": "Create an e-commerce workspace...",
    "parsed": {
      "workspace_name": "e-commerce-workspace",
      "workspace_description": "...",
      "entities": [...],
      "personas": [...],
      "scenarios": [...],
      "reality_continuum": {...},
      "drift_budget": {...}
    }
  }
}
```

### Confirm Workspace Creation

Create the workspace from parsed command:

```http
POST /api/v2/voice/create-workspace-confirm
Content-Type: application/json

{
  "parsed": {
    "workspace_name": "e-commerce-workspace",
    ...
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "workspace_id": "e-commerce-workspace",
    "name": "E-commerce Workspace",
    "creation_log": [
      "Creating workspace: e-commerce-workspace",
      "✓ Workspace 'e-commerce-workspace' created",
      "✓ Generated OpenAPI spec with 12 endpoints",
      "✓ Created 3 personas",
      "✓ Created 3 scenarios",
      "✓ Reality continuum configured",
      "✓ Drift budget configured"
    ],
    "endpoint_count": 12,
    "persona_count": 3,
    "scenario_count": 3,
    "has_reality_continuum": true,
    "has_drift_budget": true
  }
}
```

## Validation & Guardrails

LLM Studio includes validation to ensure quality workspaces:

### Endpoint Validation

- Each entity must have at least 2 endpoints
- Endpoints are validated for proper HTTP methods and paths

### Persona Validation

- At least 2 personas must be created
- Each persona must have at least one relationship

### Scenario Validation

- At least 2 scenarios must be created
- At least one scenario of type: happy_path, failure, or slow_path

### Workspace Name Validation

- Workspace names are sanitized (lowercase, alphanumeric + hyphens)
- If a workspace already exists, alternatives are suggested:
  - `workspace-name-1`, `workspace-name-2`, etc.
  - `workspace-name-20250127` (with timestamp)

## Workflow

### CLI Workflow

1. **Input**: Provide natural language description (voice or text)
2. **Parsing**: LLM parses description into structured data
3. **Preview**: Display workspace preview with all components
4. **Confirmation**: Confirm creation (or skip with `--yes`)
5. **Creation**: Workspace is created with all components
6. **Summary**: Display creation log and workspace details

### API Workflow

1. **Preview Request**: POST to `/api/v2/voice/create-workspace-preview`
2. **Review**: Frontend displays preview to user
3. **Confirmation**: User confirms creation
4. **Create Request**: POST to `/api/v2/voice/create-workspace-confirm`
5. **Response**: Workspace created with detailed creation log

## Examples

### E-commerce Workspace

```bash
mockforge voice create-workspace \
  --command "Create an e-commerce workspace with customers, orders, payments, and products. I need a happy path checkout, a failed payment scenario, and a slow inventory check scenario. Make catalog 50% real, everything else 100% mock. Use strict drift budget."
```

### Banking Workspace

```bash
mockforge voice create-workspace \
  --command "Create a banking workspace with accounts, transactions, and transfers. I need a successful transfer scenario, a failed transfer scenario, and a slow balance check scenario. Make this 100% mock for now with moderate drift tolerance."
```

### Healthcare Workspace

```bash
mockforge voice create-workspace \
  --command "Create a healthcare workspace with patients, appointments, and prescriptions. I need a successful appointment booking, a failed prescription refill, and a slow patient lookup scenario."
```

## Best Practices

1. **Be Specific**: Include entity names, relationships, and scenario types
2. **Describe Scenarios**: Mention happy path, failure, and slow path scenarios
3. **Specify Reality Levels**: Clearly state mock/real ratios and route patterns
4. **Set Drift Budgets**: Specify strictness level for contract changes
5. **Review Preview**: Always review the preview before confirming creation

## Troubleshooting

### Workspace Already Exists

If a workspace with the same name exists, you'll see:

```
Workspace 'e-commerce-workspace' already exists. Suggested alternatives: e-commerce-workspace-1, e-commerce-workspace-2, e-commerce-workspace-20250127
```

Use one of the suggested alternatives or choose a different name.

### Validation Errors

If validation fails, you'll see specific error messages:

```
Entity 'Customer' must have at least 2 endpoints. Found 1.
```

Update your description to include more endpoints or entities.

### Parsing Errors

If the LLM can't parse your description:

```
Failed to parse workspace creation command: ...
```

Try:
- Being more specific about entities and relationships
- Using clearer language
- Breaking complex descriptions into simpler parts

## Next Steps

After creating a workspace:

1. **Start MockForge Server**: Start the server to use the workspace
2. **Access Workspace**: Use `/workspace/{workspace_id}` to access
3. **View Personas**: Check persona relationships in the Admin UI
4. **Test Scenarios**: Execute behavioral scenarios
5. **Adjust Reality**: Modify reality continuum settings as needed
6. **Monitor Drift**: Track contract drift with configured budgets

## See Also

- [Voice + LLM Interface](./voice-llm-interface.md) - Basic voice command interface
- [Smart Personas](./smart-personas.md) - Persona system details
- [Reality Continuum](./reality-continuum.md) - Reality continuum configuration
- [Drift Budgets](../docs/DRIFT_BUDGETS.md) - Drift budget documentation

