import type { AuthState, User } from '../types';

/** A browser with no session cookies is signed out, rather than a refresh failure. */
export function isMissingRefreshCookie(state: AuthState, error: unknown): boolean {
  return !state.token && !state.refreshToken && !state.isAuthenticated
    && error instanceof Error && error.message === 'Missing refresh token';
}

/** Prefer the server identity, preserving a hydrated profile only for the same subject. */
export function resolveRefreshedUser(serverUser: User | undefined, currentUser: User | null, tokenUser: User | null): User | null {
  return serverUser ?? (currentUser?.id === tokenUser?.id ? currentUser : tokenUser);
}
