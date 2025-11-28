# E2E Persona Flow - Cross-Protocol Consistency Demo

This scenario demonstrates end-to-end persona flow across multiple protocols (HTTP, WebSocket, webhooks), showcasing how MockForge maintains consistent persona state across all communication channels.

## Overview

This scenario simulates a complete e-commerce user journey:

1. **Login (HTTP)** → User authenticates and persona is activated
2. **Browse (HTTP)** → User browses products using the same persona
3. **Purchase (HTTP)** → User creates an order, linked via persona graph
4. **Notifications (WebSocket)** → Real-time order status updates
5. **Webhooks** → Order status callbacks to external systems

All steps use the **same persona** and **shared state model**, ensuring consistency across protocols.

## Features Demonstrated

### 1. Cross-Protocol State Guarantees

All protocols (HTTP, WebSocket, webhooks) share the same:
- **Persona graph**: User → Orders → Payments → Notifications
- **Lifecycle state**: Subscription and order fulfillment states
- **Entity state**: Orders, payments, and user data

### 2. Shared State Model

```yaml
reality:
  state_model: "ecommerce_v1"
  share_state_across:
    - http
    - websocket
    - webhooks
```

This ensures that:
- A user logged in via HTTP is the same user in WebSocket
- An order created via HTTP triggers WebSocket notifications
- Webhook callbacks reference the same order/user

### 3. Persona Graph Relationships

The persona graph maintains relationships:
- `user:123` → `order:456` (has_orders)
- `order:456` → `payment:789` (has_payment)
- `user:123` → `notification:abc` (receives_notifications)

### 4. Lifecycle-Aware Responses

- **Subscription lifecycle**: Affects billing endpoints
- **Order fulfillment lifecycle**: Affects order status endpoints
- **Time-based transitions**: Orders progress through states over time

## API Endpoints

### Authentication

#### Login
```bash
POST /api/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "password123"
}
```

Response:
```json
{
  "token": "jwt-token-here",
  "user": {
    "id": "user-123",
    "email": "user@example.com",
    "name": "Demo User"
  }
}
```

### Products

#### Browse Products
```bash
GET /api/products?category=electronics
X-MockForge-Workspace: default
```

Response includes products filtered by the active persona's preferences.

### Orders

#### Create Order
```bash
POST /api/orders
Content-Type: application/json
X-MockForge-Workspace: default

{
  "product_id": "prod-123",
  "quantity": 2
}
```

Response:
```json
{
  "id": "ORD-123",
  "user_id": "user-123",
  "product_id": "prod-123",
  "quantity": 2,
  "status": "pending",
  "created_at": "2025-01-27T10:00:00Z"
}
```

#### Get Order Status
```bash
GET /api/orders/ORD-123
X-MockForge-Workspace: default
```

Response includes order status based on lifecycle state and persona graph relationships.

## WebSocket Events

Connect to `ws://localhost:3001` to receive real-time notifications:

### Connection
```javascript
const ws = new WebSocket('ws://localhost:3001');
ws.onopen = () => {
  // Send workspace ID to use shared state
  ws.send(JSON.stringify({
    type: 'init',
    workspace: 'default'
  }));
};
```

### Order Status Updates
```json
{
  "type": "order_status_update",
  "order_id": "ORD-123",
  "status": "shipped",
  "timestamp": "2025-01-27T10:30:00Z"
}
```

### Payment Notifications
```json
{
  "type": "payment_received",
  "order_id": "ORD-123",
  "payment_id": "PAY-456",
  "amount": 99.99,
  "timestamp": "2025-01-27T10:15:00Z"
}
```

## Webhook Callbacks

Webhooks are triggered when order status changes:

### Order Status Webhook
```bash
POST http://your-webhook-url/api/webhooks/order-status
Content-Type: application/json

{
  "event": "order.shipped",
  "order_id": "ORD-123",
  "user_id": "user-123",
  "status": "shipped",
  "timestamp": "2025-01-27T10:30:00Z"
}
```

## Complete Flow Example

### Step 1: Login (HTTP)
```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123"}'
```

This activates the persona for the workspace.

### Step 2: Browse Products (HTTP)
```bash
curl http://localhost:3000/api/products?category=electronics \
  -H "X-MockForge-Workspace: default"
```

Products are filtered based on the active persona's preferences.

### Step 3: Create Order (HTTP)
```bash
curl -X POST http://localhost:3000/api/orders \
  -H "Content-Type: application/json" \
  -H "X-MockForge-Workspace: default" \
  -d '{
    "product_id": "prod-123",
    "quantity": 2
  }'
```

Order is created and linked to the user via persona graph.

### Step 4: Receive WebSocket Notification
```javascript
// WebSocket connection receives:
{
  "type": "order_created",
  "order_id": "ORD-123",
  "status": "pending",
  "timestamp": "2025-01-27T10:00:00Z"
}
```

### Step 5: Order Status Updates (WebSocket)
As the order progresses through lifecycle states, WebSocket notifications are sent:
- `order.processing` (after 0 days)
- `order.shipped` (after 1 day)
- `order.delivered` (after 3 days)
- `order.completed` (after 7 days)

### Step 6: Webhook Callback
External system receives webhook:
```json
{
  "event": "order.shipped",
  "order_id": "ORD-123",
  "user_id": "user-123",
  "tracking_number": "TRACK123456",
  "timestamp": "2025-01-27T11:00:00Z"
}
```

## Time Travel Testing

Use time travel to test lifecycle transitions:

```bash
# Enable time travel
curl -X POST http://localhost:3000/__mockforge/time-travel/enable \
  -H "Content-Type: application/json" \
  -d '{"time": "2025-01-27T00:00:00Z"}'

# Advance time by 1 day to see order status change
curl -X POST http://localhost:3000/__mockforge/time-travel/advance \
  -H "Content-Type: application/json" \
  -d '{"duration": "1d"}'

# Check order status - should now be "shipped"
curl http://localhost:3000/api/orders/ORD-123
```

## Persona Graph Queries

Query related entities via persona graph:

```bash
# Get user with related orders
curl http://localhost:3000/api/v1/consistency/users/user-123?workspace=default

# Get user's orders
curl http://localhost:3000/api/v1/consistency/users/user-123/orders?workspace=default

# Get order with related payment
curl http://localhost:3000/api/v1/consistency/orders/ORD-123?workspace=default
```

## Configuration

See `config.yaml` for full configuration including:
- Cross-protocol state model
- Lifecycle presets
- WebSocket and webhook settings
- Time travel configuration

## Benefits

This scenario demonstrates:

1. **Consistency**: Same persona across all protocols
2. **Coherence**: Related entities are linked via persona graph
3. **Lifecycle Awareness**: Responses reflect current lifecycle state
4. **Time Awareness**: Lifecycle transitions based on virtual time
5. **Real-time Updates**: WebSocket notifications for state changes
6. **Integration**: Webhook callbacks for external systems

All of this works together to create a **measurably real** mock environment where the illusion of a real backend is maintained across all communication channels.
