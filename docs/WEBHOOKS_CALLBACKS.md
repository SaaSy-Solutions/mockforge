# Webhooks & Callbacks with MockForge

MockForge supports webhook and callback functionality through its request chaining system and chaos orchestration hooks. This enables simulating asynchronous behavior, outbound HTTP calls, and event-driven workflows.

## Overview

MockForge provides multiple mechanisms for triggering outbound calls and simulating webhook behavior:

1. **Request Chaining**: Execute sequential or parallel HTTP requests with dependencies
2. **Chaos Orchestration Hooks**: Trigger HTTP requests as part of chaos scenarios
3. **Post-Request Scripts**: Execute JavaScript code that can make outbound calls
4. **Consistency Rule Actions**: Trigger chains or HTTP requests based on conditions

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Incoming Request                          │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│            Request Handler                                   │
│  - Process main request                                      │
│  - Generate response                                         │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│            Webhook/Callback Trigger                          │
│  - Post-request hooks                                        │
│  - Chain execution                                           │
│  - Script execution                                          │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│            Outbound HTTP Request                             │
│  - POST to webhook URL                                       │
│  - Include request/response data                            │
│  - Fire-and-forget or await response                        │
└─────────────────────────────────────────────────────────────┘
```

## Method 1: Chaos Orchestration Hooks

Use hooks in chaos orchestration scenarios to trigger webhooks:

### Configuration

```yaml
chaos:
  scenarios:
    - name: order_placed_webhook
      steps:
        - name: create_order
          chaos_config:
            enabled: true

          post_hooks:
            - name: notify_webhook
              hook_type: post_step
              actions:
                - type: http_request
                  method: POST
                  url: "https://webhook.example.com/orders/created"
                  body:  # Can be a JSON string or YAML object (auto-converted)
                    event: "order.created"
                    order_id: "{{variables.order_id}}"
                    timestamp: "{{now}}"
```

### Example: Order Placement Webhook

```yaml
chaos:
  scenarios:
    - name: ecommerce_order_flow
      steps:
        - name: create_order
          post_hooks:
            - name: send_order_webhook
              actions:
                - type: http_request
                  method: POST
                  url: "https://api.example.com/webhooks/order-created"
                  body:  # JSON object (auto-serialized) or JSON string
                    event: "order.created"
                    order:
                      id: "{{variables.order_id}}"
                      total: "{{variables.order_total}}"
                      items: "{{variables.order_items}}"
                  condition:
                    type: equals
                    variable: order_status
                    value: "completed"

            - name: send_inventory_webhook
              actions:
                - type: http_request
                  method: POST
                  url: "https://inventory.example.com/webhooks/order-placed"
                  body:
                    action: "reserve_stock"
                    order_id: "{{variables.order_id}}"
                    items: "{{variables.order_items}}"
```

## Method 2: Request Chaining

Use request chains to trigger multiple HTTP requests in sequence or parallel:

### Chain Definition

```yaml
id: order_processing_chain
name: Order Processing with Webhooks
links:
  - request:
      id: create_order
      method: POST
      url: "{{base_url}}/api/orders"
      body:
        items: [...]
    extract:
      order_id: body.id
      order_total: body.total
    storeAs: order_response

  - request:
      id: notify_webhook
      method: POST
      url: "https://webhook.example.com/orders/created"
      body:
        event: "order.created"
        order_id: "{{chain.order_response.order_id}}"
        total: "{{chain.order_response.order_total}}"
        timestamp: "{{now}}"
    dependsOn: [create_order]
```

### Example: Multi-Webhook Chain

```yaml
id: payment_processing_with_webhooks
name: Payment Processing with Multiple Webhooks

links:
  - request:
      id: process_payment
      method: POST
      url: "{{base_url}}/api/payments"
      body:
        amount: 99.99
        currency: "USD"
    extract:
      payment_id: body.id
      status: body.status
    storeAs: payment_response

  # Webhook 1: Payment success notification
  - request:
      id: payment_success_webhook
      method: POST
      url: "https://notifications.example.com/webhooks/payment-success"
      headers:
        Authorization: "Bearer {{env.WEBHOOK_TOKEN}}"
      body:
        event: "payment.succeeded"
        payment_id: "{{chain.payment_response.payment_id}}"
        amount: "{{chain.payment_response.amount}}"
    dependsOn: [process_payment]
    condition: "{{chain.payment_response.status}} == 'succeeded'"

  # Webhook 2: Update inventory (parallel with webhook 1)
  - request:
      id: inventory_update_webhook
      method: POST
      url: "https://inventory.example.com/webhooks/payment-processed"
      body:
        action: "release_reservation"
        payment_id: "{{chain.payment_response.payment_id}}"
    dependsOn: [process_payment]

  # Webhook 3: Send email notification (after webhook 1)
  - request:
      id: email_webhook
      method: POST
      url: "https://email.example.com/webhooks/send"
      body:
        template: "payment_confirmation"
        payment_id: "{{chain.payment_response.payment_id}}"
    dependsOn: [payment_success_webhook]
