/**
 * Example Vitest tests using MockForge
 */

import { describe, it, expect, beforeEach } from 'vitest';

const BASE_URL = process.env.MOCKFORGE_URL || 'http://localhost:3000';

describe('MockForge Integration', () => {
  beforeEach(async () => {
    // Reset mocks before each test
    await fetch(`${BASE_URL}/__mockforge/reset`, { method: 'POST' });
  });

  it('health check endpoint returns healthy status', async () => {
    const response = await fetch(`${BASE_URL}/health`);
    expect(response.ok).toBe(true);

    const health = await response.json();
    expect(health.status).toBe('healthy');
    expect(health.version).toBeTruthy();
  });

  it('can fetch mock data from API', async () => {
    const response = await fetch(`${BASE_URL}/api/users`);
    expect(response.ok).toBe(true);

    const users = await response.json();
    expect(Array.isArray(users)).toBe(true);
  });

  it('can switch scenarios via management API', async () => {
    const switchResponse = await fetch(`${BASE_URL}/__mockforge/workspace/switch`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workspace: 'test-scenario' }),
    });
    expect(switchResponse.ok).toBe(true);

    const statsResponse = await fetch(`${BASE_URL}/__mockforge/stats`);
    expect(statsResponse.ok).toBe(true);
  });

  it('can update mocks dynamically', async () => {
    // Update a mock endpoint
    const updateResponse = await fetch(`${BASE_URL}/__mockforge/config/api/users/1`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        id: 1,
        name: 'Test User',
        email: 'test@example.com',
      }),
    });
    expect(updateResponse.ok).toBe(true);

    // Fetch the updated mock
    const getResponse = await fetch(`${BASE_URL}/api/users/1`);
    expect(getResponse.ok).toBe(true);

    const user = await getResponse.json();
    expect(user.name).toBe('Test User');
  });

  it('can get server statistics', async () => {
    const response = await fetch(`${BASE_URL}/__mockforge/stats`);
    expect(response.ok).toBe(true);

    const stats = await response.json();
    expect(stats).toBeTruthy();
  });

  it('can list available fixtures', async () => {
    const response = await fetch(`${BASE_URL}/__mockforge/fixtures`);
    expect(response.ok).toBe(true);

    const fixtures = await response.json();
    expect(Array.isArray(fixtures)).toBe(true);
  });
});

describe('User Authentication Scenarios', () => {
  it('authenticated user can access protected endpoint', async () => {
    // Switch to authenticated scenario
    await fetch(`${BASE_URL}/__mockforge/workspace/switch`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workspace: 'user-authenticated' }),
    });

    // Access protected endpoint
    const response = await fetch(`${BASE_URL}/api/protected/profile`);
    expect(response.ok).toBe(true);

    const profile = await response.json();
    expect(profile.authenticated).toBe(true);
  });

  it('unauthenticated user gets 401', async () => {
    // Switch to unauthenticated scenario
    await fetch(`${BASE_URL}/__mockforge/workspace/switch`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workspace: 'user-unauthenticated' }),
    });

    // Try to access protected endpoint
    const response = await fetch(`${BASE_URL}/api/protected/profile`);
    expect(response.status).toBe(401);
  });
});

describe('Error Handling Scenarios', () => {
  it('server error scenario returns 500', async () => {
    // Switch to error scenario
    await fetch(`${BASE_URL}/__mockforge/workspace/switch`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workspace: 'server-errors' }),
    });

    // API should return errors
    const response = await fetch(`${BASE_URL}/api/users`);
    expect(response.status).toBe(500);
  });

  it('network timeout scenario', async () => {
    // Switch to slow response scenario
    await fetch(`${BASE_URL}/__mockforge/workspace/switch`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workspace: 'slow-responses' }),
    });

    // This request should be slow but eventually succeed
    const startTime = Date.now();
    const response = await fetch(`${BASE_URL}/api/slow`);
    const duration = Date.now() - startTime;

    expect(response.ok).toBe(true);
    expect(duration).toBeGreaterThan(1000); // Should take at least 1 second
  });
});

describe('Data Validation', () => {
  it('validates request data', async () => {
    const response = await fetch(`${BASE_URL}/api/users`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: 'John Doe',
        email: 'john@example.com',
      }),
    });

    expect(response.ok).toBe(true);
    const user = await response.json();
    expect(user.name).toBe('John Doe');
    expect(user.email).toBe('john@example.com');
  });

  it('rejects invalid data', async () => {
    const response = await fetch(`${BASE_URL}/api/users`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        // Missing required fields
        invalid: 'data',
      }),
    });

    expect(response.status).toBe(400);
  });
});
