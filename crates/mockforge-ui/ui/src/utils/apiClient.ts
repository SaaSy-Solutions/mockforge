// API client with automatic JWT token injection
// Intercepts all fetch requests to add Authorization header

import { useAuthStore } from '../stores/useAuthStore';

// Store the original fetch function
const originalFetch = globalThis.fetch;

// Create a fetch wrapper that adds JWT tokens
export function createAuthenticatedFetch() {
  return async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const state = useAuthStore.getState();
    const token = state.token;

    // Clone the init object to avoid mutating the original
    const headers = new Headers(init?.headers);

    // Add Authorization header if token exists
    if (token) {
      headers.set('Authorization', `Bearer ${token}`);
    }

    // Create new init with updated headers
    const newInit: RequestInit = {
      ...init,
      headers,
    };

    // Make the request
    let response = await originalFetch(input, newInit);

    // Handle 401 Unauthorized - token might be expired
    if (response.status === 401 && token) {
      try {
        // Try to refresh the token
        await state.refreshTokenAction();

        // Retry the request with new token
        const newToken = useAuthStore.getState().token;
        if (newToken) {
          headers.set('Authorization', `Bearer ${newToken}`);
          response = await originalFetch(input, { ...newInit, headers });
        } else {
          // Refresh failed, logout
          state.logout();
        }
      } catch (error) {
        // Refresh failed, logout
        state.logout();
      }
    }

    return response;
  };
}

// Export a fetch function that can be used throughout the app
export const authenticatedFetch = createAuthenticatedFetch();
