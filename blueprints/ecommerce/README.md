# E-commerce Blueprint

Complete e-commerce blueprint with shopping carts, orders, payments, and fulfillment.

## What's Included

This blueprint provides a complete setup for e-commerce applications:

### Personas

- **Premium Customer**: High-value customer with frequent purchases and premium membership
- **Regular Customer**: Standard customer with typical shopping patterns
- **VIP Customer**: VIP member with exclusive access and benefits

### Sample Flows

1. **Browse to Checkout Flow**: Complete shopping journey from browsing to order completion
2. **Cart to Payment Flow**: Add items to cart, proceed to checkout, and process payment
3. **Order Fulfillment Flow**: Order processing, shipping, and delivery tracking

### Features

- Product catalog and inventory management
- Shopping cart operations
- Checkout and payment processing
- Order management and tracking
- Fulfillment and shipping
- Customer profiles and order history
- Real-time order updates via WebSocket

## Quick Start

```bash
# Create a new project from this blueprint
mockforge blueprint create my-store --blueprint ecommerce

# Navigate to the project
cd my-store

# Start the mock server
mockforge serve
```

## API Endpoints

### Products
- `GET /api/products` - List products
- `GET /api/products/{id}` - Get product details
- `GET /api/products/search` - Search products

### Cart
- `GET /api/cart` - Get current cart
- `POST /api/cart/items` - Add item to cart
- `PATCH /api/cart/items/{id}` - Update cart item
- `DELETE /api/cart/items/{id}` - Remove from cart
- `POST /api/cart/discount` - Apply discount code

### Checkout
- `POST /api/checkout` - Create checkout session
- `GET /api/checkout/{id}` - Get checkout details

### Orders
- `POST /api/orders` - Create order
- `GET /api/orders` - List orders
- `GET /api/orders/{id}` - Get order details
- `PATCH /api/orders/{id}` - Update order
- `POST /api/orders/{id}/process` - Process order

### Payments
- `POST /api/payments/process` - Process payment
- `GET /api/payments/{id}` - Get payment details

### Shipping
- `POST /api/shipments` - Create shipment
- `GET /api/shipments/{id}/track` - Track shipment

## Reality Configuration

This blueprint uses **high realism** with:
- 40% reality blend for most endpoints
- 50% reality for product catalog
- 60% reality for orders (realistic state transitions)
- 20% reality for payments (safer for testing)

## Chaos Patterns

E-commerce-specific chaos patterns:
- 5% error rate for payment processing
- 3% error rate for inventory checks
- 2% error rate for shipping services

## Persona Usage

Use personas to test different customer scenarios:

```bash
# Test as premium customer
curl -H "X-Persona-Id: premium-customer" http://localhost:3000/api/cart

# Test as regular customer
curl -H "X-Persona-Id: regular-customer" http://localhost:3000/api/cart

# Test as VIP customer
curl -H "X-Persona-Id: vip-customer" http://localhost:3000/api/cart
```

## WebSocket Support

Real-time order updates are available via WebSocket:

```javascript
const ws = new WebSocket('ws://localhost:3001');
ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log('Order update:', update);
};
```

## Next Steps

1. Review and customize `mockforge.yaml` configuration
2. Add your OpenAPI specification to `openapi.yaml`
3. Customize personas in the `personas/` directory
4. Add more flows in the `flows/` directory
5. Test endpoints using the playground collection

## Documentation

For more information, visit: https://docs.mockforge.dev
