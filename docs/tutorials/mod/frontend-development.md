# MOD for Frontend Development

**Pillars:** [DevX][Reality]

**Duration:** 20 minutes
**Prerequisites:** MOD Getting Started, Frontend development experience

## Overview

This tutorial shows frontend developers how to use MOD to build UIs without waiting for backend APIs.

## The Frontend Developer's Dilemma

**Problem:** Frontend developers are blocked waiting for backend APIs.

**MOD Solution:** Start building immediately with realistic mocks.

## Step 1: Get API Contracts

**Option A: Backend team provides contracts**

```bash
# Backend team shares contracts
git clone https://github.com/company/api-contracts.git
cd api-contracts
```

**Option B: Define contracts yourself**

```yaml
# contracts/api.yaml
openapi: 3.0.0
paths:
  /api/users/{id}:
    get:
      responses:
        '200':
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
```

## Step 2: Generate Mock Server

```bash
# Generate mock from contract
mockforge generate --from-openapi contracts/api.yaml --output mocks/

# Start mock server
mockforge serve --config mockforge.yaml --admin
```

## Step 3: Configure Frontend

```typescript
// src/config/api.ts
const API_BASE = process.env.NODE_ENV === 'development'
  ? 'http://localhost:3000'  // Mock server
  : process.env.REACT_APP_API_URL;  // Real API

export const apiConfig = {
  baseURL: API_BASE,
  timeout: 5000,
};
```

## Step 4: Build Frontend with Mock

```typescript
// src/services/userService.ts
import { apiConfig } from '../config/api';

export interface User {
  id: string;
  name: string;
  email: string;
}

export async function getUser(id: string): Promise<User> {
  const response = await fetch(`${apiConfig.baseURL}/api/users/${id}`);
  if (!response.ok) {
    throw new Error('Failed to fetch user');
  }
  return response.json();
}

export async function getUsers(): Promise<User[]> {
  const response = await fetch(`${apiConfig.baseURL}/api/users`);
  if (!response.ok) {
    throw new Error('Failed to fetch users');
  }
  return response.json();
}
```

```tsx
// src/components/UserList.tsx
import React, { useEffect, useState } from 'react';
import { getUser, getUsers, User } from '../services/userService';

export function UserList() {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadUsers() {
      try {
        const data = await getUsers();
        setUsers(data);
      } catch (error) {
        console.error('Failed to load users', error);
      } finally {
        setLoading(false);
      }
    }
    loadUsers();
  }, []);

  if (loading) {
    return <div>Loading...</div>;
  }

  return (
    <div>
      <h1>Users</h1>
      <ul>
        {users.map(user => (
          <li key={user.id}>
            {user.name} ({user.email})
          </li>
        ))}
      </ul>
    </div>
  );
}
```

## Step 5: Use Smart Personas for Consistency

```yaml
# mockforge.yaml
reality:
  personas:
    enabled: true
    personas:
      - name: "alice"
        domain: "general"
        traits:
          name: "Alice"
          email: "alice@example.com"
```

Now all endpoints return consistent data for the same user ID.

## Step 6: Test with Realistic Scenarios

```typescript
// src/__tests__/UserList.test.tsx
import { render, screen, waitFor } from '@testing-library/react';
import { UserList } from '../components/UserList';

// Mock server running at http://localhost:3000
test('displays user list', async () => {
  render(<UserList />);

  await waitFor(() => {
    expect(screen.getByText('Alice')).toBeInTheDocument();
  });
});
```

## Step 7: Switch to Real API

When backend is ready:

```bash
# Update environment variable
export REACT_APP_API_URL=https://api.example.com

# Frontend automatically uses real API
npm start
```

Or use Reality Continuum for gradual transition:

```yaml
# mockforge.yaml
reality:
  continuum:
    enabled: true
    blend_ratio: 0.5  # 50% mock, 50% real
    upstream_url: https://api.example.com
```

## Frontend MOD Workflow

### Daily Workflow

1. **Start Mock Server**
   ```bash
   mockforge serve --config mockforge.yaml
   ```

2. **Develop Frontend**
   - Build UI components
   - Test with mock data
   - Iterate quickly

3. **Test Integration**
   - Run tests against mock
   - Validate UI behavior
   - Check error handling

4. **Switch to Real API**
   - Update environment
   - Test with real API
   - Fix any issues

### When Backend Changes

1. **Get Updated Contract**
   ```bash
   git pull origin main  # Get latest contracts
   ```

2. **Regenerate Mock**
   ```bash
   mockforge generate --from-openapi contracts/api.yaml
   ```

3. **Update Frontend**
   - Review contract changes
   - Update frontend code
   - Test with new mock

## Tips for Frontend Developers

### 1. Use Mock Data for Development

✅ **Do:**
- Use mock server for local development
- Test with realistic data
- Validate UI behavior

❌ **Don't:**
- Hardcode test data
- Skip API integration
- Ignore error cases

### 2. Test Error Scenarios

```typescript
// Test error handling
test('handles 404 error', async () => {
  // Mock server can return 404
  const response = await fetch('http://localhost:3000/api/users/invalid');
  expect(response.status).toBe(404);
});
```

### 3. Use Personas for Consistency

```yaml
# Same user ID always returns same data
personas:
  - id: "user_123"
    traits:
      name: "Alice"
      email: "alice@example.com"
```

### 4. Test Loading States

```typescript
// Mock server can simulate latency
reality:
  level: 3
  latency:
    enabled: true
    base_ms: 500  # 500ms delay
```

## Common Frontend MOD Patterns

### Pattern: Environment-Based API Selection

```typescript
const getApiBase = () => {
  if (process.env.NODE_ENV === 'development') {
    return 'http://localhost:3000';  // Mock
  }
  return process.env.REACT_APP_API_URL;  // Real
};
```

### Pattern: Mock Toggle

```typescript
// Toggle between mock and real API
const USE_MOCK = process.env.REACT_APP_USE_MOCK === 'true';

const API_BASE = USE_MOCK
  ? 'http://localhost:3000'
  : 'https://api.example.com';
```

### Pattern: Contract Validation

```typescript
// Validate responses against contract
import { validateAgainstSchema } from './contract-validator';

async function getUser(id: string) {
  const response = await fetch(`${API_BASE}/api/users/${id}`);
  const data = await response.json();

  // Validate against OpenAPI schema
  validateAgainstSchema(data, userSchema);

  return data;
}
```

## Troubleshooting

### Problem: Mock responses don't match real API

**Solution:**
- Use Reality Continuum
- Record real API responses
- Update mock based on real behavior

### Problem: Frontend works with mock, breaks with real API

**Solution:**
- Increase mock realism
- Test with Reality Continuum
- Validate against contract

### Problem: Mock server too slow

**Solution:**
- Reduce reality level
- Disable unnecessary features
- Use static responses

## Further Reading

- [MOD Guide](../../MOD_GUIDE.md) — Complete workflow
- [MOD Patterns](../../MOD_PATTERNS.md) — Advanced patterns
- [Reality Continuum](../../REALITY_CONTINUUM.md) — Blending mock and real

---

**Frontend developers: Start building immediately with MOD. Don't wait for backends—mock them!**
