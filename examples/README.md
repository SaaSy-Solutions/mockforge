# MockForge Multi-Framework Examples

This directory contains complete example applications demonstrating MockForge's multi-framework client generation capabilities.

## Examples Overview

### React Demo (`react-demo/`)
A complete React application showcasing:
- Generated React hooks for API calls
- TypeScript type safety
- Form handling with generated types
- Error handling and loading states
- Modern React patterns with hooks

### Vue Demo (`vue-demo/`)
A complete Vue 3 application showcasing:
- Generated Vue composables for API calls
- Pinia store integration
- TypeScript type safety
- Form handling with generated types
- Error handling and loading states
- Vue 3 Composition API patterns

### Angular Demo (`angular-demo/`)
A complete Angular 17 application showcasing:
- Generated Angular services for API calls
- Standalone components architecture
- TypeScript type safety
- Form handling with generated types
- Error handling and loading states
- Angular dependency injection patterns

### Svelte Demo (`svelte-demo/`)
A complete SvelteKit application showcasing:
- Generated Svelte stores for API calls
- Reactive state management
- TypeScript type safety
- Form handling with generated types
- Error handling and loading states
- SvelteKit file-based routing

## Getting Started

### Prerequisites

1. **Install MockForge CLI**:
   ```bash
   cargo install mockforge-cli
   ```

2. **Install Node.js** (version 16 or higher)

3. **Install pnpm** (recommended) or npm:
```bash
   npm install -g pnpm
   ```

### Running the Examples

#### 1. Generate Client Code

First, generate the client code for your preferred framework:

```bash
# For React
cd react-demo
npm run generate-client

# For Vue
cd vue-demo
npm run generate-client

# For Angular
cd angular-demo
npm run generate-client

# For Svelte
cd svelte-demo
npm run generate-client
```

This will create a `generated/` directory with:
- TypeScript type definitions
- Framework-specific hooks/composables
- API client
- Package configuration
- Documentation

#### 2. Install Dependencies

```bash
# For React
cd react-demo
npm install

# For Vue
cd vue-demo
npm install

# For Angular
cd angular-demo
npm install

# For Svelte
cd svelte-demo
npm install
```

#### 3. Start MockForge Server

In a separate terminal, start the MockForge server with the sample API:

```bash
mockforge serve --spec ../user-management-api.json --port 3000
```

#### 4. Start the Application

```bash
# For React
cd react-demo
npm start

# For Vue
cd vue-demo
npm run dev

# For Angular
cd angular-demo
npm start

# For Svelte
cd svelte-demo
npm run dev
```

The applications will be available at:
- React: http://localhost:3001
- Vue: http://localhost:5173
- Angular: http://localhost:4200
- Svelte: http://localhost:5173

## Example API Specification

The examples use a comprehensive User Management API specification (`user-management-api.json`) that includes:

- **Users**: CRUD operations for user management
- **Posts**: Blog post management with user relationships
- **Comments**: Comment system with post relationships

### API Endpoints

- `GET /users` - List all users
- `POST /users` - Create a new user
- `GET /users/{id}` - Get user by ID
- `PUT /users/{id}` - Update user
- `DELETE /users/{id}` - Delete user
- `GET /posts` - List all posts
- `POST /posts` - Create a new post
- `GET /posts/{id}` - Get post by ID
- `GET /comments` - List all comments
- `POST /comments` - Create a new comment

## Generated Code Structure

### React Client
```
generated/
├── types.ts          # TypeScript type definitions
├── hooks.ts          # React hooks and API client
├── package.json      # Package configuration
└── README.md         # Usage documentation
```

### Vue Client
```
generated/
├── types.ts          # TypeScript type definitions
├── composables.ts    # Vue composables and API client
├── store.ts          # Pinia store for state management
├── package.json      # Package configuration
└── README.md         # Usage documentation
```

### Angular Client
```
generated/
├── types.ts                    # TypeScript type definitions
├── user-management.service.ts  # Angular service for API calls
├── user-management.module.ts   # Angular module configuration
├── package.json               # Package configuration
└── README.md                  # Usage documentation
```

### Svelte Client
```
generated/
├── types.ts              # TypeScript type definitions
├── store.ts              # Svelte stores and API client
├── user-management.svelte # Svelte component
├── package.json          # Package configuration
└── README.md             # Usage documentation
```

## Key Features Demonstrated

