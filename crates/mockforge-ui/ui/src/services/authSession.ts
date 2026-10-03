import type { AuthState } from '../types';

/** A browser with no session cookies is signed out, rather than a refresh failure. */
export function isMissingRefreshCookie(state: AuthState, error: unknown): boolean {
  return !state.token && !state.refreshToken && !state.isAuthenticated
    && error instanceof Error && error.message === 'Missing refresh token';
}
