import { logger } from '@/utils/logger';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { User, AuthState, AuthActions } from '../types';
import { authApi } from '../services/authApi';

interface AuthStore extends AuthState, AuthActions {
  checkAuth: () => Promise<void>;
  checkTokenExpiry: () => boolean;
  startTokenRefresh: () => void;
  stopTokenRefresh: () => void;
}

// Parse JWT token to extract user info (client-side validation only)
const parseToken = (token: string): { user: User | null; expiresAt: number | null } => {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return { user: null, expiresAt: null };

    const payload = JSON.parse(atob(parts[1])); // JWT payload is base64url encoded

    // Check expiration
    const expiresAt = payload.exp * 1000; // Convert to milliseconds
    if (expiresAt < Date.now()) {
      return { user: null, expiresAt: null };
    }

    // Extract user data from token
    const user: User = {
      id: payload.sub,
      username: payload.username,
      email: payload.email || '',
      role: payload.role,
    };

    return { user, expiresAt };
  } catch {
    return { user: null, expiresAt: null };
  }
};

// Token refresh interval management
let tokenRefreshInterval: ReturnType<typeof setInterval> | null = null;

export const useAuthStore = create<AuthStore>()(
  persist(
    (set, get) => ({
      user: null,
      token: null,
      refreshToken: null,
      isAuthenticated: false,
      isLoading: false,

      login: async (username: string, password: string) => {
        set({ isLoading: true });

        try {
          // Call real authentication API
          const response = await authApi.login(username, password);

          set({
            user: response.user,
            token: response.token,
            refreshToken: response.refresh_token,
            isAuthenticated: true,
            isLoading: false,
          });

          // Start automatic token refresh
          get().startTokenRefresh();
        } catch (error) {
          set({ isLoading: false });
          const errorMessage = error instanceof Error ? error.message : 'Login failed';
          logger.error('Login failed', errorMessage);
          throw new Error(errorMessage);
        }
      },

      logout: async () => {
        // Stop token refresh
        get().stopTokenRefresh();

        // Call logout API (fire and forget)
        try {
          await authApi.logout();
        } catch (error) {
          logger.warn('Logout API call failed', error);
        }

        // Clear local state
        set({
          user: null,
          token: null,
          refreshToken: null,
          isAuthenticated: false,
          isLoading: false,
        });
      },

      refreshTokenAction: async () => {
        const { refreshToken } = get();
        if (!refreshToken) throw new Error('No refresh token available');

        try {
          // Call real refresh token API
          const response = await authApi.refreshToken(refreshToken);

          set({
            token: response.token,
            refreshToken: response.refresh_token,
            user: response.user, // Update user info in case it changed
          });
        } catch (error) {
          // If refresh fails, logout
          logger.error('Token refresh failed', error);
          get().logout();
          throw error;
        }
      },

      checkTokenExpiry: () => {
        const { token } = get();
        if (!token) return false;

        try {
          const { expiresAt } = parseToken(token);
          if (!expiresAt) return false;

          // Check if token expires in less than 5 minutes
          const timeUntilExpiry = expiresAt - Date.now();
          return timeUntilExpiry > 5 * 60 * 1000; // 5 minutes in milliseconds
        } catch {
          return false;
        }
      },

      checkAuth: async () => {
        const { token, refreshToken } = get();
        if (!token) {
          set({ isAuthenticated: false, isLoading: false });
          return;
        }

        set({ isLoading: true });

        try {
          // Parse token to check validity
          const { user, expiresAt } = parseToken(token);

          if (user && expiresAt && expiresAt > Date.now()) {
            // Token is valid
            set({
              user,
              isAuthenticated: true,
              isLoading: false,
            });

            // Start token refresh if not already started
            get().startTokenRefresh();
          } else if (refreshToken) {
            // Token expired, try to refresh
            try {
              await get().refreshTokenAction();
            } catch {
              // Refresh failed, logout
              get().logout();
            }
          } else {
            // No refresh token, logout
            get().logout();
          }
        } catch (error) {
          logger.error('Auth check failed', error);
          get().logout();
        }
      },

      updateProfile: async (userData: User) => {
        set({ isLoading: true });

        try {
          // Update local state immediately for responsive UI
          // Note: Profile persistence would require a backend API endpoint
          // For now, profile updates are stored in local state only
          set({
            user: userData,
            isLoading: false,
          });

          // Optionally persist to localStorage as backup
          if (typeof window !== 'undefined') {
            localStorage.setItem('mockforge-user-profile', JSON.stringify(userData));
          }
        } catch (error) {
          set({ isLoading: false });
          const errorMessage = error instanceof Error ? error.message : 'Profile update failed';
          logger.error('Profile update failed', errorMessage);
          throw new Error(errorMessage);
        }
      },

      setAuthenticated: (user: User, token: string, refreshToken?: string) => {
        set({
          user,
          token,
          refreshToken: refreshToken || null,
          isAuthenticated: true,
          isLoading: false,
        });
        // Start token refresh
        get().startTokenRefresh();
      },

      startTokenRefresh: () => {
        // Clear any existing interval
        if (tokenRefreshInterval) {
          clearInterval(tokenRefreshInterval);
        }

        // Start new interval
        tokenRefreshInterval = setInterval(async () => {
          const { token, refreshToken: refresh, isAuthenticated } = get();

          if (isAuthenticated && token && refresh) {
            try {
              const payload = JSON.parse(atob(token.split('.')[2]));
              const timeUntilExpiry = payload.exp - Math.floor(Date.now() / 1000);

              // Refresh if token expires in less than 5 minutes
              if (timeUntilExpiry < 300) {
                await get().refreshTokenAction();
              }
            } catch {
              // If we can't parse the token, logout
              get().logout();
            }
          }
        }, 60000); // Check every minute
      },

      stopTokenRefresh: () => {
        if (tokenRefreshInterval) {
          clearInterval(tokenRefreshInterval);
          tokenRefreshInterval = null;
        }
      },
    }),
    {
      name: 'mockforge-auth',
      partialize: (state) => ({
        token: state.token,
        refreshToken: state.refreshToken,
        user: state.user,
        isAuthenticated: state.isAuthenticated,
      }),
    }
  )
);
