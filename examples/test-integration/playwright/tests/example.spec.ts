import { test, expect } from '@playwright/test';

/**
 * Example Playwright tests using MockForge
 */

test.describe('MockForge Integration', () => {
  test('health check endpoint returns healthy status', async ({ request }) => {
    const response = await request.get('/health');
    expect(response.ok()).toBeTruthy();

    const health = await response.json();
    expect(health.status).toBe('healthy');
    expect(health.version).toBeTruthy();
  });

  test('can fetch mock data from API', async ({ request }) => {
    const response = await request.get('/api/users');
    expect(response.ok()).toBeTruthy();

    const users = await response.json();
    expect(Array.isArray(users)).toBeTruthy();
  });

  test('can switch scenarios via management API', async ({ request }) => {
    // Switch to a different scenario
    const switchResponse = await request.post('/__mockforge/workspace/switch', {
      data: {
        workspace: 'test-scenario',
      },
    });
    expect(switchResponse.ok()).toBeTruthy();

    // Verify the scenario changed
    const statsResponse = await request.get('/__mockforge/stats');
    expect(statsResponse.ok()).toBeTruthy();
  });

  test('can update mocks dynamically', async ({ request }) => {
    // Update a mock endpoint
    const updateResponse = await request.post('/__mockforge/config/api/users/1', {
      data: {
        id: 1,
        name: 'Test User',
        email: 'test@example.com',
      },
    });
    expect(updateResponse.ok()).toBeTruthy();

    // Fetch the updated mock
    const getResponse = await request.get('/api/users/1');
    expect(getResponse.ok()).toBeTruthy();

    const user = await getResponse.json();
    expect(user.name).toBe('Test User');
  });

  test('can reset mocks to initial state', async ({ request }) => {
    // Make some changes
    await request.post('/__mockforge/config/api/test', {
      data: { test: true },
    });

    // Reset mocks
    const resetResponse = await request.post('/__mockforge/reset');
    expect(resetResponse.ok()).toBeTruthy();

    // Verify mocks were reset
    const statsResponse = await request.get('/__mockforge/stats');
    expect(statsResponse.ok()).toBeTruthy();
  });
});

test.describe('User Authentication Scenarios', () => {
  test('authenticated user can access protected endpoint', async ({ request }) => {
    // Switch to authenticated scenario
    await request.post('/__mockforge/workspace/switch', {
      data: { workspace: 'user-authenticated' },
    });

    // Access protected endpoint
    const response = await request.get('/api/protected/profile');
    expect(response.ok()).toBeTruthy();

    const profile = await response.json();
    expect(profile.authenticated).toBe(true);
  });

  test('unauthenticated user gets 401', async ({ request }) => {
    // Switch to unauthenticated scenario
    await request.post('/__mockforge/workspace/switch', {
      data: { workspace: 'user-unauthenticated' },
    });

    // Try to access protected endpoint
    const response = await request.get('/api/protected/profile');
    expect(response.status()).toBe(401);
  });
});

test.describe('Error Handling Scenarios', () => {
  test('server error scenario returns 500', async ({ request }) => {
    // Switch to error scenario
    await request.post('/__mockforge/workspace/switch', {
      data: { workspace: 'server-errors' },
    });

    // API should return errors
    const response = await request.get('/api/users');
    expect(response.status()).toBe(500);
  });

  test('network timeout scenario', async ({ request }) => {
    // Switch to slow response scenario
    await request.post('/__mockforge/workspace/switch', {
      data: { workspace: 'slow-responses' },
    });

    // This request should be slow but eventually succeed
    const startTime = Date.now();
    const response = await request.get('/api/slow');
    const duration = Date.now() - startTime;

    expect(response.ok()).toBeTruthy();
    expect(duration).toBeGreaterThan(1000); // Should take at least 1 second
  });
});
