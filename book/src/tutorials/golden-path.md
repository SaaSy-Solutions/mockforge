# The Golden Path: Blueprint → Dev-Setup → Integration

This guide walks you through the **Golden Path** - the fastest way to get from zero to a fully integrated mock API in your frontend application. This path is designed to take less than 10 minutes and provide a magical first experience.

## Overview

The Golden Path consists of three steps:

1. **Blueprint**: Start with a pre-configured app archetype
2. **Dev-Setup**: One-command frontend integration
3. **Integration**: Use generated code in your app

## Step 1: Choose and Create from a Blueprint

Blueprints are pre-configured application archetypes that include:
- **Personas**: Realistic user profiles with consistent data
- **Reality defaults**: Optimized realism levels for your use case
- **Sample flows**: Common workflows (signup, checkout, etc.)
- **Scenarios**: Happy paths, known failures, and slow paths
- **Contracts**: JSON Schema validation for endpoints
- **Playground collections**: Pre-configured test scenarios

### Available Blueprints

List available blueprints:

```bash
mockforge blueprint list
```

You'll see blueprints like:
- **b2c-saas**: B2C SaaS with authentication, subscriptions, and billing
- **ecommerce**: E-commerce with products, cart, and checkout
- **banking-lite**: Banking app with accounts, transactions, and transfers

### Create Your Project

Choose a blueprint and create your project:

```bash
# For a B2C SaaS app
mockforge init my-saas-app --blueprint b2c-saas

# For an e-commerce app
mockforge init my-store --blueprint ecommerce

# For a banking app
mockforge init my-bank --blueprint banking-lite
```

This command:
- Creates a new project directory
- Copies all blueprint files (config, personas, flows, scenarios, contracts)
- Sets up the `mockforge.yaml` configuration
- Creates a README with API documentation

### Explore What You Got

```bash
cd my-saas-app
ls -la
```

You'll see:
- `mockforge.yaml` - Main configuration
- `personas/` - User personas with consistent data
- `scenarios/` - Multi-step API workflows
- `contracts/` - JSON Schema validation
- `README.md` - API documentation

### Start the Mock Server

```bash
mockforge serve
```

Your mock API is now running! Visit `http://localhost:3000` to see it in action.

**Try it out:**
```bash
# Test the signup endpoint
curl -X POST http://localhost:3000/api/auth/signup \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123"}'
```

## Step 2: One-Command Frontend Integration

Now that your mock API is running, integrate it into your frontend application with a single command.

### Supported Frameworks

The `dev-setup` command supports:
- **React** / **Next.js**
- **Vue** / **Nuxt**
- **Angular**
- **Svelte**

### Run Dev-Setup

Navigate to your frontend project and run:

```bash
# For React
mockforge dev-setup react

# For Vue
mockforge dev-setup vue

# For Angular
mockforge dev-setup angular

# For Svelte
mockforge dev-setup svelte
```

### What Dev-Setup Does

The command automatically:

1. **Detects your project structure** - Finds your frontend framework and configuration
2. **Detects existing MockForge workspace** - If you're in a blueprint project, it auto-detects the config
3. **Auto-detects OpenAPI spec** - Finds `openapi.yaml`, `openapi.json`, or spec in `mockforge.yaml`
4. **Generates typed client** - Creates a fully-typed API client from your OpenAPI spec
5. **Creates framework examples** - Generates example hooks/composables/services
6. **Sets up environment variables** - Creates `.env.mockforge.example` with configuration
7. **Installs dependencies** - Adds required packages to your `package.json`

### Example Output

```
🚀 Setting up MockForge for react...

  ✓ Detected project root: /path/to/my-frontend
  ✓ Detected existing MockForge workspace configuration
    Base URL: http://localhost:3000
    Reality level: moderate
  ✓ Found OpenAPI spec: openapi.yaml
  ✓ Generated typed client: src/mockforge/client.ts
  ✓ Created React Query hooks: src/mockforge/hooks.ts
  ✓ Created example component: src/components/MockForgeExample.tsx
  ✓ Created environment template: .env.mockforge.example

✅ Setup complete!

Next steps:
  1. Copy .env.mockforge.example to .env.local
  2. Check out src/components/MockForgeExample.tsx
  3. Start using the generated hooks in your components
```

## Step 3: Integrate into Your App

Now use the generated code in your application.

### React / Next.js Example

```tsx
import { useGetUsers, useCreateUser } from '@/mockforge/hooks';

function UsersList() {
  const { data: users, isLoading, error } = useGetUsers();
  const createUser = useCreateUser();

  const handleCreate = async () => {
    await createUser.mutateAsync({
      email: 'newuser@example.com',
      name: 'New User',
    });
  };

  if (isLoading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div>
      <button onClick={handleCreate}>Create User</button>
      <ul>
        {users?.map(user => (
          <li key={user.id}>{user.name} - {user.email}</li>
        ))}
      </ul>
    </div>
  );
}
```

### Vue / Nuxt Example

```vue
<template>
  <div>
    <button @click="createUser">Create User</button>
    <ul v-if="users">
      <li v-for="user in users" :key="user.id">
        {{ user.name }} - {{ user.email }}
      </li>
    </ul>
  </div>
</template>

<script setup>
import { useGetUsers, useCreateUser } from '@/mockforge/composables';

const { data: users, isLoading, error } = useGetUsers();
const { mutate: createUser } = useCreateUser();

const handleCreate = () => {
  createUser({
    email: 'newuser@example.com',
    name: 'New User',
  });
};
</script>
```