### 1. Type Safety
Both examples showcase full TypeScript integration with generated types:

```typescript
// Generated types
interface User {
  id: number;
  name: string;
  email: string;
  avatar?: string;
  createdAt: string;
  updatedAt: string;
}

// Type-safe API calls
const { data: users } = useGetUsers(); // users is User[]
```

### 2. Reactive Data Fetching

#### React
```typescript
const { data, loading, error } = useGetUsers();

if (loading) return <div>Loading...</div>;
if (error) return <div>Error: {error.message}</div>;

return (
  <div>
    {data?.map(user => (
      <div key={user.id}>{user.name}</div>
    ))}
  </div>
);
```

#### Vue
```vue
<template>
  <div v-if="loading">Loading...</div>
  <div v-else-if="error">Error: {{ error.message }}</div>
  <div v-else>
    <div v-for="user in data" :key="user.id">
      {{ user.name }}
    </div>
  </div>
</template>

<script setup lang="ts">
const { data, loading, error } = useGetUsers();
</script>
```

#### Angular
```typescript
@Component({
  template: `
    <div *ngIf="loading">Loading...</div>
    <div *ngIf="error">Error: {{ error.message }}</div>
    <div *ngFor="let user of users">
      {{ user.name }}
    </div>
  `
})
export class UserComponent implements OnInit {
  users: User[] = [];
  loading = false;
  error: Error | null = null;

  constructor(private userService: UserManagementService) {}

  ngOnInit() {
    this.loadUsers();
  }

  loadUsers() {
    this.loading = true;
    this.userService.getUsers().subscribe({
      next: (users) => {
        this.users = users;
        this.loading = false;
      },
      error: (error) => {
        this.error = error;
        this.loading = false;
      }
    });
  }
}
```

#### Svelte
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { createGetUsersStore } from './generated/store';

  let users: User[] = [];
  let loading = false;
  let error: string | null = null;

  const usersStore = createGetUsersStore();

  onMount(() => {
    const unsubscribe = usersStore.subscribe((state) => {
      users = state.data || [];
      loading = state.loading;
      error = state.error?.message || null;
    });

    usersStore.execute();
    return unsubscribe;
  });
</script>

{#if loading}
  <div>Loading...</div>
{:else if error}
  <div>Error: {error}</div>
{:else}
  {#each users as user (user.id)}
    <div>{user.name}</div>
  {/each}
{/if}
```

### 3. Form Handling
Both examples include forms for creating users and posts with proper validation and error handling.

### 4. Error Handling
Comprehensive error handling patterns are demonstrated:
- Network errors
- Validation errors
- User feedback
- Loading states

## Customization

### Custom Base URL
You can customize the API base URL by modifying the generated client configuration:

```typescript
// In generated/hooks.ts or generated/composables.ts
const defaultConfig: ApiConfig = {
  baseUrl: process.env.REACT_APP_API_URL || 'http://localhost:3000',
  headers: {
    'Content-Type': 'application/json',
  },
};
```

### Environment Variables
Set up environment variables for different environments:

```bash
# .env.development
REACT_APP_API_URL=http://localhost:3000

# .env.production
REACT_APP_API_URL=https://api.myapp.com
```

## Troubleshooting

### Common Issues

1. **Client generation fails**:
   - Ensure MockForge CLI is installed and in PATH
   - Check that the OpenAPI spec file exists and is valid
   - Verify the framework name is correct (react, vue, angular, svelte)

2. **TypeScript compilation errors**:
   - Ensure all generated types are properly imported
   - Check that dependencies are installed
   - Verify TypeScript configuration

3. **API connection issues**:
   - Ensure MockForge server is running on the correct port
   - Check the base URL configuration
   - Verify CORS settings if needed

4. **Missing dependencies**:
   - Run `npm install` in the generated client directory
   - Check that peer dependencies are installed

### Getting Help

- Check the generated README.md for framework-specific instructions
- Review the MockForge documentation
- Open an issue on the MockForge GitHub repository

## Next Steps

After running these examples, you can:

1. **Modify the API specification** to match your needs
2. **Customize the generated templates** for your specific requirements
3. **Add new frameworks** by implementing the `ClientGeneratorPlugin` trait
4. **Integrate with your existing applications** using the generated clients

## Contributing

We welcome contributions to improve these examples! Please see our [Contributing Guide](../../CONTRIBUTING.md) for details on how to get started.
