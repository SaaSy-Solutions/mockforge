# Order Fulfillment Lifecycle with Time Travel

This example workspace demonstrates how to use persona lifecycles with time travel to simulate realistic order fulfillment scenarios that evolve over time.

## Overview

This workspace showcases:
- **Order Fulfillment Lifecycle Preset**: Pre-configured lifecycle states (PENDING → PROCESSING → SHIPPED → DELIVERED → COMPLETED)
- **Time Travel Integration**: Virtual time controls that automatically update persona lifecycle states
- **Reality Trace Observability**: See exactly how responses are generated and why
- **Deep-Linking**: Navigate from trace panels to persona/scenario/chaos configurations

## Quick Start

1. **Start MockForge with this configuration:**
   ```bash
   mockforge --config examples/lifecycle-time-travel/config.yaml
   ```

2. **Enable Time Travel:**
   - Navigate to the Time Travel page in the UI
   - Click "Enable Time Travel"
   - Set initial time to a specific date (e.g., 2024-01-01)

3. **Apply Lifecycle Preset to Persona:**
   - Navigate to the Lifecycle Preset Library
   - Select "Order Fulfillment" preset
   - Click "Apply" to assign it to your active persona

4. **Make API Requests:**
   ```bash
   # Get order status (will reflect current lifecycle state)
   curl http://localhost:3000/api/orders/123
   
   # Get shipment tracking (updates based on lifecycle state)
   curl http://localhost:3000/api/orders/123/shipment
   ```

5. **Advance Time:**
   - Use the Time Travel widget to advance time by days/weeks
   - Watch as the persona lifecycle state automatically transitions
   - Observe how API responses change based on the new state

## Lifecycle States and Transitions

### Order Fulfillment Lifecycle

| State | Description | Transitions To | After Days | Condition |
|-------|-------------|----------------|------------|-----------|
| **PENDING** | Order just placed | PROCESSING | 0 | - |
| **PROCESSING** | Order being prepared | SHIPPED | 1 | `inventory_available == true` |
| **SHIPPED** | Order in transit | DELIVERED | 3 | - |
| **DELIVERED** | Order delivered | COMPLETED | 7 | - |
| **COMPLETED** | Order finalized | - | - | Terminal state |

### How States Affect Responses

#### Order Status Endpoint (`GET /api/orders/{id}`)

- **PENDING**: `status: "pending"`, `estimated_fulfillment_date: null`
- **PROCESSING**: `status: "processing"`, `estimated_fulfillment_date: "+2 days"`
- **SHIPPED**: `status: "shipped"`, `tracking_number: "TRACK123"`, `carrier: "FedEx"`
- **DELIVERED**: `status: "delivered"`, `delivered_at: "2024-01-10T14:30:00Z"`
- **COMPLETED**: `status: "completed"`, `completed_at: "2024-01-17T10:00:00Z"`

#### Shipment Tracking Endpoint (`GET /api/orders/{id}/shipment`)

- **PENDING/PROCESSING**: Returns 404 (no shipment yet)
- **SHIPPED**: Returns tracking info with `in_transit: true`
- **DELIVERED**: Returns tracking info with `delivered: true`, `delivery_date`
- **COMPLETED**: Returns final tracking summary

## Time Travel Workflow

### Example: Simulating a 2-Week Order Journey

1. **Day 0 (2024-01-01)**: Order placed
   - Lifecycle state: `PENDING`
   - API response: `status: "pending"`

2. **Day 1 (2024-01-02)**: Advance time by 1 day
   - Lifecycle automatically transitions to: `PROCESSING`
   - API response: `status: "processing"`, `estimated_fulfillment_date: "2024-01-04"`

3. **Day 2 (2024-01-03)**: Advance time by 1 day
   - Lifecycle automatically transitions to: `SHIPPED`
   - API response: `status: "shipped"`, `tracking_number: "TRACK123"`

4. **Day 5 (2024-01-06)**: Advance time by 3 days
   - Lifecycle automatically transitions to: `DELIVERED`
   - API response: `status: "delivered"`, `delivered_at: "2024-01-06T14:30:00Z"`

