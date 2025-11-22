# Lifecycles and Time Travel

**Pillars:** [Reality][DevX]

[Reality] - Makes mocks feel like real backends through time-aware state management
[DevX] - Developer experience improvements for testing time-dependent behavior

## Overview

Lifecycle presets and time travel work together to create realistic, time-aware mock environments. Lifecycles define how entities (users, orders, subscriptions) progress through states over time, while time travel allows you to instantly test these transitions without waiting for real time to pass.

## Lifecycle Presets

Lifecycle presets are pre-configured state machines that model common business processes. They define states, transitions, and how state affects endpoint responses.

### Available Presets

#### Subscription Lifecycle

Models subscription management:
- **States**: NEW → ACTIVE → PAST_DUE → CANCELED
- **Transitions**: Time-based and event-based
- **Affects**: Billing, payment, subscription status endpoints

```yaml
lifecycle:
  presets:
    - name: "subscription"
      type: "subscription"
      states:
        - name: "new"
          transitions:
            - to: "active"
              after_days: 0
        - name: "active"
          transitions:
            - to: "past_due"
              after_days: 30
              condition: "payment_failed_count > 0"
            - to: "canceled"
              after_days: 60
              condition: "payment_failed_count > 2"
```

#### Order Fulfillment Lifecycle

Models order processing and shipping:
- **States**: PENDING → PROCESSING → SHIPPED → DELIVERED → COMPLETED
- **Transitions**: Time-based with inventory conditions
- **Affects**: Order status, fulfillment, shipment, delivery endpoints

#### User Engagement Lifecycle

Models user engagement and retention:
- **States**: NEW → ACTIVE → CHURN_RISK → CHURNED
- **Transitions**: Time-based with activity conditions
- **Affects**: User profile, activity feed, engagement, notifications endpoints

```yaml
lifecycle:
  presets:
    - name: "order_fulfillment"
      type: "order_fulfillment"
      states:
        - name: "pending"
          transitions:
            - to: "processing"
              after_days: 0
        - name: "processing"
          transitions:
            - to: "shipped"
              after_days: 1
              condition: "inventory_available == true"
        - name: "shipped"
          transitions:
            - to: "delivered"
              after_days: 3
        - name: "delivered"
          transitions:
            - to: "completed"
              after_days: 7
```

#### Loan Lifecycle

Models loan application and repayment:
- **States**: APPLICATION → APPROVED → ACTIVE → PAST_DUE → DEFAULTED
- **Transitions**: Time-based with payment conditions
- **Affects**: Loan status, payment, credit endpoints

```yaml
lifecycle:
  presets:
    - name: "loan"
      type: "loan"
      states:
        - name: "application"
          transitions:
            - to: "approved"
              after_days: 2
        - name: "approved"
          transitions:
            - to: "active"
              after_days: 0
        - name: "active"
          transitions:
            - to: "past_due"
              after_days: 30
              condition: "payment_missed == true"
            - to: "defaulted"
              after_days: 90
              condition: "payment_missed_count > 2"
```

## How Lifecycle States Affect Endpoints

Each lifecycle preset automatically modifies endpoint responses based on the current state.

### Subscription Lifecycle Effects

#### Billing Endpoints

- **NEW State**: `billing_status: "pending"`, `subscription_status: "trial"`, `payment_method: "none"`
- **ACTIVE State**: `billing_status: "active"`, `subscription_status: "active"`, payment method set
- **PAST_DUE State**: `billing_status: "warning"`, `subscription_status: "at_risk"`, `last_payment_failed: true`
- **CANCELED State**: `billing_status: "canceled"`, `subscription_status: "cancelled"`, cancellation date set

#### Payment Endpoints

- **NEW State**: No payment history, trial period active
- **ACTIVE State**: Regular payment schedule, successful payments
- **PAST_DUE State**: Failed payment attempts, retry logic active
- **CANCELED State**: Payment processing disabled

### Order Fulfillment Lifecycle Effects

#### Order Status Endpoints

- **PENDING State**: `status: "pending"`, no shipping info
- **PROCESSING State**: `status: "processing"`, warehouse assignment
- **SHIPPED State**: `status: "shipped"`, tracking number, estimated delivery
- **DELIVERED State**: `status: "delivered"`, delivery confirmation, delivery date
- **COMPLETED State**: `status: "completed"`, order closed, feedback requested

#### Fulfillment Endpoints

- **PENDING State**: No fulfillment data
- **PROCESSING State**: Warehouse location, picker assigned
- **SHIPPED State**: Carrier info, tracking number, shipping date
- **DELIVERED State**: Delivery confirmation, signature (if required)
- **COMPLETED State**: Final status, no further updates

### Loan Lifecycle Effects

#### Loan Status Endpoints

