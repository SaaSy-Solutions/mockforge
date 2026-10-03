import type { User } from '../types';

/**
 * Centralized access-token storage.
 *
 * Every part of the app MUST read and write the auth JWT through this module
 * instead of touching `localStorage` directly.
 *
 * STORAGE MODEL (current): the JWT is held in MEMORY ONLY. Authentication
 * across reloads rides on the HttpOnly `mockforge_session` /
 * `mockforge_refresh` cookies that the registry sets on login/register/
 * refresh — those are invisible to JavaScript, so XSS in the bundle can no
 * longer exfiltrate a long-lived credential. Session restore after a reload
 * happens via `/api/v1/auth/me` in `useAuthStore.checkAuth()`.
 *
 */

let memoryToken: string | null = null;

/** Read the current in-memory auth JWT, or null when logged out. */
export function getAuthToken(): string | null {
  return memoryToken;
}

/** Hold the auth JWT for this page session only (login/register/refresh). */
export function setAuthToken(token: string): void {
  memoryToken = token;
}

/** Clear the page's token. Server logout belongs to the explicit logout action. */
export function clearAuthToken(): void {
  memoryToken = null;
}

// Parse JWT token to extract user info (client-side validation only)
export const parseToken = (token: string): { user: User | null; expiresAt: number | null } => {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return { user: null, expiresAt: null };

    const payload = JSON.parse(atob(parts[1].replace(/-/g, '+').replace(/_/g, '/'))); // JWT payload is base64url encoded

    // Check expiration
    if (typeof payload.sub !== 'string' || !Number.isFinite(payload.exp)) {
      return { user: null, expiresAt: null };
    }
    const expiresAt = payload.exp * 1000; // Convert to milliseconds
    if (expiresAt < Date.now()) {
      return { user: null, expiresAt: null };
    }

    // Extract what we can from the token (registry JWT may only have sub)
    const user: User = {
      id: payload.sub,
      username: payload.username || '',
      email: payload.email || '',
      role: payload.role || 'user',
    };

    return { user, expiresAt };
  } catch {
    return { user: null, expiresAt: null };
  }
};
