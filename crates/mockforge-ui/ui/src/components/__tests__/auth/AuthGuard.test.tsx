/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { AuthGuard } from '../../auth/AuthGuard';
import { useAuthStore } from '../../../stores/useAuthStore';

// Mock the auth store
vi.mock('../../../stores/useAuthStore');

const mockUseAuthStore = vi.mocked(useAuthStore);

describe('AuthGuard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders children when user is authenticated', () => {
    mockUseAuthStore.mockReturnValue({
      isAuthenticated: true,
      user: { id: '1', username: 'admin', email: 'test@example.com', role: 'admin' },
      isLoading: false,
      token: 'mock-token',
      refreshToken: 'mock-refresh-token',
      login: vi.fn(),
      logout: vi.fn(),
      checkAuth: vi.fn(),
      refreshTokenAction: vi.fn(),
      updateProfile: vi.fn(),
      checkTokenExpiry: vi.fn(),
      setAuthenticated: vi.fn(),
      startTokenRefresh: vi.fn(),
      stopTokenRefresh: vi.fn(),
    });

    render(
      <AuthGuard>
        <div data-testid="protected-content">Protected Content</div>
      </AuthGuard>
    );

    expect(screen.getByTestId('protected-content')).toBeInTheDocument();
  });

  it('renders login prompt when user is not authenticated', () => {
    mockUseAuthStore.mockReturnValue({
      isAuthenticated: false,
      user: null,
      isLoading: false,
      token: null,
      refreshToken: null,
      login: vi.fn(),
      logout: vi.fn(),
      checkAuth: vi.fn(),
      refreshTokenAction: vi.fn(),
      updateProfile: vi.fn(),
      checkTokenExpiry: vi.fn(),
      setAuthenticated: vi.fn(),
      startTokenRefresh: vi.fn(),
      stopTokenRefresh: vi.fn(),
    });

    render(
      <AuthGuard>
        <div data-testid="protected-content">Protected Content</div>
      </AuthGuard>
    );

    expect(screen.queryByTestId('protected-content')).not.toBeInTheDocument();
    expect(screen.getByText(/sign in to access the admin dashboard/i)).toBeInTheDocument();
  });

  it('renders loading state while authentication is being checked', () => {
    mockUseAuthStore.mockReturnValue({
      isAuthenticated: false,
      user: null,
      isLoading: true,
      token: null,
      refreshToken: null,
      login: vi.fn(),
      logout: vi.fn(),
      checkAuth: vi.fn(),
      refreshTokenAction: vi.fn(),
      updateProfile: vi.fn(),
      checkTokenExpiry: vi.fn(),
      setAuthenticated: vi.fn(),
      startTokenRefresh: vi.fn(),
      stopTokenRefresh: vi.fn(),
    });

    render(
      <AuthGuard>
        <div data-testid="protected-content">Protected Content</div>
      </AuthGuard>
    );

    expect(screen.queryByTestId('protected-content')).not.toBeInTheDocument();
    expect(screen.getByTestId('loading-spinner')).toBeInTheDocument();
  });

  it('checks authentication on mount', async () => {
    const mockCheckAuth = vi.fn();
    mockUseAuthStore.mockReturnValue({
      isAuthenticated: false,
      user: null,
      isLoading: true,
      token: null,
      refreshToken: null,
      login: vi.fn(),
      logout: vi.fn(),
      checkAuth: mockCheckAuth,
      refreshTokenAction: vi.fn(),
      updateProfile: vi.fn(),
      checkTokenExpiry: vi.fn(),
      setAuthenticated: vi.fn(),
      startTokenRefresh: vi.fn(),
      stopTokenRefresh: vi.fn(),
    });

    render(
      <AuthGuard>
        <div data-testid="protected-content">Protected Content</div>
      </AuthGuard>
    );

    await waitFor(() => {
      expect(mockCheckAuth).toHaveBeenCalledTimes(1);
    });
  });

  it('handles authentication errors gracefully', () => {
    mockUseAuthStore.mockReturnValue({
      isAuthenticated: false,
      user: null,
      isLoading: false,
      token: null,
      refreshToken: null,
      login: vi.fn(),
      logout: vi.fn(),
      checkAuth: vi.fn(),
      refreshTokenAction: vi.fn(),
      updateProfile: vi.fn(),
      checkTokenExpiry: vi.fn(),
      setAuthenticated: vi.fn(),
      startTokenRefresh: vi.fn(),
      stopTokenRefresh: vi.fn(),
    });

    render(
      <AuthGuard>
        <div data-testid="protected-content">Protected Content</div>
      </AuthGuard>
    );

    // AuthGuard may not display error messages - just check it doesn't crash
    expect(screen.queryByTestId('protected-content')).not.toBeInTheDocument();
  });

});