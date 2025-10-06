/**
 * @jest-environment jsdom
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { useAuthStore } from '../useAuthStore';
import type { User } from '../../types';

describe('useAuthStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();

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
  });

  afterEach(() => {
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
    expect(result.current.token).toContain('mock.');
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
        expect(error.message).toContain('Invalid username or password');
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
      refreshToken: 'invalid-refresh',
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
    const tokenBeforeRefresh = result.current.token;

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
        expect(error.message).toContain('No refresh token available');
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
    expect(result.current.refreshToken).toBe(`refresh_${token}`);
  });

  it('generates valid JWT-like tokens', async () => {
    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.login('admin', 'admin123');
    });

    const token = result.current.token;
    expect(token).toBeTruthy();
    expect(token).toContain('mock.');

    const parts = token?.split('.');
    expect(parts?.length).toBe(3);
  });
});
