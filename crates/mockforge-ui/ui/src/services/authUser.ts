import { logger } from '@/utils/logger';
import type { User } from '../types';
import { authApi } from './authApi';

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