```

## Method 3: Post-Request Scripts

Execute JavaScript scripts that can make outbound HTTP calls:

### JavaScript Script

```javascript
// In chain link configuration
scripting:
  post_script: |
    const orderId = mockforge.chain_context.get("order_response.id");
    const orderTotal = mockforge.chain_context.get("order_response.total");

    // Make webhook call
    const webhookResponse = await mockforge.http.post(
      "https://webhook.example.com/orders/created",
      {
        event: "order.created",
        order_id: orderId,
        total: orderTotal,
        timestamp: new Date().toISOString()
      }
    );

    // Store webhook response in context
    mockforge.chain_context.set("webhook_response", webhookResponse);

    // Log result
    console.log("Webhook called:", webhookResponse.status);
```

### Chain Configuration with Script

```yaml
links:
  - request:
      id: create_order
      method: POST
      url: "{{base_url}}/api/orders"
      body:
        items: [...]
    extract:
      order_id: body.id
    storeAs: order_response

    scripting:
      post_script: |
        // Trigger webhook asynchronously
        const webhookUrl = "https://webhook.example.com/orders/created";
        const payload = {
          event: "order.created",
          order_id: mockforge.chain_context.get("order_response.id"),
          timestamp: new Date().toISOString()
        };

        // Fire and forget (async)
        mockforge.http.post(webhookUrl, payload)
          .then(response => {
            console.log("Webhook sent:", response.status);
          })
          .catch(error => {
            console.error("Webhook failed:", error);
          });
```

## Method 4: Consistency Rules

Trigger chains or webhooks based on conditions:

```yaml
intelligent_behavior:
  behavior_model:
    consistency_rules:
      - name: trigger_webhook_on_order
        condition: "method == 'POST' AND path == '/api/orders' AND status == 201"
        action:
          type: execute_chain
          chain_id: "order_webhook_chain"

      - name: trigger_payment_webhook
        condition: "method == 'POST' AND path == '/api/payments' AND body.status == 'completed'"
        action:
          type: execute_chain
          chain_id: "payment_webhook_chain"
```

## Real-World Examples

### Example 1: E-commerce Order Webhook

Simulate an e-commerce order flow with multiple webhooks:

```yaml
id: ecommerce_order_webhooks
name: E-commerce Order with Webhooks

links:
  # Step 1: Create order
  - request:
      id: create_order
      method: POST
      url: "http://localhost:3000/api/orders"
      body:
        customer_id: "cust_123"
        items:
          - product_id: "prod_456"
            quantity: 2
            price: 29.99
    extract:
      order_id: body.id
      status: body.status
    storeAs: order

  # Step 2: Send order confirmation webhook (parallel)
  - request:
      id: order_confirmation_webhook
      method: POST
      url: "https://webhooks.example.com/order-created"
      headers:
        X-Webhook-Signature: "{{env.WEBHOOK_SECRET}}"
      body:
        event: "order.created"
        order_id: "{{chain.order.order_id}}"
        customer_id: "{{request.body.customer_id}}"
        total: "{{chain.order.total}}"
        timestamp: "{{now}}"
    dependsOn: [create_order]

  # Step 3: Update inventory webhook (parallel)
  - request:
      id: inventory_webhook
      method: POST
      url: "https://inventory.example.com/webhooks/reserve"
      body:
        action: "reserve"
        order_id: "{{chain.order.order_id}}"
        items: "{{request.body.items}}"
    dependsOn: [create_order]

  # Step 4: Send email notification (sequential, after webhooks)
  - request:
      id: email_webhook
      method: POST
      url: "https://email-service.example.com/webhooks/send"
      body:
        template: "order_confirmation"
        to: "{{chain.customer.email}}"
        order_id: "{{chain.order.order_id}}"
    dependsOn: [order_confirmation_webhook]
```

### Example 2: Payment Gateway Callbacks

Simulate payment gateway callbacks with retry logic:

```yaml
id: payment_with_callbacks
name: Payment Processing with Callbacks

links:
  - request:
      id: initiate_payment
      method: POST
      url: "{{base_url}}/api/payments"
      body:
        amount: 99.99
        currency: "USD"
    extract:
      payment_id: body.id
      status: body.status
    storeAs: payment

  # Callback: Payment gateway success
  - request:
      id: payment_gateway_callback
      method: POST
      url: "{{base_url}}/api/payments/{{chain.payment.payment_id}}/callback"
      body:
        event: "payment.succeeded"
        payment_id: "{{chain.payment.payment_id}}"
        transaction_id: "txn_abc123"
        timestamp: "{{now}}"
      retry:
        max_attempts: 3
        delay_seconds: 5
        exponential_backoff: true
    dependsOn: [initiate_payment]
    condition: "{{chain.payment.status}} == 'processing'"
