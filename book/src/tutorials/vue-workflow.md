# Vue + MockForge Workflow

**Goal**: Build a Vue 3 application that uses MockForge as a backend mock server for development and testing.

**Time**: 10-15 minutes

## Overview

This tutorial shows you how to:
1. Set up MockForge with an OpenAPI specification
2. Generate TypeScript client code for Vue 3
3. Build a Vue app that consumes the mock API using Pinia
4. Develop and test frontend features against mock data

## Prerequisites

- MockForge installed ([Installation Guide](../getting-started/installation.md))
- Node.js 16+ and npm/pnpm installed
- Basic Vue 3 and TypeScript knowledge

## Step 1: Prepare Your OpenAPI Specification

Create or use an existing OpenAPI spec. We'll use the same User Management API from the React tutorial:

**`user-management-api.json`:**
```json
{
  "openapi": "3.0.3",
  "info": {
    "title": "User Management API",
    "version": "1.0.0"
  },
  "paths": {
    "/users": {
      "get": {
        "summary": "List all users",
        "responses": {
          "200": {
            "description": "List of users",
            "content": {
              "application/json": {
                "schema": {
                  "type": "array",
                  "items": {
                    "$ref": "#/components/schemas/User"
                  }
                }
              }
            }
          }
        }
      },
      "post": {
        "summary": "Create a user",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/UserInput"
              }
            }
          }
        },
        "responses": {
          "201": {
            "description": "User created",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/User"
                }
              }
            }
          }
        }
      }
    },
    "/users/{id}": {
      "get": {
        "summary": "Get user by ID",
        "parameters": [
          {
            "name": "id",
            "in": "path",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "200": {
            "description": "User details",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/User"
                }
              }
            }
          }
        }
      }
    }
  },
  "components": {
    "schemas": {
      "User": {
        "type": "object",
        "required": ["id", "name", "email"],
        "properties": {
          "id": {
            "type": "string",
            "example": "{{uuid}}"
          },
          "name": {
            "type": "string",
            "example": "John Doe"
          },
          "email": {
            "type": "string",
            "format": "email",
            "example": "john@example.com"
          },
          "createdAt": {
            "type": "string",
            "format": "date-time",
            "example": "{{now}}"
          }
        }
      },
      "UserInput": {
        "type": "object",
        "required": ["name", "email"],
        "properties": {
          "name": {
            "type": "string"
          },
          "email": {
            "type": "string",
            "format": "email"
          }
        }
      }
    }
  }
}
```

## Step 2: Start MockForge Server

Start the mock server with your OpenAPI spec:

```bash
# Terminal 1: Start MockForge
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
  mockforge serve --spec user-management-api.json --http-port 3000 --admin
```

You should see:
```
🚀 MockForge v1.0.0 starting...
📡 HTTP server listening on 0.0.0.0:3000
✅ Ready to serve requests at http://localhost:3000
```

**Tip**: Keep this terminal running. The `--admin` flag enables the admin UI at http://localhost:9080.

## Step 3: Create Vue Application

Create a new Vue 3 app with TypeScript:

```bash
# Create Vue app with TypeScript
npm create vue@latest my-app
cd my-app

# Select TypeScript when prompted
# Install dependencies
npm install
```

## Step 4: Install Pinia (State Management)

```bash
npm install pinia
```

Set up Pinia in `src/main.ts`:

```typescript
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'

const app = createApp(App)
app.use(createPinia())
app.mount('#app')
```

## Step 5: Generate TypeScript Client (Optional)

MockForge can generate type-safe Vue composables from your OpenAPI spec:

```bash
# Install MockForge CLI as dev dependency
npm install --save-dev mockforge-cli

# Add to package.json scripts
```

Update `package.json`:

```json
{
  "scripts": {
    "generate-client": "mockforge client generate --spec ../user-management-api.json --framework vue --output ./src/generated",
    "dev": "vite",
    "build": "vue-tsc && vite build"
  }
}
```

