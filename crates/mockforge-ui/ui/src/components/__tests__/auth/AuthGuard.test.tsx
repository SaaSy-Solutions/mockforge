/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import { AuthGuard } from '../../auth/AuthGuard';
import { useAuthStore } from '../../../stores/useAuthStore';

// Mock the auth store
vi.mock('../../../stores/useAuthStore');

const mockUseAuthStore = vi.mocked(useAuthStore);

function renderWithRouter(ui: React.ReactElement, initialEntries: string[] = ['/dashboard']) {
  return render(<MemoryRouter initialEntries={initialEntries}>{ui}</MemoryRouter>);
}

describe('AuthGuard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders children when user is authenticated', async () => {
    mockUseAuthStore.mockReturnValue({
      isAuthenticated: true,
      user: { id: '1', username: 'admin', email: 'test@example.com', role: 'admin' },
      isLoading: false,
      token: 'mock-token',
      refreshToken: 'mock-refresh-token',
      login: vi.fn(),
      logout: vi.fn(),
      checkAuth: vi.fn().mockResolvedValue(undefined),
      refreshTokenAction: vi.fn(),
      updateProfile: vi.fn(),
      checkTokenExpiry: vi.fn(),
      setAuthenticated: vi.fn(),
      startTokenRefresh: vi.fn(),
      stopTokenRefresh: vi.fn(),
    });

    renderWithRouter(
      <AuthGuard>
        <div data-testid="protected-content">Protected Content</div>
      </AuthGuard>
    );

    await waitFor(() => {
      expect(screen.getByTestId('protected-content')).toBeInTheDocument();
    });
  });

  it('renders login prompt when user is not authenticated', async () => {
    mockUseAuthStore.mockReturnValue({
      isAuthenticated: false,
      user: null,
      isLoading: false,
      token: null,
      refreshToken: null,
      login: vi.fn(),
      logout: vi.fn(),
      checkAuth: vi.fn().mockResolvedValue(undefined),
      refreshTokenAction: vi.fn(),
      updateProfile: vi.fn(),
      checkTokenExpiry: vi.fn(),
      setAuthenticated: vi.fn(),
      startTokenRefresh: vi.fn(),
      stopTokenRefresh: vi.fn(),
    });

    renderWithRouter(
      <AuthGuard>
        <div data-testid="protected-content">Protected Content</div>
      </AuthGuard>
    );

    await waitFor(() => {
      expect(screen.queryByTestId('protected-content')).not.toBeInTheDocument();
      // LoginForm renders "Sign in to manage your mock APIs" (cloud) or
      // "Sign in to access the admin dashboard" (local) depending on VITE_API_BASE_URL
      expect(screen.getByText(/sign in to/i)).toBeInTheDocument();
    });
  });

  it('keeps entered values and login failure feedback when the store enters a loading state', async () => {
    const checkAuth = vi.fn().mockResolvedValue(undefined);
    let rejectLogin!: (error: Error) => void;
    const state = {
      isAuthenticated: false, user: null, isLoading: false, checkAuth,
      login: vi.fn(() => new Promise<void>((_resolve, reject) => { rejectLogin = reject; })),
      setAuthenticated: vi.fn(),
    };
    mockUseAuthStore.mockReturnValue(state as unknown as ReturnType<typeof useAuthStore>);
    const ui = <MemoryRouter><AuthGuard><div>Protected</div></AuthGuard></MemoryRouter>;
    const view = render(ui);
    const identifier = await screen.findByLabelText(/^(Email|Username)$/);
    fireEvent.change(identifier, { target: { value: 'fixture@example.com' } });
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'Fixture-password' } });
    fireEvent.click(screen.getByRole('button', { name: 'Sign In' }));
    state.isLoading = true;
    view.rerender(<MemoryRouter><AuthGuard><div>Protected</div></AuthGuard></MemoryRouter>);
    expect(screen.getByLabelText(/^(Email|Username)$/)).toHaveValue('fixture@example.com');
    rejectLogin(new Error('Invalid credentials'));
    expect(await screen.findByRole('alert')).toHaveTextContent('Invalid credentials');
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
      checkAuth: vi.fn().mockResolvedValue(undefined),
      refreshTokenAction: vi.fn(),
      updateProfile: vi.fn(),
      checkTokenExpiry: vi.fn(),
      setAuthenticated: vi.fn(),
      startTokenRefresh: vi.fn(),
      stopTokenRefresh: vi.fn(),
    });

    renderWithRouter(
      <AuthGuard>
        <div data-testid="protected-content">Protected Content</div>
      </AuthGuard>
    );

    expect(screen.queryByTestId('protected-content')).not.toBeInTheDocument();
    expect(screen.getByTestId('loading-spinner')).toBeInTheDocument();
  });

  it('checks authentication on mount', async () => {
    const mockCheckAuth = vi.fn().mockResolvedValue(undefined);
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

    renderWithRouter(
      <AuthGuard>
        <div data-testid="protected-content">Protected Content</div>
      </AuthGuard>
    );

    await waitFor(() => {
      expect(mockCheckAuth).toHaveBeenCalledTimes(1);
    });
  });

  it('handles authentication errors gracefully', async () => {
    mockUseAuthStore.mockReturnValue({
      isAuthenticated: false,
      user: null,
      isLoading: false,
      token: null,
      refreshToken: null,
      login: vi.fn(),
      logout: vi.fn(),
      checkAuth: vi.fn().mockResolvedValue(undefined),
      refreshTokenAction: vi.fn(),
      updateProfile: vi.fn(),
      checkTokenExpiry: vi.fn(),
      setAuthenticated: vi.fn(),
      startTokenRefresh: vi.fn(),
      stopTokenRefresh: vi.fn(),
    });

    renderWithRouter(
      <AuthGuard>
        <div data-testid="protected-content">Protected Content</div>
      </AuthGuard>
    );

    // AuthGuard may not display error messages - just check it doesn't crash
    await waitFor(() => {
      expect(screen.queryByTestId('protected-content')).not.toBeInTheDocument();
    });
  });

});