```

### Example 3: Event-Driven Architecture

Simulate an event-driven system with multiple subscribers:

```yaml
id: event_driven_webhooks
name: Event-Driven Webhooks

links:
  - request:
      id: publish_event
      method: POST
      url: "{{base_url}}/api/events"
      body:
        event_type: "user.registered"
        data:
          user_id: "usr_123"
          email: "user@example.com"
    extract:
      event_id: body.id
    storeAs: event

  # Webhook to analytics service
  - request:
      id: analytics_webhook
      method: POST
      url: "https://analytics.example.com/webhooks/events"
      body:
        event_id: "{{chain.event.event_id}}"
        event_type: "{{request.body.event_type}}"
        data: "{{request.body.data}}"
    dependsOn: [publish_event]

  # Webhook to notification service
  - request:
      id: notification_webhook
      method: POST
      url: "https://notifications.example.com/webhooks/events"
      body:
        event_id: "{{chain.event.event_id}}"
        event_type: "{{request.body.event_type}}"
        data: "{{request.body.data}}"
    dependsOn: [publish_event]

  # Webhook to email service
  - request:
      id: email_webhook
      method: POST
      url: "https://email.example.com/webhooks/events"
      body:
        event_id: "{{chain.event.event_id}}"
        event_type: "{{request.body.event_type}}"
        data: "{{request.body.data}}"
    dependsOn: [publish_event]
```

## Advanced Features

### Conditional Webhooks

Trigger webhooks based on conditions:

```yaml
post_hooks:
  - name: success_webhook
    condition:
      type: equals
      variable: response_status
      value: 200
    actions:
      - type: http_request
        method: POST
        url: "https://webhook.example.com/success"

  - name: error_webhook
    condition:
      type: greater_than_or_equal
      variable: response_status
      value: 400
    actions:
      - type: http_request
        method: POST
        url: "https://webhook.example.com/error"
        body:
          error_code: "{{variables.response_status}}"
          error_message: "{{variables.error_message}}"
```

### Webhook Retry Logic

Configure retry behavior for webhooks:

```yaml
- request:
    id: webhook_with_retry
    method: POST
    url: "https://webhook.example.com/events"
    retry:
      max_attempts: 3
      delay_seconds: 5
      exponential_backoff: true
```

### Webhook Signing

Add authentication/signing to webhooks:

```yaml
- request:
    id: signed_webhook
    method: POST
    url: "https://webhook.example.com/events"
    headers:
      X-Webhook-Signature: "{{hash(env.WEBHOOK_SECRET + request.body)}}"
      X-Webhook-Timestamp: "{{now}}"
    body:
      event: "order.created"
      data: {...}
```

## Testing Webhooks

### Using MockForge to Receive Webhooks

You can use MockForge itself as a webhook receiver:

```yaml
# config.yaml - Run MockForge on port 3001 to receive webhooks
http:
  port: 3001

# Define a route to receive webhooks
routes:
  - path: /webhooks/orders
    method: POST
    response:
      status: 200
      body:
        message: "Webhook received"
```

### Test Webhook Execution

```bash
# Start MockForge with webhook chain
mockforge serve --config config.yaml

# Trigger the chain
curl -X POST http://localhost:3000/api/orders \
  -H "Content-Type: application/json" \
  -d '{"items": [...]}'

# Check webhook was called (if webhook endpoint is another MockForge instance)
curl http://localhost:3001/webhooks/orders/logs
```

## Best Practices

1. **Fire-and-Forget**: Webhooks are typically asynchronous; don't wait for response
2. **Error Handling**: Webhook failures shouldn't fail the main request
3. **Idempotency**: Use unique IDs to prevent duplicate webhook processing
4. **Retry Logic**: Implement exponential backoff for failed webhooks
5. **Authentication**: Sign webhooks with secrets to verify authenticity
6. **Logging**: Log all webhook calls for debugging and auditing

## Programmatic Access (Rust API)

```rust
use mockforge_chaos::advanced_orchestration::{Hook, HookAction, HookType};

let webhook_hook = Hook {
    name: "order_webhook".to_string(),
    hook_type: HookType::PostStep,
    actions: vec![
        HookAction::HttpRequest {
            url: "https://webhook.example.com/orders".to_string(),
            method: "POST".to_string(),
            body: Some(serde_json::json!({
                "event": "order.created",
                "order_id": "ord_123"
            })),
        }
    ],
    condition: None,
};

// Execute hook
let mut context = ExecutionContext::new();
webhook_hook.execute(&mut context).await?;
```

## Conclusion

MockForge provides comprehensive webhook and callback capabilities through request chaining, chaos orchestration hooks, and JavaScript scripting. These features enable realistic simulation of event-driven architectures and asynchronous workflows.