5. **Day 12 (2024-01-13)**: Advance time by 7 days
   - Lifecycle automatically transitions to: `COMPLETED`
   - API response: `status: "completed"`, `completed_at: "2024-01-13T10:00:00Z"`

## Observability Features

### Reality Trace Panel

Each request includes a Reality Trace Panel showing:
- **Reality Level**: Overall realism of the mock (1-5)
- **Data Source Breakdown**: Percentage from recorded/generator/upstream
- **Active Persona**: Current persona ID (clickable to navigate to persona config)
- **Active Scenario**: Current scenario (clickable to navigate to scenario config)
- **Active Chaos Profiles**: Active chaos rules (clickable to navigate to chaos config)

### Response Generation Trace

Click "Why Did I Get This Response?" to see:
- **Template/Fixture Selection**: Which template was chosen and why
- **Response Selection Mode**: How the response was selected (First, Scenario, Sequential, Random, Weighted)
- **Persona Graph Usage**: Which persona graph nodes were used
- **Rules Executed**: Any hooks or scripts that modified the response
- **Template Expansions**: Step-by-step template expansion
- **Final Payload**: The final resolved payload before sending
- **Schema Validation Diff**: Any differences from the contract schema

## API Endpoints

### Order Management

- `GET /api/orders/{id}` - Get order status (lifecycle-aware)
- `GET /api/orders/{id}/shipment` - Get shipment tracking (lifecycle-aware)
- `POST /api/orders` - Create new order (starts at PENDING state)

### Lifecycle Management

- `GET /api/v1/consistency/lifecycle-presets` - List all available presets
- `GET /api/v1/consistency/lifecycle-presets/{preset_name}` - Get preset details
- `POST /api/v1/consistency/lifecycle-presets/apply` - Apply preset to persona

### Time Travel

- `GET /api/v1/time-travel/status` - Get current time travel status
- `POST /api/v1/time-travel/enable` - Enable time travel
- `POST /api/v1/time-travel/advance` - Advance time by duration
- `POST /api/v1/time-travel/set-time` - Set specific time

## Configuration

See `config.yaml` for the complete workspace configuration including:
- Persona definitions with lifecycle presets
- Reality continuum settings
- Time travel configuration
- Endpoint definitions

## Testing Scenarios

### Scenario 1: Fast Fulfillment (3 days)
1. Create order
2. Advance time by 1 day → PROCESSING
3. Advance time by 1 day → SHIPPED
4. Advance time by 1 day → DELIVERED

### Scenario 2: Standard Fulfillment (11 days)
1. Create order
2. Advance time by 1 day → PROCESSING
3. Advance time by 1 day → SHIPPED
4. Advance time by 3 days → DELIVERED
5. Advance time by 7 days → COMPLETED

### Scenario 3: Delayed Processing
1. Create order
2. Set persona trait `inventory_available: false`
3. Advance time by 1 day → Still PROCESSING (condition not met)
4. Set persona trait `inventory_available: true`
5. Advance time by 1 day → SHIPPED

## UI Features

### Lifecycle Preset Library
- View all available presets with states, transitions, and affected endpoints
- Apply presets to personas with one click
- See detailed preset information including transition rules

### Time Travel Widget
- Enable/disable time travel
- Quick advance buttons (+1h, +1d, +1 week, +1 month)
- Advanced controls: time slider, date/time picker, speed control
- Lifecycle update notifications when time advances

### Request Logs
- View all API requests with "View Trace" button
- Click to see detailed response generation trace
- Deep-link to persona/scenario/chaos configurations

## Next Steps

1. **Customize Lifecycle Presets**: Create your own lifecycle presets for your domain
2. **Add More Endpoints**: Extend the workspace with more lifecycle-aware endpoints
3. **Integrate with Tests**: Use time travel in your integration tests to simulate time-based scenarios
4. **Explore Other Presets**: Try Subscription, Loan, or User Engagement presets

## See Also

- [Reality Trace Documentation](../docs/REALITY_TRACE.md)
- [Lifecycles and Time Documentation](../docs/LIFECYCLES_AND_TIME.md)
- [Personas Documentation](../docs/PERSONAS.md)

