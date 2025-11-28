# React + MockForge Workflow

**Goal**: Build a React application that uses MockForge as a backend mock server for development and testing.

**Time**: 10-15 minutes

## Overview

This tutorial shows you how to:
1. Set up MockForge with an OpenAPI specification
2. Generate TypeScript client code for React
3. Build a React app that consumes the mock API
4. Develop and test frontend features against mock data

## Prerequisites

- MockForge installed ([Installation Guide](../getting-started/installation.md))
- Node.js 16+ and npm/pnpm installed
- Basic React and TypeScript knowledge

## Step 1: Prepare Your OpenAPI Specification

Create or use an existing OpenAPI spec. For this tutorial, we'll use a User Management API:

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

**Tip**: Keep this terminal running. The `--admin` flag enables the admin UI at http://localhost:9080 for monitoring requests.

## Step 3: Create React Application

Create a new React app (or use an existing one):

```bash
# Create React app with TypeScript
npx create-react-app my-app --template typescript
cd my-app
```

## Step 4: Generate TypeScript Client (Optional)

MockForge can generate type-safe React hooks from your OpenAPI spec:

```bash
# Install MockForge CLI as dev dependency
npm install --save-dev mockforge-cli

# Add to package.json scripts
```

Update `package.json`:

```json
{
  "scripts": {
    "generate-client": "mockforge client generate --spec ../user-management-api.json --framework react --output ./src/generated",
    "start": "react-scripts start",
    "build": "react-scripts build"
  }
}
```

Generate the client:
```bash
npm run generate-client
```

This creates:
- `src/generated/types.ts` - TypeScript type definitions
- `src/generated/hooks.ts` - React hooks for API calls

## Step 5: Configure React App

### Option A: Using Generated Hooks

If you generated the client, use the hooks:

**`src/App.tsx`:**
```typescript
import React, { useState } from 'react';
import { useGetUsers, useCreateUser } from './generated/hooks';
import type { UserInput } from './generated/types';

function App() {
  const { data: users, loading, error, refetch } = useGetUsers();
  const { execute: createUser, loading: creating } = useCreateUser();
  
  const [formData, setFormData] = useState<UserInput>({
    name: '',
    email: ''
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await createUser(formData);
      setFormData({ name: '', email: '' });
      refetch(); // Refresh user list
    } catch (error) {
      console.error('Failed to create user:', error);
    }
  };

  if (loading) return <div>Loading users...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div className="App">
      <h1>User Management</h1>
      
      <form onSubmit={handleSubmit}>
        <input
          type="text"
          placeholder="Name"
          value={formData.name}
          onChange={(e) => setFormData({ ...formData, name: e.target.value })}
        />
        <input
          type="email"
          placeholder="Email"
          value={formData.email}
          onChange={(e) => setFormData({ ...formData, email: e.target.value })}
        />
        <button type="submit" disabled={creating}>
          {creating ? 'Creating...' : 'Create User'}
        </button>
      </form>

      <ul>
        {users?.map(user => (
          <li key={user.id}>
            <strong>{user.name}</strong> - {user.email}
          </li>
        ))}
      </ul>
    </div>
  );
}

export default App;
```

### Option B: Manual Fetch Implementation

If you prefer manual implementation:

**`src/App.tsx`:**
```typescript
import React, { useState, useEffect } from 'react';

interface User {
  id: string;
  name: string;
  email: string;
  createdAt: string;
}

function App() {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [formData, setFormData] = useState({ name: '', email: '' });

  useEffect(() => {
    fetch('http://localhost:3000/users')
      .then(res => res.json())
      .then(data => {
        setUsers(data);
        setLoading(false);
      })
      .catch(err => {
        console.error('Error fetching users:', err);
        setLoading(false);
      });
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const res = await fetch('http://localhost:3000/users', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(formData)
      });
      const newUser = await res.json();
      setUsers([...users, newUser]);
      setFormData({ name: '', email: '' });
    } catch (error) {
      console.error('Failed to create user:', error);
    }
  };

  if (loading) return <div>Loading...</div>;

  return (
    <div className="App">
      <h1>User Management</h1>
      
      <form onSubmit={handleSubmit}>
        <input
          type="text"
          placeholder="Name"
          value={formData.name}
          onChange={(e) => setFormData({ ...formData, name: e.target.value })}
        />
        <input
          type="email"
          placeholder="Email"
          value={formData.email}
          onChange={(e) => setFormData({ ...formData, email: e.target.value })}
        />
        <button type="submit">Create User</button>
      </form>

      <ul>
        {users.map(user => (
          <li key={user.id}>
            <strong>{user.name}</strong> - {user.email}
          </li>
        ))}
      </ul>
    </div>
  );
}

export default App;
```