- **APPLICATION State**: `status: "application"`, under review
- **APPROVED State**: `status: "approved"`, terms available, awaiting acceptance
- **ACTIVE State**: `status: "active"`, repayment schedule active
- **PAST_DUE State**: `status: "past_due"`, late fees applied, collection notices
- **DEFAULTED State**: `status: "defaulted"`, collections active, credit impact

## Lifecycle Preset Library

The Admin UI includes a **Lifecycle Preset Library** component that provides:

### Viewing Presets

- **Preset List**: See all available lifecycle presets with descriptions
- **Preset Details**: Click "View Details" to see:
  - Initial state
  - All state transitions with conditions
  - Affected endpoints
  - Transition rules (after_days, conditions)

### Applying Presets

1. Navigate to the **Lifecycle Preset Library** (available in Config or AI Studio)
2. Select a preset (e.g., "Order Fulfillment", "Subscription", "User Engagement")
3. Click **"Apply"** to assign the preset to your active persona
4. The persona's lifecycle state is immediately set to the preset's initial state

### API Endpoints

- `GET /api/v1/consistency/lifecycle-presets` - List all available presets
- `GET /api/v1/consistency/lifecycle-presets/{preset_name}` - Get detailed preset information
- `POST /api/v1/consistency/lifecycle-presets/apply` - Apply a preset to a persona

## Integrating Lifecycles with Personas

Lifecycles are most powerful when combined with personas:

```yaml
consistency:
  personas:
    premium_user:
      id: "user:premium-001"
      lifecycle:
        preset: "subscription"
        initial_state: "active"
      traits:
        subscription_tier: "premium"
        billing_cycle: "monthly"
```

The persona's lifecycle state automatically influences all related endpoints:
- Billing endpoints reflect the subscription state
- Payment endpoints show appropriate payment history
- User profile endpoints show subscription status

### Applying Presets via UI

1. **Activate a Persona**: Set an active persona in your workspace
2. **Open Lifecycle Preset Library**: Navigate to the preset library component
3. **Select Preset**: Choose the lifecycle preset you want (e.g., "Order Fulfillment")
4. **Apply**: Click "Apply" - the preset is immediately applied to the active persona
5. **Verify**: Check the persona configuration to see the lifecycle state

## Using Time Travel to Test Lifecycle Transitions

Time travel allows you to instantly test lifecycle transitions without waiting for real time to pass.

### Example: Testing Order Fulfillment

```bash
# Start with an order in PENDING state
mockforge time enable --time "2025-01-01T00:00:00Z"

# Create an order (starts in PENDING)
curl POST /api/orders
# Response: { "status": "pending", ... }

# Advance time by 1 day (order moves to PROCESSING)
mockforge time advance 1d
curl GET /api/orders/123
# Response: { "status": "processing", ... }

# Advance time by 1 more day (order moves to SHIPPED)
mockforge time advance 1d
curl GET /api/orders/123
# Response: { "status": "shipped", "tracking_number": "...", ... }

# Advance time by 3 days (order moves to DELIVERED)
mockforge time advance 3d
curl GET /api/orders/123
# Response: { "status": "delivered", "delivered_at": "...", ... }

# Advance time by 7 days (order moves to COMPLETED)
mockforge time advance 7d
curl GET /api/orders/123
# Response: { "status": "completed", ... }
```

### Example: Testing Subscription Lifecycle

```bash
# Start with a new subscription
mockforge time enable --time "2025-01-01T00:00:00Z"

# Create subscription (starts in NEW/trial)
curl POST /api/subscriptions
# Response: { "status": "trial", "trial_ends_at": "2025-01-08", ... }

# Advance time by 8 days (trial ends, moves to ACTIVE)
mockforge time advance 8d
curl GET /api/subscriptions/current
# Response: { "status": "active", "billing_status": "active", ... }

# Simulate payment failure and advance 30 days (moves to PAST_DUE)
# (In real scenario, you'd configure payment failure)
mockforge time advance 30d
curl GET /api/subscriptions/current
# Response: { "status": "at_risk", "billing_status": "warning", ... }
```

## Time Controls in UI

The Admin UI provides a Time Travel widget for easy time manipulation:

### Time Travel Widget

Located in the dashboard, the widget provides:
- **Enable/Disable**: Toggle time travel on/off
- **Current Time Display**: Shows virtual time or real time
- **Quick Advance Buttons**: +1h, +1d, +1 week, +1 month
- **Time Slider**: Drag to jump to any time within ±30 days
- **Date/Time Picker**: Set exact time
- **Time Scale**: Run time faster/slower than real time
- **Lifecycle Update Notifications**: Shows when persona lifecycle states are automatically updated

### Lifecycle State Changes

