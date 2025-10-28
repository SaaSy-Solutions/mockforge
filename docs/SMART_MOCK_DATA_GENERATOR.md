# Smart Mock Data Generator

The Smart Mock Data Generator is a powerful feature that automatically populates API responses with realistic data patterns using token-based templating, domain-specific generators, and AI-generated examples.

## Features

✅ **Token-based Templating** - Use `$random`, `$faker`, and `$ai` tokens in response bodies
✅ **Domain-specific Generators** - Pre-configured generators for Finance, IoT, Healthcare, E-commerce, and Social domains
✅ **High Performance** - Sub-microsecond token resolution (<1µs for simple tokens, <4µs for complex objects)
✅ **AI Integration** - Optional AI-powered data generation using RAG
✅ **Static Override** - Users can override generated data with static values at any time

## Performance

The Smart Mock Data Generator has been benchmarked to ensure high performance:

- **Simple tokens** (3 fields): ~980 ns (0.98 µs)
- **Nested tokens** (4 levels deep): ~1.6 µs
- **Array tokens** (5 items): ~3.0 µs
- **Large objects** (10+ fields): ~3.2 µs
- **Domain generators**: 26-80 ns per field

**Result**: All operations complete in **<200ms**, meeting the performance requirement. In practice, even complex responses with dozens of tokens resolve in microseconds.

## Token Types

### 1. Random Tokens (`$random.*`)

Generate random values on each request:

```json
{
  "id": "$random.uuid",
  "count": "$random.int",
  "small_number": "$random.int.small",
  "large_number": "$random.int.large",
  "price": "$random.float",
  "enabled": "$random.bool",
  "hex_id": "$random.hex",
  "short_hex": "$random.hex.short",
  "code": "$random.alphanumeric",
  "status": "$random.choice"
}
```

**Available `$random` types:**
- `$random.int` - Random integer (0-1000)
- `$random.int.small` - Small integer (0-100)
- `$random.int.large` - Large integer (0-1,000,000)
- `$random.float` - Random float (0.0-1000.0)
- `$random.bool` - Random boolean
- `$random.uuid` - UUID v4
- `$random.hex` - 32-character hex string
- `$random.hex.short` - 8-character hex string
- `$random.alphanumeric` - 10-character alphanumeric string
- `$random.choice` - Random choice from predefined options

### 2. Faker Tokens (`$faker.*`)

Generate realistic fake data using the Faker library:

```json
{
  "name": "$faker.name",
  "email": "$faker.email",
  "phone": "$faker.phone",
  "address": "$faker.address",
  "company": "$faker.company",
  "website": "$faker.url",
  "ip_address": "$faker.ipv4",
  "created_at": "$faker.datetime",
  "description": "$faker.sentence",
  "bio": "$faker.paragraph",
  "uuid": "$faker.uuid"
}
```

**Available `$faker` types:**
- **Person**: `name`, `email`, `phone`
- **Address**: `address`
- **Company**: `company`
- **Internet**: `url`, `ipv4`, `ip`
- **Date/Time**: `date`, `datetime`, `timestamp`, `iso8601`
- **Lorem**: `word`, `words`, `sentence`, `paragraph`
- **ID**: `uuid`

### 3. AI Tokens (`$ai(prompt)`)

Generate intelligent, context-aware data using AI:

```json
{
  "description": "$ai(generate a product description for a laptop)",
  "review": "$ai(write a 5-star customer review)",
  "bio": "$ai(create a professional bio for a software engineer)"
}
```

