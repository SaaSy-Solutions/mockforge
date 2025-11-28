# Scenario State Machines 2.0

Scenario State Machines 2.0 provides a visual flow editor for modeling complex workflows and multi-step scenarios. Create state machines with conditional transitions, reusable sub-scenarios, and real-time state tracking.

## Overview

State machines enable you to model complex API behaviors that depend on previous interactions:

- **Visual Flow Editor**: Drag-and-drop interface for creating state machines
- **Conditional Transitions**: If/else logic for state transitions
- **Reusable Sub-Scenarios**: Compose complex workflows from simpler components
- **Real-Time Preview**: See active state and available transitions
- **VBR Integration**: Synchronize state with VBR entities

## Quick Start

### Create a State Machine

1. Navigate to **State Machines** in the Admin UI
2. Click **Create New State Machine**
3. Add states and transitions using the visual editor
4. Configure conditions for transitions
5. Save the state machine

### Basic Example: Order Workflow

```yaml
name: order_workflow
initial_state: pending
states:
  - name: pending
    response:
      status_code: 200
      body: '{"order_id": "{{resource_id}}", "status": "pending"}'
  
  - name: processing
    response:
      status_code: 200
      body: '{"order_id": "{{resource_id}}", "status": "processing"}'
  
  - name: shipped
    response:
      status_code: 200
      body: '{"order_id": "{{resource_id}}", "status": "shipped"}'

transitions:
  - from: pending
    to: processing
    condition: 'method == "PUT" && path == "/api/orders/{id}/process"'
  
  - from: processing
    to: shipped
    condition: 'method == "PUT" && path == "/api/orders/{id}/ship"'
```

## Visual Editor

The visual editor provides a React Flow-based interface for creating state machines:

### Adding States

1. Click **Add State** button
2. Configure state name and response
3. Position state on canvas
4. Connect states with transitions

### Creating Transitions

1. Drag from one state to another
2. Configure transition condition
3. Set transition metadata (optional)

### Editing States

- Double-click a state to edit
- Right-click for context menu
- Drag to reposition

## Conditional Transitions

Transitions can include conditions that determine when they execute:

### Method-Based Conditions

```yaml
transitions:
  - from: pending
    to: processing
    condition: 'method == "POST" && path == "/api/orders/{id}/process"'
```

### Header-Based Conditions

```yaml
transitions:
  - from: pending
    to: processing
    condition: 'header["X-Admin"] == "true"'
```

### Body-Based Conditions

```yaml
transitions:
  - from: pending
    to: processing
    condition: 'body.status == "ready"'
```

### Complex Conditions

```yaml
transitions:
  - from: pending
    to: processing
    condition: '(method == "PUT" || method == "PATCH") && body.amount > 100'
```

## Sub-Scenarios

Create reusable sub-scenarios that can be embedded in larger workflows:

### Define Sub-Scenario

```yaml
name: payment_processing
states:
  - name: initiated
  - name: processing
  - name: completed
  - name: failed

transitions:
  - from: initiated
    to: processing
    condition: 'method == "POST" && path == "/api/payments"'
```

### Use Sub-Scenario

```yaml
name: order_workflow
states:
  - name: pending
  - name: payment
    sub_scenario: payment_processing
  - name: completed

transitions:
  - from: pending
    to: payment
    condition: 'method == "POST" && path == "/api/orders/{id}/pay"'
  
  - from: payment
    to: completed
    condition: 'sub_scenario_state == "completed"'
```

## VBR Integration

Synchronize state machine state with VBR entities:

### Configure VBR Entity

```yaml
vbr:
  entities:
    - name: orders
      state_machine: order_workflow
      state_field: status
```

### State Synchronization

When a state transition occurs, the corresponding VBR entity is updated:

```bash
# Transition order to processing
PUT /api/orders/123/process

# VBR entity automatically updated
GET /vbr-api/orders/123
# Response: {"id": 123, "status": "processing", ...}
```

## API Endpoints

### State Machine CRUD

```http
# Create state machine
POST /__mockforge/state-machines
Content-Type: application/json

{
  "name": "order_workflow",
  "initial_state": "pending",
  "states": [...],
  "transitions": [...]
}

# List state machines
GET /__mockforge/state-machines

# Get state machine
GET /__mockforge/state-machines/{id}

# Update state machine
PUT /__mockforge/state-machines/{id}

# Delete state machine
DELETE /__mockforge/state-machines/{id}
```

### State Instances

```http
# Create state instance
POST /__mockforge/state-machines/{id}/instances
Content-Type: application/json

{
  "resource_id": "order-123",
  "initial_state": "pending"
}

# List instances
GET /__mockforge/state-machines/{id}/instances

# Get instance
GET /__mockforge/state-machines/{id}/instances/{instance_id}

# Transition instance
POST /__mockforge/state-machines/{id}/instances/{instance_id}/transition
Content-Type: application/json

{
  "to_state": "processing",
  "condition_override": null
}
```

### Current State

```http
# Get current state
GET /__mockforge/state-machines/{id}/instances/{instance_id}/state

# Get next possible states
GET /__mockforge/state-machines/{id}/instances/{instance_id}/next-states
```

### Import/Export

```http
# Export state machine
GET /__mockforge/state-machines/{id}/export

# Import state machine
POST /__mockforge/state-machines/import
Content-Type: application/json

{
  "name": "order_workflow",
  "definition": {...}
}
```

## Real-Time Updates

State machines support real-time updates via WebSocket:

### WebSocket Events

```json
{
  "type": "state_machine_transition",
  "state_machine_id": "uuid",
  "instance_id": "uuid",
  "from_state": "pending",
  "to_state": "processing",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

### Subscribe to Updates

```javascript
const ws = new WebSocket('ws://localhost:9080/ws');
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  if (data.type === 'state_machine_transition') {
    console.log('State transition:', data);
  }
};
```

## Undo/Redo

The visual editor supports undo/redo operations:

- **Undo**: `Ctrl+Z` or `Cmd+Z`
- **Redo**: `Ctrl+Shift+Z` or `Cmd+Shift+Z`
- **History**: View edit history in editor

## Use Cases

### Order Processing Workflow

Model a complete order lifecycle:

```yaml
states:
  - pending
  - payment_pending
  - payment_processing
  - payment_completed
  - payment_failed
  - processing
  - shipped
  - delivered
  - cancelled
```

### User Onboarding

Track user onboarding progress:

```yaml
states:
  - signup
  - email_verification
  - profile_setup
  - onboarding_complete
```

### Approval Workflows

Model multi-step approval processes:

```yaml
states:
  - draft
  - submitted
  - review
  - approved
  - rejected
```

## Best Practices

1. **Start Simple**: Begin with basic state machines before adding complexity
2. **Use Sub-Scenarios**: Break complex workflows into reusable components
3. **Test Transitions**: Verify all transitions work as expected
4. **Document Conditions**: Keep transition conditions well-documented
5. **Version Control**: Export and version control state machine definitions

## Troubleshooting

### State Not Transitioning

- Verify transition condition is correct
- Check that request matches condition
- Review server logs for errors

### Sub-Scenario Not Executing

- Ensure sub-scenario is properly defined
- Verify input/output mapping is correct
- Check sub-scenario state transitions

### VBR Sync Issues

- Verify VBR entity configuration
- Check state field name matches
- Review VBR entity state

## Related Documentation

- [VBR Engine](vbr-engine.md) - State persistence
- [Temporal Simulation](temporal-simulation.md) - Time-based state transitions
- [Admin UI](admin-ui.md) - Visual editor usage