When time travel is enabled and time advances:
- **Automatic Updates**: Persona lifecycle states are automatically checked and updated
- **Visual Feedback**: The Time Travel widget shows a notification when lifecycle updates occur
- **State Transitions**: Lifecycle states transition based on elapsed time and conditions
- **Response Changes**: API responses immediately reflect the new lifecycle state

**Implementation**: The `useLivePreviewLifecycleUpdates` hook automatically updates persona lifecycles when virtual time changes, providing real-time feedback as you manipulate time.

### Time Travel Page

Full-featured time travel control:
- Detailed time controls
- Time scale adjustment
- Scheduled time changes
- Time-based event visualization

## Example: Order Moving Through States Over Time

Complete example demonstrating order lifecycle with time travel:

```yaml
# config.yaml
lifecycle:
  presets:
    - name: "order_fulfillment"
      type: "order_fulfillment"

consistency:
  personas:
    customer:
      id: "user:customer-001"
      lifecycle:
        preset: "order_fulfillment"
        initial_state: "pending"
```

```bash
# Day 0: Create order
mockforge time set "2025-01-01T10:00:00Z"
curl POST /api/orders -d '{"items": [...]}'
# Response: { "id": "order-123", "status": "pending", "created_at": "2025-01-01T10:00:00Z" }

# Day 0 (same day): Order moves to processing
curl GET /api/orders/order-123
# Response: { "status": "processing", "warehouse": "WH-001", ... }

# Day 1: Order ships
mockforge time advance 1d
curl GET /api/orders/order-123
# Response: { "status": "shipped", "tracking_number": "TRACK-123", "shipped_at": "2025-01-02T10:00:00Z" }

# Day 4: Order delivered
mockforge time advance 3d
curl GET /api/orders/order-123
# Response: { "status": "delivered", "delivered_at": "2025-01-05T14:30:00Z" }

# Day 11: Order completed
mockforge time advance 7d
curl GET /api/orders/order-123
# Response: { "status": "completed", "completed_at": "2025-01-12T00:00:00Z" }
```

## Best Practices

### Lifecycle Design

1. **Start Simple**: Begin with basic states and add complexity as needed
2. **Use Conditions**: Leverage event-based transitions for realistic behavior
3. **Test Transitions**: Use time travel to verify all state transitions work
4. **Document States**: Clearly document what each state means and affects

### Time Travel Usage

1. **Test Edge Cases**: Use time travel to test boundary conditions (e.g., exactly 30 days)
2. **Verify State Changes**: Always verify that state changes occur at expected times
3. **Reset When Needed**: Reset time travel between test scenarios for consistency
4. **Use Time Scale**: Use time scale for long-running simulations (e.g., 1 year = 1 hour)

### Integration with Personas

1. **Match Lifecycles to Personas**: Choose lifecycle presets that match persona use cases
2. **Set Initial States**: Configure appropriate initial states for different persona types
3. **Test Persona Evolution**: Use time travel to see how personas evolve over time
4. **Verify Cross-Endpoint Consistency**: Ensure lifecycle state affects all relevant endpoints

## Common Patterns

### Pattern 1: Subscription Trial to Paid

```yaml
persona:
  lifecycle:
    preset: "subscription"
    initial_state: "new"  # Starts in trial
```

Test flow:
1. Day 0: Create subscription (trial)
2. Day 7: Trial ends, moves to active (paid)
3. Day 37: Payment fails, moves to past_due
4. Day 67: Multiple failures, moves to canceled

### Pattern 2: Order Fulfillment with Inventory

```yaml
persona:
  lifecycle:
    preset: "order_fulfillment"
    initial_state: "pending"
```

Test flow:
1. Day 0: Create order (pending)
2. Day 0: Inventory available, moves to processing
3. Day 1: Order ships
4. Day 4: Order delivered
5. Day 11: Order completed

### Pattern 3: Loan Application to Default

```yaml
persona:
  lifecycle:
    preset: "loan"
    initial_state: "application"
```

Test flow:
1. Day 0: Submit application
2. Day 2: Application approved
3. Day 2: Loan activated
4. Day 32: Payment missed, moves to past_due
5. Day 92: Multiple missed payments, moves to defaulted

## Example Workspace

A complete example workspace demonstrating lifecycle with time travel is available:

**Location**: `examples/lifecycle-time-travel/`

This example includes:
- Complete order fulfillment lifecycle configuration
- Time travel setup
- API endpoints that respond to lifecycle states
- Step-by-step testing scenarios
- Reality trace observability

See `examples/lifecycle-time-travel/README.md` for detailed instructions.

## Related Documentation

- [PERSONAS.md](PERSONAS.md) - Persona configuration and usage
- [REALITY_TRACE.md](REALITY_TRACE.md) - Understanding response generation
- [TIME_TRAVEL.md](TIME_TRAVEL.md) - Detailed time travel documentation
- [REALITY_CONTINUUM.md](REALITY_CONTINUUM.md) - Reality blending features