## Step 6: Configure API Base URL

Set the API URL as an environment variable:

**`.env.development`:**
```
REACT_APP_API_URL=http://localhost:3000
```

**`.env.production`:**
```
REACT_APP_API_URL=https://api.yourdomain.com
```

Update your fetch calls to use the environment variable:
```typescript
const API_URL = process.env.REACT_APP_API_URL || 'http://localhost:3000';

fetch(`${API_URL}/users`)
```

## Step 7: Start React App

```bash
# Terminal 2: Start React app
npm start
```

Your React app will be available at http://localhost:3001 (or 3000 if available).

## Step 8: Test the Integration

1. **Create a user**: Fill out the form and submit
2. **View users**: See the list update with new users
3. **Monitor requests**: Open http://localhost:9080 (Admin UI) to see all requests

## Development Workflow

### Typical Development Cycle

1. **Start MockForge** with your API spec
2. **Develop React features** against mock data
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

### Testing

Run tests against the mock server:

```bash
# Start mock server in background
mockforge serve --spec user-management-api.json --http-port 3000 &
MOCKFORGE_PID=$!

# Run tests
npm test

# Stop mock server
kill $MOCKFORGE_PID
```

## Common Issues

### CORS Errors

If you see CORS errors, enable CORS in MockForge config:

```yaml
# mockforge.yaml
http:
  port: 3000
  cors:
    enabled: true
    allowed_origins: ["http://localhost:3000", "http://localhost:3001"]
```

### Template Variables Not Expanding

Make sure template expansion is enabled:
```bash
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve ...
```

### Client Generation Fails

- Ensure MockForge CLI is in PATH
- Check OpenAPI spec is valid JSON/YAML
- Verify framework name is correct (`react`, not `reactjs`)

## Advanced Usage

### Custom Hooks

Wrap generated hooks with custom logic:

```typescript
import { useGetUsers as useGetUsersBase } from './generated/hooks';

export function useGetUsers() {
  const result = useGetUsersBase();
  
  // Add custom logic
  useEffect(() => {
    if (result.data) {
      console.log('Users loaded:', result.data.length);
    }
  }, [result.data]);
  
  return result;
}
```

### Error Handling

Implement global error handling:

```typescript
import { useGetUsers, useCreateUser } from './generated/hooks';

function App() {
  const { data, error } = useGetUsers();
  
  if (error) {
    // Show user-friendly error message
    return <ErrorDisplay error={error} />;
  }
  
  // ... rest of component
}
```

### Request Interceptors

Add authentication or custom headers:

```typescript
// In generated/hooks.ts, modify the base configuration
const apiConfig = {
  baseUrl: 'http://localhost:3000',
  headers: {
    'Authorization': `Bearer ${getToken()}`,
  }
};
```

## Next Steps

- **View Complete Example**: See [React Demo](../../examples/react-demo/) for a full implementation
- **Learn Vue Workflow**: [Vue + MockForge Workflow](vue-workflow.md)
- **Explore Admin UI**: [Admin UI Walkthrough](admin-ui-walkthrough.md)
- **Advanced Features**: [Dynamic Data Generation](../user-guide/http-mocking/dynamic-data.md)

---

**Need help?** Check the [FAQ](../reference/faq.md) or [Troubleshooting Guide](../reference/troubleshooting.md).