Generate the client:
```bash
npm run generate-client
```

This creates:
- `src/generated/types.ts` - TypeScript type definitions
- `src/generated/composables.ts` - Vue composables for API calls
- `src/generated/store.ts` - Pinia store for state management

## Step 6: Configure Vue App

### Option A: Using Generated Composables

If you generated the client, use the composables:

**`src/App.vue`:**
```vue
<template>
  <div class="app">
    <h1>User Management</h1>
    
    <form @submit.prevent="handleSubmit">
      <input
        v-model="formData.name"
        type="text"
        placeholder="Name"
        required
      />
      <input
        v-model="formData.email"
        type="email"
        placeholder="Email"
        required
      />
      <button type="submit" :disabled="creating">
        {{ creating ? 'Creating...' : 'Create User' }}
      </button>
    </form>

    <div v-if="loading">Loading users...</div>
    <div v-else-if="error">Error: {{ error.message }}</div>
    <ul v-else>
      <li v-for="user in users" :key="user.id">
        <strong>{{ user.name }}</strong> - {{ user.email }}
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useGetUsers, useCreateUser } from './generated/composables';
import type { UserInput } from './generated/types';

const { data: users, loading, error, refetch } = useGetUsers();
const { execute: createUser, loading: creating } = useCreateUser();

const formData = ref<UserInput>({
  name: '',
  email: ''
});

const handleSubmit = async () => {
  try {
    await createUser(formData.value);
    formData.value = { name: '', email: '' };
    refetch(); // Refresh user list
  } catch (error) {
    console.error('Failed to create user:', error);
  }
};
</script>

<style scoped>
.app {
  max-width: 800px;
  margin: 0 auto;
  padding: 20px;
}

form {
  margin-bottom: 20px;
}

input {
  margin-right: 10px;
  padding: 8px;
}

button {
  padding: 8px 16px;
  cursor: pointer;
}

ul {
  list-style: none;
  padding: 0;
}

li {
  padding: 10px;
  margin: 5px 0;
  background: #f5f5f5;
  border-radius: 4px;
}
</style>
```

### Option B: Manual Implementation with Pinia Store

Create a Pinia store for user management:

**`src/stores/userStore.ts`:**
```typescript
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';

interface User {
  id: string;
  name: string;
  email: string;
  createdAt: string;
}

interface UserInput {
  name: string;
  email: string;
}

const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000';

export const useUserStore = defineStore('users', () => {
  const users = ref<User[]>([]);
  const loading = ref(false);
  const error = ref<Error | null>(null);

  const userCount = computed(() => users.value.length);

  async function fetchUsers() {
    loading.value = true;
    error.value = null;
    try {
      const response = await fetch(`${API_URL}/users`);
      if (!response.ok) throw new Error('Failed to fetch users');
      users.value = await response.json();
    } catch (e) {
      error.value = e as Error;
      console.error('Error fetching users:', e);
    } finally {
      loading.value = false;
    }
  }

  async function createUser(input: UserInput) {
    loading.value = true;
    error.value = null;
    try {
      const response = await fetch(`${API_URL}/users`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(input)
      });
      if (!response.ok) throw new Error('Failed to create user');
      const newUser = await response.json();
      users.value.push(newUser);
    } catch (e) {
      error.value = e as Error;
      console.error('Error creating user:', e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  return {
    users,
    loading,
    error,
    userCount,
    fetchUsers,
    createUser
  };
});
```

Use the store in your component:

