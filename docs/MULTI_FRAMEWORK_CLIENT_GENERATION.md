# Multi-Framework Client Generation

MockForge now supports generating framework-specific mock clients from OpenAPI specifications. This feature enables developers to quickly integrate mock APIs into their frontend applications using familiar patterns and tools.

## Supported Frameworks

- **React** - Generates React hooks and TypeScript types
- **Vue** - Generates Vue 3 composables, Pinia stores, and TypeScript types
- **Angular** - Coming soon
- **Svelte** - Coming soon

## Quick Start

### 1. Generate Client Code

Use the MockForge CLI to generate client code for your preferred framework:

```bash
# Generate React client
mockforge client generate --spec api-spec.json --framework react --output ./generated

# Generate Vue client
mockforge client generate --spec api-spec.json --framework vue --output ./generated
```

### 2. Install Generated Client

```bash
cd generated
npm install
```

### 3. Use in Your Application

#### React Example

```typescript
import { useGetUsers, useCreateUser } from './generated/hooks';

function UserList() {
  const { data: users, loading, error } = useGetUsers();
  const { execute: createUser } = useCreateUser();

  const handleCreateUser = async (userData) => {
    await createUser(userData);
  };

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div>
      {users?.map(user => (
        <div key={user.id}>{user.name}</div>
      ))}
    </div>
  );
}
```

#### Vue Example

```vue
<template>
  <div>
    <div v-if="loading">Loading...</div>
    <div v-else-if="error">Error: {{ error.message }}</div>
    <div v-else>
      <div v-for="user in data" :key="user.id">
        {{ user.name }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useGetUsers } from './generated/composables';

const { data, loading, error } = useGetUsers();
</script>
```

## Generated Files

Each framework generates a set of files tailored to its ecosystem:

### React Client
- `types.ts` - TypeScript type definitions
- `hooks.ts` - React hooks and API client
- `package.json` - Package configuration
- `README.md` - Usage documentation

### Vue Client
- `types.ts` - TypeScript type definitions
- `composables.ts` - Vue 3 composables and API client
- `store.ts` - Pinia store for state management
- `package.json` - Package configuration
- `README.md` - Usage documentation

## Configuration Options

The client generation supports various configuration options:

```bash
mockforge client generate \
  --spec api-spec.json \
  --framework react \
  --output ./generated \
  --base-url http://localhost:3000 \
  --include-types true \
  --include-mocks false \
  --options '{"customOption": "value"}'
```

### Available Options

- `--spec` - Path to OpenAPI specification file (JSON or YAML)
- `--framework` - Target framework (react, vue, angular, svelte)
- `--output` - Output directory for generated files
- `--base-url` - Base URL for the API (default: http://localhost:3000)
- `--include-types` - Include TypeScript types (default: true)
- `--include-mocks` - Include mock data generation (default: false)
- `--template-dir` - Custom template directory
- `--options` - Additional options as JSON string

## Plugin Architecture

The multi-framework support is built on MockForge's plugin system, making it easy to add support for new frameworks.

### Creating a Custom Framework Plugin

1. **Implement the ClientGeneratorPlugin trait:**

```rust
use mockforge_plugin_core::{
    ClientGeneratorPlugin, ClientGeneratorConfig,
    ClientGenerationResult, OpenApiSpec
};

pub struct MyFrameworkGenerator;

#[async_trait::async_trait]
impl ClientGeneratorPlugin for MyFrameworkGenerator {
    fn framework_name(&self) -> &str {
        "my-framework"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["ts", "js"]
    }

    async fn generate_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult> {
        // Implementation here
    }

    async fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("My Framework Generator")
            .with_capability("client_generator")
            .with_description("Generates clients for My Framework")
    }
}
```

2. **Register the plugin:**

```rust
use mockforge_plugin_core::ClientGeneratorManager;

let mut manager = ClientGeneratorManager::new()?;
manager.register_generator("my-framework", Box::new(MyFrameworkGenerator::new()?));
```

### Plugin Interface

The `ClientGeneratorPlugin` trait provides a standardized interface for generating framework-specific clients:

```rust
#[async_trait::async_trait]
pub trait ClientGeneratorPlugin {
    /// Get the framework name this plugin supports
    fn framework_name(&self) -> &str;

    /// Get the supported file extensions for this framework
    fn supported_extensions(&self) -> Vec<&str>;

    /// Generate mock client code from OpenAPI specification
    async fn generate_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult>;

    /// Get plugin metadata
    async fn get_metadata(&self) -> PluginMetadata;

    /// Validate the plugin configuration
    async fn validate_config(&self, config: &ClientGeneratorConfig) -> Result<()>;
}
```

## Examples

### Complete React Application

See the `examples/react-demo/` directory for a complete React application that demonstrates:

- Using generated React hooks
- Form handling with generated types
- Error handling and loading states
- Real-time data updates

### Complete Vue Application

See the `examples/vue-demo/` directory for a complete Vue 3 application that demonstrates:

- Using generated Vue composables
- Pinia store integration
- Form handling with generated types
- Error handling and loading states

## Best Practices

### 1. Type Safety

Always use the generated TypeScript types for better development experience:

```typescript
import { User, CreateUserRequest } from './generated/types';

const user: User = {
  id: 1,
  name: "John Doe",
  email: "john@example.com"
};
```

### 2. Error Handling

Implement proper error handling for API calls:

```typescript
const { data, loading, error } = useGetUsers();

if (error) {
  console.error('API Error:', error.message);
  // Handle error appropriately
}
```

### 3. Loading States

Always handle loading states for better UX:

```typescript
if (loading) {
  return <LoadingSpinner />;
}
```

### 4. Configuration Management

Use environment variables for API configuration:

```typescript
const config = {
  baseUrl: process.env.REACT_APP_API_URL || 'http://localhost:3000'
};
```

## Troubleshooting

### Common Issues

1. **TypeScript compilation errors**: Ensure all generated types are properly imported
2. **Missing dependencies**: Run `npm install` in the generated client directory
3. **API connection issues**: Verify the base URL configuration

### Getting Help

- Check the generated README.md for framework-specific usage instructions
- Review the examples in the `examples/` directory
- Open an issue on the MockForge GitHub repository

## Roadmap

### Planned Features

- **Angular Support** - Generate Angular services and components
- **Svelte Support** - Generate Svelte stores and components
- **GraphQL Support** - Generate clients from GraphQL schemas
- **Custom Templates** - Support for custom code generation templates
- **Live Reload** - Automatic regeneration on spec changes

### Contributing

We welcome contributions to add support for new frameworks! Please see our [Contributing Guide](CONTRIBUTING.md) for details on how to get started.
