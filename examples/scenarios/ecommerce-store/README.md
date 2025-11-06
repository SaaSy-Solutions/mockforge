# E-commerce Store Scenario

A complete e-commerce API mock with shopping carts, products, orders, and user management.

## Features

- **Product Catalog**: Browse and search products with categories
- **Shopping Cart**: Add/remove items, update quantities
- **User Management**: Registration, authentication, profiles
- **Order Processing**: Checkout flow, order tracking, status updates
- **Real-time Updates**: WebSocket support for cart and order updates

## API Endpoints

### Products
- `GET /api/products` - List all products
- `GET /api/products/{id}` - Get product details
- `GET /api/products/search?q={query}` - Search products
- `GET /api/categories` - List product categories

### Cart
- `GET /api/cart` - Get current cart
- `POST /api/cart/items` - Add item to cart
- `PUT /api/cart/items/{id}` - Update cart item quantity
- `DELETE /api/cart/items/{id}` - Remove item from cart
- `POST /api/cart/clear` - Clear cart

### Orders
- `POST /api/orders` - Create order from cart
- `GET /api/orders` - List user orders
- `GET /api/orders/{id}` - Get order details
- `PUT /api/orders/{id}/status` - Update order status

### Users
- `POST /api/users/register` - Register new user
- `POST /api/users/login` - Authenticate user
- `GET /api/users/me` - Get current user profile

## Usage

1. Install the scenario:
   ```bash
   mockforge scenario install ./examples/scenarios/ecommerce-store
   ```

2. Apply to your workspace:
   ```bash
   mockforge scenario use ecommerce-store
   ```

3. Start the server:
   ```bash
   mockforge serve --config config.yaml
   ```

## Example Data

The scenario includes example products, users, and orders in the `examples/` directory.