**`src/App.vue`:**
```vue
<template>
  <div class="app">
    <h1>User Management</h1>
    
    <form @submit.prevent="handleSubmit">
      <input
        v-model="formData.name"
        type="text"
        placeholder="Name"
        required
      />
      <input
        v-model="formData.email"
        type="email"
        placeholder="Email"
        required
      />
      <button type="submit" :disabled="userStore.loading">
        {{ userStore.loading ? 'Creating...' : 'Create User' }}
      </button>
    </form>

    <div v-if="userStore.loading && userStore.users.length === 0">
      Loading users...
    </div>
    <div v-else-if="userStore.error">
      Error: {{ userStore.error.message }}
    </div>
    <ul v-else>
      <li v-for="user in userStore.users" :key="user.id">
        <strong>{{ user.name }}</strong> - {{ user.email }}
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useUserStore } from './stores/userStore';

const userStore = useUserStore();
const formData = ref({ name: '', email: '' });

onMounted(() => {
  userStore.fetchUsers();
});

const handleSubmit = async () => {
  try {
    await userStore.createUser(formData.value);
    formData.value = { name: '', email: '' };
  } catch (error) {
    // Error already handled in store
  }
};
</script>
```

## Step 7: Configure API Base URL

Set the API URL as an environment variable:

**`.env.development`:**
```
VITE_API_URL=http://localhost:3000
```

**`.env.production`:**
```
VITE_API_URL=https://api.yourdomain.com
```

## Step 8: Start Vue App

```bash
# Terminal 2: Start Vue app
npm run dev
```

Your Vue app will be available at http://localhost:5173 (or next available port).

## Step 9: Test the Integration

1. **Create a user**: Fill out the form and submit
2. **View users**: See the list update with new users
3. **Monitor requests**: Open http://localhost:9080 (Admin UI) to see all requests

## Development Workflow

### Typical Development Cycle

1. **Start MockForge** with your API spec
2. **Develop Vue features** against mock data
3. **View requests** in Admin UI for debugging
4. **Update spec** as API evolves
5. **Regenerate client** when spec changes

### Updating API Spec

When the OpenAPI spec changes:

```bash
# Regenerate TypeScript client
npm run generate-client

# Restart MockForge with updated spec
# (Ctrl+C in Terminal 1, then restart)
mockforge serve --spec user-management-api.json --http-port 3000 --admin
```

### Testing with Vitest

Create tests against the mock server:

**`src/components/__tests__/UserForm.spec.ts`:**
```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { setActivePinia, createPinia } from 'pinia';
import UserForm from '../UserForm.vue';

describe('UserForm', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('creates a user', async () => {
    const wrapper = mount(UserForm);
    // Your test logic here
  });
});
```

## Common Issues

### CORS Errors

Enable CORS in MockForge config:

```yaml
# mockforge.yaml
http:
  port: 3000
  cors:
    enabled: true
    allowed_origins: ["http://localhost:5173", "http://localhost:3000"]
```

### Template Variables Not Expanding

Make sure template expansion is enabled:
```bash
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve ...
```

### Environment Variables Not Loading

Vite requires the `VITE_` prefix for environment variables. Ensure your `.env` file uses:
```
VITE_API_URL=http://localhost:3000
```

## Advanced Usage

### Reactive Data with Computed Properties

```vue
<script setup lang="ts">
import { computed } from 'vue';
import { useUserStore } from './stores/userStore';

const userStore = useUserStore();

const activeUsers = computed(() => 
  userStore.users.filter(u => !u.deleted)
);
</script>
```

### Error Handling with Vue Toast

```typescript
import { useToast } from 'vue-toastification';

const toast = useToast();

async function createUser(input: UserInput) {
  try {
    await userStore.createUser(input);
    toast.success('User created successfully!');
  } catch (error) {
    toast.error('Failed to create user');
  }
}
```

## Next Steps

- **View Complete Example**: See [Vue Demo](../../examples/vue-demo/) for a full implementation
- **Learn React Workflow**: [React + MockForge Workflow](react-workflow.md)
- **Explore Admin UI**: [Admin UI Walkthrough](admin-ui-walkthrough.md)
- **Advanced Features**: [Dynamic Data Generation](../user-guide/http-mocking/dynamic-data.md)

---

**Need help?** Check the [FAQ](../reference/faq.md) or [Troubleshooting Guide](../reference/troubleshooting.md).