**Note**: AI tokens require RAG configuration. See [AI Integration](#ai-integration) below.

## Domain-Specific Generators

Pre-configured generators for common domains provide industry-specific data patterns:

### Finance Domain

```json
{
  "account_number": "$domain.finance.account_number",
  "routing_number": "$domain.finance.routing_number",
  "iban": "$domain.finance.iban",
  "swift": "$domain.finance.swift",
  "amount": "$domain.finance.amount",
  "currency": "$domain.finance.currency",
  "transaction_id": "$domain.finance.transaction_id",
  "card_number": "$domain.finance.card_number",
  "cvv": "$domain.finance.cvv",
  "expiry": "$domain.finance.expiry",
  "stock_symbol": "$domain.finance.stock_symbol"
}
```

### IoT Domain

```json
{
  "device_id": "$domain.iot.device_id",
  "sensor_id": "$domain.iot.sensor_id",
  "temperature": "$domain.iot.temperature",
  "humidity": "$domain.iot.humidity",
  "pressure": "$domain.iot.pressure",
  "voltage": "$domain.iot.voltage",
  "battery_level": "$domain.iot.battery_level",
  "latitude": "$domain.iot.latitude",
  "longitude": "$domain.iot.longitude",
  "status": "$domain.iot.status",
  "firmware_version": "$domain.iot.firmware_version",
  "mac_address": "$domain.iot.mac_address"
}
```

### Healthcare Domain

```json
{
  "patient_id": "$domain.healthcare.patient_id",
  "mrn": "$domain.healthcare.mrn",
  "diagnosis_code": "$domain.healthcare.icd10",
  "procedure_code": "$domain.healthcare.cpt",
  "npi": "$domain.healthcare.npi",
  "blood_pressure": "$domain.healthcare.blood_pressure",
  "heart_rate": "$domain.healthcare.heart_rate",
  "temperature": "$domain.healthcare.temperature",
  "blood_glucose": "$domain.healthcare.blood_glucose",
  "blood_type": "$domain.healthcare.blood_type",
  "medication": "$domain.healthcare.medication",
  "dosage": "$domain.healthcare.dosage"
}
```

### E-commerce Domain

```json
{
  "order_id": "$domain.ecommerce.order_id",
  "product_id": "$domain.ecommerce.product_id",
  "product_name": "$domain.ecommerce.product_name",
  "category": "$domain.ecommerce.category",
  "price": "$domain.ecommerce.price",
  "quantity": "$domain.ecommerce.quantity",
  "discount": "$domain.ecommerce.discount",
  "rating": "$domain.ecommerce.rating",
  "shipping_method": "$domain.ecommerce.shipping_method",
  "tracking_number": "$domain.ecommerce.tracking_number",
  "order_status": "$domain.ecommerce.order_status"
}
```

### Social Media Domain

```json
{
  "user_id": "$domain.social.user_id",
  "post_id": "$domain.social.post_id",
  "username": "$domain.social.username",
  "display_name": "$domain.social.display_name",
  "follower_count": "$domain.social.follower_count",
  "following_count": "$domain.social.following_count",
  "likes": "$domain.social.likes",
  "shares": "$domain.social.shares",
  "hashtag": "$domain.social.hashtag",
  "verified": "$domain.social.verified"
}
```

## Usage Examples

### Basic Token Usage

Create an endpoint with token-based responses:

```yaml
routes:
  - path: /api/users/:id
    method: GET
    response:
      status: 200
      body:
        type: Static
        content:
          id: "$random.uuid"
          name: "$faker.name"
          email: "$faker.email"
          phone: "$faker.phone"
          created_at: "$faker.datetime"
          is_active: "$random.bool"
```

### Nested Objects

Tokens work at any nesting level:

```yaml
response:
  body:
    type: Static
    content:
      user:
        id: "$random.uuid"
        profile:
          name: "$faker.name"
          contact:
            email: "$faker.email"
            phone: "$faker.phone"
```

### Arrays

Generate arrays with token-based items:

```yaml
response:
  body:
    type: Static
    content:
      users:
        - id: "$random.uuid"
          name: "$faker.name"
        - id: "$random.uuid"
          name: "$faker.name"
        - id: "$random.uuid"
          name: "$faker.name"
```

### Domain-Specific Example (IoT)

```yaml
routes:
  - path: /api/sensors/:id/readings
    method: GET
    response:
      status: 200
      body:
        type: Faker
        schema:
          device_id: "$random.uuid"
          sensor_id: "$random.uuid"
          readings:
            - temperature: "$random.float"
              humidity: "$random.float"
              pressure: "$random.float"
              timestamp: "$faker.datetime"
            - temperature: "$random.float"
              humidity: "$random.float"
              pressure: "$random.float"
              timestamp: "$faker.datetime"
          location:
            latitude: "$random.float"
            longitude: "$random.float"
          status: "$random.choice"
```

### Mixed Domain Example (E-commerce)

```yaml
routes:
  - path: /api/orders
    method: POST
    response:
      status: 201
      body:
        type: Static
        content:
          order_id: "$random.uuid"
          customer:
            id: "$random.uuid"
            name: "$faker.name"
            email: "$faker.email"
            phone: "$faker.phone"
          items:
            - id: "$random.uuid"
              name: "$faker.word"
              price: "$random.float"
              quantity: "$random.int.small"
            - id: "$random.uuid"
              name: "$faker.word"
              price: "$random.float"
              quantity: "$random.int.small"
          total: "$random.float"
          status: "$random.choice"
          created_at: "$faker.datetime"
```

## AI Integration

To use `$ai()` tokens, configure RAG support:

```rust
use mockforge_data::{TokenResolver, rag::RagConfig};

// Create resolver with RAG support
let rag_config = RagConfig::default();
let resolver = TokenResolver::with_rag(rag_config);

// Resolve tokens with AI support
let response = resolver.resolve(&json!({
    "description": "$ai(generate a product description for a laptop)"
})).await?;
```

## API Integration

### UI Builder Integration

The Smart Mock Data Generator is fully integrated with the UI Builder. When creating or editing endpoints, you can use tokens directly in the response body:

```json
POST /__mockforge/ui-builder/endpoints
{
  "protocol": "http",
  "name": "Get User",
  "enabled": true,
  "config": {
    "type": "Http",
    "method": "GET",
    "path": "/api/users/:id",
    "response": {
      "status": 200,
      "body": {
        "type": "Static",
        "content": {
          "id": "$random.uuid",
          "name": "$faker.name",
          "email": "$faker.email",
          "created_at": "$faker.datetime"
        }
      }
    }
  }
}
```

### Programmatic Usage

```rust
use mockforge_data::{resolve_tokens, TokenResolver};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple token resolution
    let value = json!({
        "id": "$random.uuid",
        "name": "$faker.name",
        "email": "$faker.email"
    });

    let resolved = resolve_tokens(&value).await?;
    println!("{}", serde_json::to_string_pretty(&resolved)?);

    Ok(())
}
```

### Domain Generator Usage

```rust
use mockforge_data::{Domain, DomainGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a finance domain generator
    let generator = DomainGenerator::new(Domain::Finance);

    // Generate domain-specific data
    let account = generator.generate("account_number")?;
    let iban = generator.generate("iban")?;
    let amount = generator.generate("amount")?;

    println!("Account: {}", account);
    println!("IBAN: {}", iban);
    println!("Amount: {}", amount);

    Ok(())
}
```

## Static Data Override

While the Smart Mock Data Generator automatically creates realistic data, you can always override with static values:

```yaml
# Token-based (dynamic)
response:
  body:
    type: Static
    content:
      id: "$random.uuid"
      name: "$faker.name"

# Static override (fixed)
response:
  body:
    type: Static
    content:
      id: "550e8400-e29b-41d4-a716-446655440000"
      name: "Alice Johnson"
```

Simply remove the token syntax and provide the literal value you want.

## Best Practices

1. **Use appropriate token types**: Choose `$random` for IDs and numbers, `$faker` for realistic names/emails/addresses
2. **Domain generators for specialized data**: Use domain-specific generators when working with finance, IoT, or healthcare APIs
3. **Cache AI responses**: AI token resolution is slower; consider caching responses for repeated patterns
4. **Mix static and dynamic**: Combine static values with tokens for semi-dynamic responses
5. **Performance**: Token resolution is extremely fast (<4µs for complex objects), but AI tokens may add latency

## Troubleshooting

### Tokens not resolving

Make sure:
- Token syntax is correct: `$random.uuid`, not `$randomuuid`
- JSON is valid
- Content-Type is `application/json`

### AI tokens not working

Ensure:
- RAG is configured: `TokenResolver::with_rag(rag_config)`
- LLM provider is accessible
- API keys are set in environment

### Performance issues

- Avoid excessive nesting (>10 levels)
- Limit array sizes for token-heavy responses
- Cache responses when possible
- Consider static overrides for frequently accessed data

## Related Documentation

- [Token Resolver API](../crates/mockforge-data/src/token_resolver.rs)
- [Domain Generators](../crates/mockforge-data/src/domains.rs)
- [UI Builder Integration](../crates/mockforge-http/src/ui_builder.rs)
- [Performance Benchmarks](../crates/mockforge-data/benches/token_resolver_bench.rs)
