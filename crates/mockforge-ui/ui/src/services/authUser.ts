import { logger } from '@/utils/logger';
import type { User } from '../types';
import { authApi } from './authApi';

/**
 * Hydrate cloud-mode-only fields (role, is_verified, email, created_at) from
 * `/api/v1/users/me`. The login response and JWT only carry user_id+username,
 * so without this admin users would be misclassified as `role: 'user'` and
 * RoleGuard would deny them admin features.
 */
export async function hydrateUserFromServer(base: User): Promise<User> {
  if (!authApi.isCloud()) return base;
  try {
    const profile = await authApi.getMe();
    return {
      ...base,
      id: profile.user_id,
      username: profile.username,
      email: profile.email || base.email,
      role: profile.is_admin ? 'admin' : (base.role === 'viewer' ? 'viewer' : 'user'),
      is_verified: profile.is_verified,
      created_at: profile.created_at,
    };
  } catch (error) {
    logger.warn('Failed to hydrate user profile from server', error);
    return base;
  }
}

/** Restore the cookie identity; rejected or unavailable cookies are a normal signed-out result. */
export async function readCookieUser(existingUser: User | null): Promise<User | null> {
  try {
    const response = await fetch('/api/v1/auth/me', { credentials: 'include' });
    if (!response.ok) return null;
    const me = await response.json();
    return {
      ...existingUser,
      id: me.user_id,
      username: me.username,
      email: me.email,
      is_verified: me.is_verified,
      role: me.is_admin ? 'admin' : 'user',
    };
  } catch {
    return null;
  }
}
