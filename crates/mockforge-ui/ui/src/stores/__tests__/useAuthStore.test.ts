/**
 * @jest-environment jsdom
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { useAuthStore } from '../useAuthStore';

// Mock the API service
const mockApi = {
  login: vi.fn(),
  logout: vi.fn(),
  getCurrentUser: vi.fn(),
  refreshToken: vi.fn(),
};

vi.mock('../../services/api', () => ({
  api: mockApi,
}));

// Mock localStorage
const mockLocalStorage = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};

Object.defineProperty(window, 'localStorage', {
  value: mockLocalStorage,
});

describe('useAuthStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockLocalStorage.getItem.mockReturnValue(null);
    
    // Reset store state
    useAuthStore.setState({
      user: null,
      isAuthenticated: false,
      isLoading: false,
      error: null,
      token: null,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('initializes with default state', () => {
    const { result } = renderHook(() => useAuthStore());

    expect(result.current.user).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
    expect(result.current.token).toBeNull();
  });

  it('restores authentication state from localStorage on init', () => {
    const mockUser = { id: '1', email: 'test@example.com', role: 'admin' };
    const mockToken = 'mock-jwt-token';

    mockLocalStorage.getItem
      .mockReturnValueOnce(JSON.stringify(mockUser))
      .mockReturnValueOnce(mockToken);

    const { result } = renderHook(() => useAuthStore());

    act(() => {
      result.current.initializeAuth();
    });

    expect(result.current.user).toEqual(mockUser);
    expect(result.current.token).toBe(mockToken);
    expect(result.current.isAuthenticated).toBe(true);
  });

  it('handles successful login', async () => {
    const mockUser = { id: '1', email: 'test@example.com', role: 'admin' };
    const mockToken = 'mock-jwt-token';
    const credentials = { email: 'test@example.com', password: 'password123' };

    mockApi.login.mockResolvedValue({
      user: mockUser,
      token: mockToken,
      expiresIn: 3600,
    });

    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.login(credentials);
    });

    expect(mockApi.login).toHaveBeenCalledWith(credentials);
    expect(result.current.user).toEqual(mockUser);
    expect(result.current.token).toBe(mockToken);
    expect(result.current.isAuthenticated).toBe(true);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();

    expect(mockLocalStorage.setItem).toHaveBeenCalledWith('auth_user', JSON.stringify(mockUser));
    expect(mockLocalStorage.setItem).toHaveBeenCalledWith('auth_token', mockToken);
  });

  it('handles login failure', async () => {
    const credentials = { email: 'test@example.com', password: 'wrongpassword' };
    const errorMessage = 'Invalid credentials';

    mockApi.login.mockRejectedValue(new Error(errorMessage));

    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.login(credentials);
    });

    expect(result.current.user).toBeNull();
    expect(result.current.token).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBe(errorMessage);
  });

  it('sets loading state during login', async () => {
    const credentials = { email: 'test@example.com', password: 'password123' };
    
    // Create a promise that we can control
    let resolveLogin;
    const loginPromise = new Promise((resolve) => {
      resolveLogin = resolve;
    });
    mockApi.login.mockReturnValue(loginPromise);

    const { result } = renderHook(() => useAuthStore());

    // Start login
    act(() => {
      result.current.login(credentials);
    });

    // Should be loading
    expect(result.current.isLoading).toBe(true);

    // Resolve login
    await act(async () => {
      resolveLogin({ user: { id: '1' }, token: 'token' });
      await loginPromise;
    });

    // Should no longer be loading
    expect(result.current.isLoading).toBe(false);
  });

  it('handles logout', async () => {
    // Set up authenticated state
    useAuthStore.setState({
      user: { id: '1', email: 'test@example.com', role: 'admin' },
      token: 'mock-token',
      isAuthenticated: true,
    });

    mockApi.logout.mockResolvedValue(undefined);

    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.logout();
    });

    expect(mockApi.logout).toHaveBeenCalled();
    expect(result.current.user).toBeNull();
    expect(result.current.token).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
    expect(result.current.error).toBeNull();

    expect(mockLocalStorage.removeItem).toHaveBeenCalledWith('auth_user');
    expect(mockLocalStorage.removeItem).toHaveBeenCalledWith('auth_token');
  });

  it('checks authentication status', async () => {
    const mockUser = { id: '1', email: 'test@example.com', role: 'admin' };
    mockApi.getCurrentUser.mockResolvedValue(mockUser);

    // Set up token in state
    useAuthStore.setState({ token: 'valid-token' });

    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.checkAuth();
    });

    expect(mockApi.getCurrentUser).toHaveBeenCalled();
    expect(result.current.user).toEqual(mockUser);
    expect(result.current.isAuthenticated).toBe(true);
  });

  it('handles invalid token during auth check', async () => {
    mockApi.getCurrentUser.mockRejectedValue(new Error('Invalid token'));

    // Set up token in state
    useAuthStore.setState({ token: 'invalid-token' });

    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.checkAuth();
    });

    expect(result.current.user).toBeNull();
    expect(result.current.token).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
    expect(mockLocalStorage.removeItem).toHaveBeenCalledWith('auth_user');
    expect(mockLocalStorage.removeItem).toHaveBeenCalledWith('auth_token');
  });

  it('refreshes token', async () => {
    const newToken = 'new-jwt-token';
    const newUser = { id: '1', email: 'test@example.com', role: 'admin' };

    mockApi.refreshToken.mockResolvedValue({
      token: newToken,
      user: newUser,
      expiresIn: 3600,
    });

    // Set up existing token
    useAuthStore.setState({ token: 'old-token' });

    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.refreshToken();
    });

    expect(mockApi.refreshToken).toHaveBeenCalled();
    expect(result.current.token).toBe(newToken);
    expect(result.current.user).toEqual(newUser);
    expect(mockLocalStorage.setItem).toHaveBeenCalledWith('auth_token', newToken);
  });

  it('handles token refresh failure', async () => {
    mockApi.refreshToken.mockRejectedValue(new Error('Refresh failed'));

    // Set up existing token
    useAuthStore.setState({ 
      token: 'old-token',
      user: { id: '1' },
      isAuthenticated: true 
    });

    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.refreshToken();
    });

    // Should clear auth state on refresh failure
    expect(result.current.user).toBeNull();
    expect(result.current.token).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
  });

  it('clears error on successful operation after error', async () => {
    const mockUser = { id: '1', email: 'test@example.com', role: 'admin' };
    const credentials = { email: 'test@example.com', password: 'password123' };

    // First, cause an error
    mockApi.login.mockRejectedValueOnce(new Error('Network error'));

    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.login(credentials);
    });

    expect(result.current.error).toBe('Network error');

    // Then succeed
    mockApi.login.mockResolvedValueOnce({
      user: mockUser,
      token: 'token',
    });

    await act(async () => {
      await result.current.login(credentials);
    });

    expect(result.current.error).toBeNull();
  });

  it('updates user profile', async () => {
    const currentUser = { id: '1', email: 'test@example.com', role: 'admin' };
    const updatedUser = { id: '1', email: 'newemail@example.com', role: 'admin' };

    // Set up authenticated state
    useAuthStore.setState({
      user: currentUser,
      isAuthenticated: true,
    });

    mockApi.updateProfile = vi.fn().mockResolvedValue(updatedUser);

    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.updateProfile({ email: 'newemail@example.com' });
    });

    expect(result.current.user).toEqual(updatedUser);
    expect(mockLocalStorage.setItem).toHaveBeenCalledWith('auth_user', JSON.stringify(updatedUser));
  });

  it('handles concurrent authentication requests', async () => {
    const credentials = { email: 'test@example.com', password: 'password123' };
    const mockUser = { id: '1', email: 'test@example.com', role: 'admin' };

    mockApi.login.mockResolvedValue({
      user: mockUser,
      token: 'token',
    });

    const { result } = renderHook(() => useAuthStore());

    // Start multiple login attempts concurrently
    const loginPromises = [
      result.current.login(credentials),
      result.current.login(credentials),
      result.current.login(credentials),
    ];

    await act(async () => {
      await Promise.all(loginPromises);
    });

    // Should only call API once due to concurrent request handling
    expect(mockApi.login).toHaveBeenCalledTimes(1);
    expect(result.current.user).toEqual(mockUser);
  });

  it('provides user role checking utilities', () => {
    const { result } = renderHook(() => useAuthStore());

    // Not authenticated
    expect(result.current.hasRole('admin')).toBe(false);
    expect(result.current.hasAnyRole(['admin', 'user'])).toBe(false);

    act(() => {
      useAuthStore.setState({
        user: { id: '1', email: 'test@example.com', role: 'admin' },
        isAuthenticated: true,
      });
    });

    expect(result.current.hasRole('admin')).toBe(true);
    expect(result.current.hasRole('user')).toBe(false);
    expect(result.current.hasAnyRole(['admin', 'user'])).toBe(true);
    expect(result.current.hasAnyRole(['moderator', 'user'])).toBe(false);
  });

  it('handles token expiration', async () => {
    // Set up authenticated state with expired token
    useAuthStore.setState({
      user: { id: '1', email: 'test@example.com', role: 'admin' },
      token: 'expired-token',
      isAuthenticated: true,
    });

    mockApi.getCurrentUser.mockRejectedValue(new Error('Token expired'));

    const { result } = renderHook(() => useAuthStore());

    await act(async () => {
      await result.current.checkAuth();
    });

    // Should clear auth state when token is expired
    expect(result.current.user).toBeNull();
    expect(result.current.token).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
  });
});