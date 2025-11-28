# Svelte Demo - MockForge

This demo shows how to use MockForge-generated Svelte stores with a mock API.

## Features

- **SvelteKit** with TypeScript support
- **Svelte stores** for state management
- **Mock API** integration with MockForge
- **Responsive design** with modern CSS
- **User management** example
- **Reactive UI** with Svelte's reactivity system

## Prerequisites

- Node.js 18+
- MockForge CLI

## Setup

1. **Install dependencies:**
   ```bash
   npm install
   ```

2. **Start MockForge server:**
   ```bash
   # From the examples directory
   mockforge serve --spec user-management-api.json --port 3000
   ```

3. **Generate Svelte client:**
   ```bash
   npm run generate-client
   ```

4. **Update the component:**
   - Uncomment the import statements in `src/routes/+page.svelte`
   - Replace mock data with real API calls using the generated stores

5. **Start the Svelte app:**
   ```bash
   npm run dev
   ```

## Project Structure

```
svelte-demo/
├── src/
│   ├── routes/
│   │   ├── +layout.svelte     # Layout component
│   │   └── +page.svelte       # Main page component
│   ├── app.html               # Main HTML template
│   └── app.css                # Global styles
├── package.json               # Dependencies and scripts
├── svelte.config.js           # Svelte configuration
├── vite.config.js             # Vite configuration
├── tsconfig.json              # TypeScript configuration
└── generated/                 # Generated client files (after running generate-client)
    ├── types.ts
    ├── store.ts
    ├── user-management.svelte
    └── package.json
```

## Generated Files

After running `npm run generate-client`, the following files will be created:

- **`types.ts`** - TypeScript type definitions
- **`store.ts`** - Svelte stores for API calls
- **`user-management.svelte`** - Svelte component
- **`package.json`** - Package configuration for the generated client

## Usage Example

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { createGetUsersStore } from '../generated/store';
  import type { User } from '../generated/types';

  let users: User[] = [];
  let loading = false;
  let error: string | null = null;

  // Create the store
  const usersStore = createGetUsersStore();

  onMount(() => {
    // Subscribe to store updates
    const unsubscribe = usersStore.subscribe((state) => {
      users = state.data || [];
      loading = state.loading;
      error = state.error?.message || null;
    });

    // Execute the API call
    usersStore.execute();

    return unsubscribe;
  });

  function refreshUsers() {
    usersStore.refresh();
  }
</script>

<div>
  {#if loading}
    <div class="loading">Loading users...</div>
  {:else if error}
    <div class="error">Error: {error}</div>
  {:else}
    <div>
      {#each users as user (user.id)}
        <div class="user-card">
          <h3>{user.name}</h3>
          <p>{user.email}</p>
        </div>
      {/each}
    </div>
  {/if}

  <button on:click={refreshUsers}>Refresh</button>
</div>
```

## Svelte Stores

The generated Svelte stores provide:

- **Reactive state** - Automatically updates UI when data changes
- **Loading states** - Track loading, error, and success states
- **Error handling** - Built-in error management
- **Refresh functionality** - Easy data refresh capabilities

### Store Structure

```typescript
interface StoreState<T> {
  data: T | null;
  loading: boolean;
  error: Error | null;
  execute: () => Promise<void>;
  refresh: () => Promise<void>;
}
```

## API Endpoints

The demo uses the following mock API endpoints:

- `GET /api/users` - Get all users
- `POST /api/users` - Create a new user
- `GET /api/users/{id}` - Get user by ID
- `PUT /api/users/{id}` - Update user
- `DELETE /api/users/{id}` - Delete user

## Customization

You can customize the generated client by:

1. **Modifying templates** in the MockForge plugin
2. **Adding custom options** to the generation command
3. **Extending the generated stores** with additional methods
4. **Creating custom components** that use the generated types

## SvelteKit Features

This demo uses SvelteKit features:

- **File-based routing** - Automatic route generation
- **Server-side rendering** - Better SEO and performance
- **TypeScript support** - Full type safety
- **Vite integration** - Fast development and building

## Troubleshooting

### Common Issues

1. **CORS errors**: Make sure MockForge is running and accessible
2. **Import errors**: Ensure the generated client files exist
3. **Type errors**: Check that TypeScript configuration is correct
4. **Build errors**: Verify all dependencies are installed

### Debug Mode

Enable debug logging in MockForge:
```bash
mockforge serve --spec user-management-api.json --log-level debug
```

## Next Steps

- Explore the generated store files
- Add more complex API operations
- Implement error boundaries
- Add unit tests with Vitest
- Deploy to production

## Learn More

- [Svelte Documentation](https://svelte.dev/docs)
- [SvelteKit Documentation](https://kit.svelte.dev/docs)
- [MockForge Documentation](../README.md)
- [TypeScript Documentation](https://www.typescriptlang.org/docs/)
