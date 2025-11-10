# Smart Personas

Smart Personas enable generating coherent, consistent mock data using persona profiles with unique backstories and deterministic generation. The same persona always generates the same data, ensuring consistency across endpoints and requests.

## Overview

Smart Personas provide:

- **Persona Profiles**: Unique personas with IDs and domain associations
- **Coherent Backstories**: Template-based backstory generation
- **Persona Relationships**: Connections between personas (users, devices, organizations)
- **Deterministic Generation**: Same persona = same data every time
- **Domain-Specific Templates**: Finance, E-commerce, Healthcare, IoT personas

## Quick Start

### Enable Smart Personas

```yaml
# config.yaml
data:
  personas:
    enabled: true
    auto_generate_backstories: true
    domain: "ecommerce"  # or "finance", "healthcare", "iot"
```

### Use in Templates

```yaml
responses:
  - path: "/api/users/{id}"
    body: |
      {
        "id": "{{persona.id}}",
        "name": "{{persona.name}}",
        "email": "{{persona.email}}",
        "backstory": "{{persona.backstory}}"
      }
```

## Persona Profiles

### Automatic Persona Creation

Personas are automatically created when referenced:

```bash
# Request to /api/users/123
# Persona with ID "123" is automatically created
# Same persona used for all requests with ID "123"
```

### Manual Persona Creation

```rust
use mockforge_data::{PersonaProfile, PersonaRegistry};

let mut registry = PersonaRegistry::new();
let persona = PersonaProfile::new("user-123", "ecommerce");
registry.add_persona(persona);
```

## Backstories

### Automatic Backstory Generation

Backstories are automatically generated based on domain:

```yaml
data:
  personas:
    enabled: true
    auto_generate_backstories: true
    domain: "ecommerce"
```

### Domain-Specific Templates

#### E-commerce

```
"Alice is a 32-year-old marketing professional living in San Francisco. 
She frequently shops online for electronics and fashion items. 
Her average order value is $150, and she prefers express shipping."
```

#### Finance

```
"Bob is a 45-year-old investment banker based in New York. 
He manages a portfolio worth $2.5M and prefers conservative investments. 
He has been a customer for 8 years."
```

#### Healthcare

```
"Carol is a 28-year-old nurse practitioner in Boston. 
She manages chronic conditions for 50+ patients. 
She prefers digital health tools and telemedicine."
```

#### IoT

```
"Device-001 is a smart thermostat installed in a 3-bedroom home in Seattle. 
It monitors temperature, humidity, and energy usage. 
It's connected to 5 other smart home devices."
```

### Custom Backstories

Set custom backstories:

```rust
let mut persona = PersonaProfile::new("user-123", "ecommerce");
persona.set_backstory("Custom backstory text".to_string());
```

## Persona Relationships

### Define Relationships

```rust
use mockforge_data::PersonaRegistry;

let mut registry = PersonaRegistry::new();

// Add relationship
registry.add_relationship(
    "user-123",
    "device-456",
    "owns"
);

// Get related personas
let devices = registry.get_related_personas("user-123", "owns");
```

### Relationship Types

Common relationship types:

- `owns` - User owns device/organization
- `belongs_to` - Device/organization belongs to user
- `manages` - User manages organization
- `connected_to` - Device connected to other device
- `parent_of` - Organization parent-child relationship

### Cross-Entity Consistency

Same base ID across different entity types:

```rust
// User persona
let user = registry.get_or_create_persona_by_type("123", EntityType::User, "ecommerce");

// Device persona (same ID, different type)
let device = registry.get_or_create_persona_by_type("123", EntityType::Device, "iot");

// Automatically establishes relationship
```

## Deterministic Generation

### Same Persona, Same Data

The same persona always generates the same data:

```bash
# First request
GET /api/users/123
# Response: {"id": 123, "name": "Alice", "email": "alice@example.com"}

# Second request (same persona ID)
GET /api/users/123
# Response: {"id": 123, "name": "Alice", "email": "alice@example.com"}  # Same!
```

### Seed-Based Generation

Personas use deterministic seeds:

```rust
let persona = PersonaProfile::new("user-123", "ecommerce");
// Seed is derived from persona ID and domain
// Same ID + same domain = same seed = same data
```

## Template Functions

### Persona Functions

```yaml
# In response templates
{
  "id": "{{persona.id}}",
  "name": "{{persona.name}}",
  "email": "{{persona.email}}",
  "phone": "{{persona.phone}}",
  "address": "{{persona.address}}",
  "backstory": "{{persona.backstory}}",
  "traits": "{{persona.traits}}"
}
```

### Relationship Functions

```yaml
# Get related personas
{
  "user": {
    "id": "{{persona.id}}",
    "name": "{{persona.name}}"
  },
  "devices": "{{persona.related.owns}}"
}
```

## Configuration

### Full Configuration

```yaml
data:
  personas:
    enabled: true
    auto_generate_backstories: true
    domain: "ecommerce"  # finance, healthcare, iot, generic
    backstory_templates:
      ecommerce:
        - "{{name}} is a {{age}}-year-old {{profession}} living in {{city}}."
        - "They frequently shop for {{interests}} with an average order value of ${{avg_order_value}}."
    relationship_types:
      - owns
      - belongs_to
      - manages
      - connected_to
```

## Use Cases

### Consistent User Data

Generate consistent user data across endpoints:

```yaml
# User endpoint
responses:
  - path: "/api/users/{id}"
    body: |
      {
        "id": "{{persona.id}}",
        "name": "{{persona.name}}",
        "email": "{{persona.email}}"
      }

# User's orders endpoint
responses:
  - path: "/api/users/{id}/orders"
    body: |
      {
        "user_id": "{{persona.id}}",
        "user_name": "{{persona.name}}",
        "orders": [...]
      }
```

### Device Relationships

Model device ownership:

```yaml
# Device endpoint
responses:
  - path: "/api/devices/{id}"
    body: |
      {
        "id": "{{persona.id}}",
        "owner_id": "{{persona.relationship.owner}}",
        "type": "{{persona.type}}"
      }
```

### Organization Hierarchies

Model organizational structures:

```yaml
# Organization endpoint
responses:
  - path: "/api/organizations/{id}"
    body: |
      {
        "id": "{{persona.id}}",
        "name": "{{persona.name}}",
        "parent_id": "{{persona.relationship.parent}}",
        "children": "{{persona.related.children}}"
      }
```

## Best Practices

1. **Use Consistent IDs**: Use the same persona ID across related endpoints
2. **Choose Appropriate Domain**: Select domain that matches your use case
3. **Leverage Relationships**: Use relationships to model complex data structures
4. **Customize Backstories**: Add domain-specific details to backstories
5. **Test Determinism**: Verify same persona generates same data

## Troubleshooting

### Persona Not Found

- Ensure personas are enabled in configuration
- Check persona ID is consistent across requests
- Verify domain matches persona domain

### Backstory Not Generated

- Check `auto_generate_backstories` is enabled
- Verify domain is supported
- Review persona creation logs

### Relationships Not Working

- Verify relationship types are defined
- Check relationship is added to registry
- Review relationship query syntax

## Related Documentation

- [VBR Engine](vbr-engine.md) - State management with personas
- [Data Generation](../reference/fixtures.md) - Data generation features
- [Configuration Guide](../configuration/files.md) - Complete configuration reference