### Angular Example

```typescript
import { Component } from '@angular/core';
import { UserService } from '@/mockforge/services/user.service';

@Component({
  selector: 'app-users',
  template: `
    <button (click)="createUser()">Create User</button>
    <ul>
      <li *ngFor="let user of users$ | async">
        {{ user.name }} - {{ user.email }}
      </li>
    </ul>
  `,
})
export class UsersComponent {
  users$ = this.userService.getUsers();

  constructor(private userService: UserService) {}

  createUser() {
    this.userService.createUser({
      email: 'newuser@example.com',
      name: 'New User',
    }).subscribe();
  }
}
```

### Svelte Example

```svelte
<script>
  import { getUsers, createUser } from '@/mockforge/stores';

  let users = $state([]);

  $effect(() => {
    getUsers().then(data => users = data);
  });

  async function handleCreate() {
    await createUser({
      email: 'newuser@example.com',
      name: 'New User',
    });
    // Refresh users
    users = await getUsers();
  }
</script>

<button on:click={handleCreate}>Create User</button>
<ul>
  {#each users as user}
    <li>{user.name} - {user.email}</li>
  {/each}
</ul>
```

## Complete Workflow Example

Let's walk through a complete example from start to finish:

### 1. Create Project from Blueprint

```bash
mockforge init my-saas-app --blueprint b2c-saas
cd my-saas-app
```

### 2. Start Mock Server

```bash
mockforge serve
```

The server starts on `http://localhost:3000` with:
- Authentication endpoints (`/api/auth/*`)
- User management (`/api/users/*`)
- Subscription management (`/api/subscriptions/*`)
- Billing endpoints (`/api/billing/*`)

### 3. Set Up Frontend

In a separate terminal, navigate to your React app:

```bash
cd ../my-react-app
mockforge dev-setup react
```

This generates:
- `src/mockforge/client.ts` - Typed API client
- `src/mockforge/hooks.ts` - React Query hooks
- `src/components/MockForgeExample.tsx` - Example component

### 4. Configure Environment

```bash
cp .env.mockforge.example .env.local
```

Edit `.env.local`:
```env
NEXT_PUBLIC_MOCKFORGE_URL=http://localhost:3000
MOCKFORGE_REALITY_LEVEL=moderate
```

### 5. Use in Your App

```tsx
// app/users/page.tsx
import { useGetUsers } from '@/mockforge/hooks';

export default function UsersPage() {
  const { data: users, isLoading } = useGetUsers();

  if (isLoading) return <div>Loading...</div>;

  return (
    <div>
      <h1>Users</h1>
      <ul>
        {users?.map(user => (
          <li key={user.id}>
            {user.name} ({user.email})
          </li>
        ))}
      </ul>
    </div>
  );
}
```

### 6. Test the Integration

```bash
# Start your frontend app
npm run dev

# Visit http://localhost:3001 (or your app's port)
# You should see users loaded from the mock API!
```

## Advanced Features

### Using Personas

Blueprints include personas for consistent data. Use them in your tests:

```typescript
// In your test or component
import { usePersona } from '@/mockforge/hooks';

// Use a specific persona
const { data: user } = useGetUser({ persona: 'premium-user' });
```

### Using Scenarios

Test different scenarios (happy path, failures, slow paths):

```typescript
// Activate a scenario
await activateScenario('happy-path-signup');

// Make API calls - they'll follow the scenario
const response = await signup({ email: 'test@example.com' });
```

### Config Validation

The VS Code extension provides real-time validation:

1. Open `mockforge.yaml` in VS Code
2. See inline errors for invalid configuration
3. Get autocomplete for all options

### Playground Integration

Test endpoints interactively:

1. Hover over an endpoint reference in your code
2. Click "Open in Playground"
3. Test the endpoint with different parameters

## Troubleshooting

### Dev-Setup Can't Find OpenAPI Spec

If dev-setup can't auto-detect your OpenAPI spec:

```bash
# Specify it explicitly
mockforge dev-setup react --spec ./openapi.yaml
```

### Mock Server Not Running

Make sure the mock server is running:

```bash
# In your blueprint project
mockforge serve
```

### Type Errors in Generated Client

Regenerate the client:

```bash
mockforge dev-setup react --force
```

### Port Conflicts

If port 3000 is in use:

```bash
# Change the port in mockforge.yaml
base_url: http://localhost:3001

# Or use environment variable
MOCKFORGE_PORT=3001 mockforge serve
```

## Next Steps

Now that you've completed the Golden Path:

1. **Explore Personas**: Use different personas for varied test data
2. **Customize Reality**: Adjust the reality slider for different test scenarios
3. **Add Scenarios**: Create custom scenarios for your workflows
4. **Extend Contracts**: Add more JSON Schema contracts for validation
5. **Create Custom Blueprints**: Build your own blueprints for your team

## Related Documentation

- [Blueprints Guide](../user-guide/blueprints.md) - Learn more about blueprints
- [Dev-Setup Reference](../api/cli.md#dev-setup) - Complete CLI reference
- [React Workflow Tutorial](./react-workflow.md) - Detailed React integration
- [Vue Workflow Tutorial](./vue-workflow.md) - Detailed Vue integration
- [IDE Integration](../user-guide/ide-integration.md) - VS Code extension features

