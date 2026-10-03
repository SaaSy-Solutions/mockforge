/**
 * @jest-environment jsdom
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { logger } from '@/utils/logger';
import { useAuthStore } from '../useAuthStore';
import { authApi } from '../../services/authApi';
import type { User } from '../../types';

vi.mock('../../services/authApi', () => ({
  authApi: {
    login: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    isCloud: vi.fn(() => false),
    updateProfile: vi.fn(),
    getMe: vi.fn(),
  },
}));

const createToken = (user: User, expiresInSeconds = 3600) => {
  const payload = {
    sub: user.id,
    username: user.username,
    email: user.email,
    role: user.role,
    exp: Math.floor(Date.now() / 1000) + expiresInSeconds,
  };
  return `${btoa('header')}.${btoa(JSON.stringify(payload))}.${btoa('signature')}`;
};

describe('useAuthStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(authApi.isCloud).mockReturnValue(false);
    useAuthStore.getState().stopTokenRefresh();

    // Reset store state
    useAuthStore.setState({
      user: null,
      isAuthenticated: false,
      isLoading: false,
      token: null,
      refreshToken: null,
    });

    // Clear localStorage
    localStorage.clear();

    const adminUser: User = {
      id: 'admin-001',
      username: 'admin',
      email: 'admin@mockforge.dev',
      role: 'admin',
    };
    const viewerUser: User = {
      id: 'viewer-001',
      username: 'viewer',
      email: 'viewer@mockforge.dev',
      role: 'viewer',
    };

    vi.mocked(authApi.login).mockImplementation(async (username: string, password: string) => {
      if (username === 'admin' && password === 'admin123') {
        const token = createToken(adminUser);
        return { token, refresh_token: `refresh_${token}`, user: adminUser, expires_in: 3600 };
      }
      if (username === 'viewer' && password === 'viewer123') {
        const token = createToken(viewerUser);
        return { token, refresh_token: `refresh_${token}`, user: viewerUser, expires_in: 3600 };
      }
      throw new Error('Invalid username or password');
    });
    vi.mocked(authApi.logout).mockResolvedValue(undefined);
    vi.mocked(authApi.refreshToken).mockImplementation(async (refreshToken?: string) => ({
      token: createToken(adminUser),
      refresh_token: refreshToken ?? 'cookie-refresh',
      user: adminUser,
      expires_in: 3600,
    }));
  });

  afterEach(() => {
    useAuthStore.getState().stopTokenRefresh();
    vi.restoreAllMocks();
  });

  it('initializes with default state', () => {
    const { result } = renderHook(() => useAuthStore());

    expect(result.current.user).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.token).toBeNull();
  });

  it('handles successful login with admin user', async () => {
    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.login('admin', 'admin123');
    });

    expect(result.current.user).toMatchObject({
      id: 'admin-001',
      username: 'admin',
      role: 'admin',
      email: 'admin@mockforge.dev',
    });
    expect(result.current.isAuthenticated).toBe(true);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.token).toBeTruthy();
    expect(result.current.token).toContain('.');
  });

  it('handles successful login with viewer user', async () => {
    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.login('viewer', 'viewer123');
    });

    expect(result.current.user).toMatchObject({
      id: 'viewer-001',
      username: 'viewer',
      role: 'viewer',
      email: 'viewer@mockforge.dev',
    });
    expect(result.current.isAuthenticated).toBe(true);
  });

  it('hydrates admin role from /users/me in cloud mode (login response lacks is_admin)', async () => {
    vi.mocked(authApi.isCloud).mockReturnValue(true);
    // Cloud login response normalizes to role='user' since the backend
    // doesn't include is_admin/role in /api/v1/auth/login. The store must
    // call getMe() to discover that this user is actually an admin.
    vi.mocked(authApi.login).mockResolvedValueOnce({
      token: createToken({ id: 'u-1', username: 'rootuser', email: 'root@mockforge.dev', role: 'user' }),
      refresh_token: 'r',
      user: { id: 'u-1', username: 'rootuser', email: 'root@mockforge.dev', role: 'user' },
      expires_in: 3600,
    });
    vi.mocked(authApi.getMe).mockResolvedValueOnce({
      user_id: 'u-1',
      username: 'rootuser',
      email: 'root@mockforge.dev',
      is_verified: true,
      is_admin: true,
      two_factor_enabled: false,
      email_notifications: true,
      security_alerts: true,
      preferences: {},
      created_at: '2026-01-15T00:00:00Z',
      updated_at: '2026-01-15T00:00:00Z',
    });

    const { result } = renderHook(() => useAuthStore());
    await act(async () => {
      await result.current.login('rootuser', 'pw');
    });

    expect(authApi.getMe).toHaveBeenCalled();
    expect(result.current.user?.role).toBe('admin');
    expect(result.current.user?.is_verified).toBe(true);
    expect(result.current.user?.created_at).toBe('2026-01-15T00:00:00Z');
  });

  it('handles login failure with invalid credentials', async () => {
    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      try {
        await result.current.login('admin', 'wrongpassword');
      } catch (error) {
        expect(error).toBeDefined();
      }
    });

    expect(result.current.user).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
    expect(result.current.token).toBeNull();
  });

  it('handles login failure with non-existent user', async () => {
    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      try {
        await result.current.login('nonexistent', 'password');
      } catch (error) {
        expect(error instanceof Error ? error.message : String(error)).toContain('Invalid username or password');
      }
    });

    expect(result.current.user).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
  });

  it('sets loading state during login', async () => {
    const { result } = renderHook(() => useAuthStore());

    const loginPromise = act(async () => {
      await result.current.login('admin', 'admin123');
    });

    // Should eventually finish loading
    await loginPromise;
    expect(result.current.isLoading).toBe(false);
  });

  it('handles logout', async () => {
    const { result } = renderHook(() => useAuthStore());

    // First login
    await act(async () => {
      await result.current.login('admin', 'admin123');
    });

    expect(result.current.isAuthenticated).toBe(true);

    // Then logout
    await act(async () => {
      await result.current.logout();
    });

    expect(result.current.user).toBeNull();
    expect(result.current.token).toBeNull();
    expect(result.current.refreshToken).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
  });

  it('checks authentication status with valid token', async () => {
    const { result } = renderHook(() => useAuthStore());

    // Login first
    await act(async () => {
      await result.current.login('admin', 'admin123');
    });

    const token = result.current.token;

    // Reset state but keep token
    useAuthStore.setState({
      user: null,
      isAuthenticated: false,
      token,
      refreshToken: result.current.refreshToken,
    });

    // Check auth should restore user
    await act(async () => {
      await result.current.checkAuth();
    });

    expect(result.current.isAuthenticated).toBe(true);
    expect(result.current.user).toBeTruthy();
  });

  it('handles invalid token during auth check', async () => {
    const { result } = renderHook(() => useAuthStore());

    // Set invalid token
    useAuthStore.setState({
      token: 'invalid-token',
      refreshToken: null,
    });

    await act(async () => {
      await result.current.checkAuth();
    });

    expect(result.current.isAuthenticated).toBe(false);
    expect(result.current.user).toBeNull();
  });

  it('refreshes token', async () => {
    const { result } = renderHook(() => useAuthStore());

    // Login first
    await act(async () => {
      await result.current.login('admin', 'admin123');
    });

    expect(result.current.isAuthenticated).toBe(true);

    // Refresh token
    await act(async () => {
      await result.current.refreshTokenAction();
    });

    // Token should still exist and user should still be authenticated
    expect(result.current.token).toBeTruthy();
    expect(result.current.isAuthenticated).toBe(true);
    // Note: Token may be the same if generated in same second (iat/exp are in seconds)
    // The important thing is refresh works without error
  });

  it('handles token refresh failure when not authenticated', async () => {
    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      try {
        await result.current.refreshTokenAction();
      } catch (error) {
        expect(error instanceof Error ? error.message : String(error)).toContain('No refresh token available');
      }
    });
  });

  it('updates user profile', async () => {
    const { result } = renderHook(() => useAuthStore());

    // Login first
    await act(async () => {
      await result.current.login('admin', 'admin123');
    });

    const updatedUser: User = {
      id: 'admin-001',
      username: 'admin',
      email: 'newemail@example.com',
      role: 'admin',
    };

    await act(async () => {
      await result.current.updateProfile(updatedUser);
    });

    expect(result.current.user).toMatchObject(updatedUser);
    expect(result.current.user?.email).toBe('newemail@example.com');
  });

  it('validates token expiry correctly', () => {
    const { result } = renderHook(() => useAuthStore());

    // No token should return false
    expect(result.current.checkTokenExpiry()).toBe(false);
  });

  it('sets authenticated state directly', () => {
    const { result } = renderHook(() => useAuthStore());

    const user: User = {
      id: '1',
      username: 'testuser',
      email: 'test@example.com',
      role: 'admin',
    };
    const token = 'test-token';

    act(() => {
      result.current.setAuthenticated(user, token);
    });

    expect(result.current.user).toEqual(user);
    expect(result.current.token).toBe(token);
    expect(result.current.isAuthenticated).toBe(true);
    expect(result.current.refreshToken).toBeNull();
  });

  it('generates valid JWT-like tokens', async () => {
    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.login('admin', 'admin123');
    });

    const token = result.current.token;
    expect(token).toBeTruthy();
    expect(token).toContain('.');

    const parts = token?.split('.');
    expect(parts?.length).toBe(3);
  });
  it('does not send server logout when tokenless state changes', () => {
    useAuthStore.setState({ isLoading: true });
    useAuthStore.setState({ isLoading: false });
    expect(fetch).not.toHaveBeenCalled();
    expect(authApi.logout).not.toHaveBeenCalled();
  });

  it('restores a cookie session and admin role without logging out', async () => {
    vi.mocked(authApi.isCloud).mockReturnValue(true);
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify({
      user_id: 'cookie-user', username: 'root', email: 'root@example.com', is_admin: true,
    })));
    const user: User = { id: 'cookie-user', username: 'root', email: 'root@example.com', role: 'admin' };
    vi.mocked(authApi.refreshToken).mockResolvedValueOnce({ token: createToken(user), refresh_token: 'cookie-refresh', expires_in: 3600 });
    await useAuthStore.getState().checkAuth();
    expect(useAuthStore.getState()).toMatchObject({
      isAuthenticated: true, user: { id: 'cookie-user', role: 'admin' },
    });
    expect(authApi.logout).not.toHaveBeenCalled();
    expect(authApi.refreshToken).toHaveBeenCalledWith(undefined);
    expect(useAuthStore.getState().token).toBeTruthy();
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it('restores an expired access cookie using the HttpOnly refresh cookie', async () => {
    vi.mocked(authApi.isCloud).mockReturnValue(true);
    vi.mocked(fetch).mockResolvedValueOnce(new Response('{}', { status: 401 }));
    const user: User = { id: 'cookie-user', username: 'root', email: 'root@example.com', role: 'admin' };
    vi.mocked(authApi.refreshToken).mockResolvedValueOnce({
      token: createToken(user), refresh_token: 'rotated', expires_in: 3600,
    });
    vi.mocked(authApi.getMe).mockResolvedValueOnce({
      user_id: user.id, username: user.username, email: user.email, is_admin: true,
    } as Awaited<ReturnType<typeof authApi.getMe>>);
    await useAuthStore.getState().checkAuth();
    expect(authApi.refreshToken).toHaveBeenCalledWith(undefined);
    expect(useAuthStore.getState()).toMatchObject({
      isAuthenticated: true, isLoading: false, refreshToken: 'rotated', user: { id: user.id, role: 'admin' },
    });
    expect(authApi.logout).not.toHaveBeenCalled();
  });

  it('shares concurrent refresh requests and preserves the current user', async () => {
    await useAuthStore.getState().login('admin', 'admin123');
    const user = useAuthStore.getState().user;
    let resolve!: (value: Awaited<ReturnType<typeof authApi.refreshToken>>) => void;
    vi.mocked(authApi.refreshToken).mockReturnValueOnce(new Promise(done => { resolve = done; }));
    const first = useAuthStore.getState().refreshTokenAction();
    const second = useAuthStore.getState().refreshTokenAction();
    expect(authApi.refreshToken).toHaveBeenCalledTimes(1);
    resolve({ token: createToken(user!), refresh_token: 'rotated', expires_in: 3600 });
    await Promise.all([first, second]);
    expect(useAuthStore.getState().user).toEqual(user);
    expect(useAuthStore.getState().refreshToken).toBe('rotated');
  });

  it('marks an expired-token session authenticated after refresh', async () => {
    const user: User = { id: '1', username: 'test', email: 'test@example.com', role: 'user' };
    useAuthStore.setState({ token: createToken(user, -60), refreshToken: 'old', isLoading: false });
    await useAuthStore.getState().checkAuth();
    expect(useAuthStore.getState()).toMatchObject({ isAuthenticated: true, isLoading: false });
  });

  it('does not clear a new login when an old refresh fails', async () => {
    await useAuthStore.getState().login('admin', 'admin123');
    let reject!: (reason: Error) => void;
    vi.mocked(authApi.refreshToken).mockReturnValueOnce(new Promise((_, fail) => { reject = fail; }));
    const refresh = useAuthStore.getState().refreshTokenAction();
    await useAuthStore.getState().login('viewer', 'viewer123');
    reject(new Error('Refresh token has been revoked'));
    await refresh;
    expect(useAuthStore.getState()).toMatchObject({ isAuthenticated: true, user: { username: 'viewer' } });
    expect(authApi.logout).not.toHaveBeenCalled();
  });

  it('does not overwrite a new login when an old refresh succeeds', async () => {
    await useAuthStore.getState().login('admin', 'admin123');
    let resolve!: (value: Awaited<ReturnType<typeof authApi.refreshToken>>) => void;
    vi.mocked(authApi.refreshToken).mockReturnValueOnce(new Promise(done => { resolve = done; }));
    const refresh = useAuthStore.getState().refreshTokenAction();
    await useAuthStore.getState().login('viewer', 'viewer123');
    const token = useAuthStore.getState().token;
    resolve({ token: 'old-session', refresh_token: 'old-refresh', expires_in: 3600 });
    await refresh;
    expect(useAuthStore.getState().token).toBe(token);
    expect(useAuthStore.getState().user?.username).toBe('viewer');
  });

  it('logs out once when concurrent refresh callers receive a revoked token', async () => {
    await useAuthStore.getState().login('admin', 'admin123');
    vi.mocked(authApi.refreshToken).mockRejectedValueOnce(new Error('Refresh token has been revoked'));
    const results = await Promise.allSettled([
      useAuthStore.getState().refreshTokenAction(), useAuthStore.getState().refreshTokenAction(),
    ]);
    expect(results.every(result => result.status === 'rejected')).toBe(true);
    expect(authApi.logout).toHaveBeenCalledTimes(1);
    expect(useAuthStore.getState()).toMatchObject({ token: null, refreshToken: null, isAuthenticated: false });
  });

  it('migrates legacy cloud tokens out of localStorage', async () => {
    vi.mocked(authApi.isCloud).mockReturnValue(true);
    localStorage.setItem('mockforge-auth', JSON.stringify({
      state: { token: 'stale-access', refreshToken: 'revoked-refresh', user: null }, version: 0,
    }));
    await useAuthStore.persist.rehydrate();
    expect(useAuthStore.getState()).toMatchObject({ token: null, refreshToken: null });
    const persisted = JSON.parse(localStorage.getItem('mockforge-auth')!);
    expect(persisted.state).not.toHaveProperty('token');
    expect(persisted.state).not.toHaveProperty('refreshToken');
  });

  it('treats a missing refresh cookie as a normal signed-out browser', async () => {
    vi.mocked(authApi.isCloud).mockReturnValue(true);
    vi.mocked(fetch).mockResolvedValueOnce(new Response('{}', { status: 401 }));
    vi.mocked(authApi.refreshToken).mockRejectedValueOnce(new Error('Missing refresh token'));
    const logError = vi.spyOn(logger, 'error');
    await useAuthStore.getState().checkAuth();
    expect(useAuthStore.getState()).toMatchObject({
      token: null, refreshToken: null, user: null, isAuthenticated: false, isLoading: false,
    });
    expect(logError).not.toHaveBeenCalled();
    expect(authApi.logout).not.toHaveBeenCalled();
  });

});
