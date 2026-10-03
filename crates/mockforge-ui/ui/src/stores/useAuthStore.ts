import { logger } from '@/utils/logger';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { User, AuthState, AuthActions } from '../types';
import { authApi } from '../services/authApi';
import { saveUserProfile } from '../services/userProfile';
import { hydrateUserFromServer, readCookieUser } from '../services/authUser';
import { isMissingRefreshCookie, resolveRefreshedUser } from '../services/authSession';
import { parseToken, setAuthToken, clearAuthToken } from '../services/tokenStorage';

interface AuthStore extends AuthState, AuthActions {
  checkAuth: () => Promise<void>;
  checkTokenExpiry: () => boolean;
  startTokenRefresh: () => void;
  stopTokenRefresh: () => void;
}

// Token refresh interval management
let authGeneration = 0;
let pendingRefresh: { generation: number; promise: Promise<void> } | null = null;

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
        const generation = ++authGeneration;
        set({ isLoading: true });

        try {
          // Call real authentication API
          const response = await authApi.login(username, password);

          if (generation !== authGeneration) return;

          // Persist tokens immediately so the hydrate call carries Authorization.
          set({
            user: response.user,
            token: response.token,
            refreshToken: response.refresh_token,
            isAuthenticated: true,
            isLoading: false,
          });

          // Cloud login response and JWT lack role/is_verified/email; hydrate
          // them from /users/me so admins resolve to role='admin'.
          const hydrated = await hydrateUserFromServer(response.user);
          if (generation !== authGeneration) return;
          set({ user: hydrated });
          // Start automatic token refresh
          get().startTokenRefresh();
        } catch (error) {
          if (generation !== authGeneration) return;
          set({ isLoading: false });
          const errorMessage = error instanceof Error ? error.message : 'Login failed';
          logger.error('Login failed', errorMessage);
          throw new Error(errorMessage);
        }
      },

      logout: async () => {
        ++authGeneration;
        get().stopTokenRefresh();
        // Clear immediately; a delayed logout response must not clear a new login.
        set({ user: null, token: null, refreshToken: null,
          isAuthenticated: false, isLoading: false });
        await authApi.logout();
      },

      refreshTokenAction: async () => {
        const generation = authGeneration;
        if (pendingRefresh?.generation === generation) return pendingRefresh.promise;
        const { refreshToken } = get();
        if (!refreshToken && !authApi.isCloud()) throw new Error('No refresh token available');

        const promise = (async () => {
          try {
            // Cloud can restore from the HttpOnly refresh cookie after a reload.
            const response = await authApi.refreshToken(refreshToken ?? undefined);
            if (generation !== authGeneration) return;
            const parsedUser = parseToken(response.token).user;
            const currentUser = get().user;
            const user = resolveRefreshedUser(response.user, currentUser, parsedUser);
            if (!user) throw new Error('Invalid refreshed session');
            set({ token: response.token, refreshToken: response.refresh_token,
              user, isAuthenticated: true, isLoading: false });
            const hydrated = await hydrateUserFromServer(user);
            if (generation !== authGeneration) return;
            set({ user: hydrated });
            get().startTokenRefresh();
          } catch (error) {
            if (generation !== authGeneration) return;
            const current = get();
            if (isMissingRefreshCookie(current, error)) {
              // A fresh browser has no refresh cookie. This is a normal signed-out
              // state, and must not emit an error or send a server logout request.
              get().stopTokenRefresh();
              set({ user: null, isAuthenticated: false, isLoading: false });
              return;
            }
            logger.error('Token refresh failed', error);
            await get().logout();
            throw error;
          }
        })();
        pendingRefresh = { generation, promise };
        try {
          await promise;
        } finally {
          if (pendingRefresh?.promise === promise) pendingRefresh = null;
        }
      },

      checkTokenExpiry: () => {
        // parseToken already rejects missing, malformed, and expired JWTs.
        const { expiresAt } = parseToken(get().token ?? '');
        return Number(expiresAt) - Date.now() > 5 * 60 * 1000;
      },

      checkAuth: async () => {
        const generation = authGeneration;
        const { token, refreshToken, user: existingUser } = get();
        if (!token && authApi.isCloud()) {
          const cookieUser = await readCookieUser(existingUser);
          if (generation !== authGeneration) return;
          if (cookieUser) {
            set({ user: cookieUser, isAuthenticated: true, isLoading: true });
          }
          // Cookie authentication and legacy bearer consumers both need restoration
          // to finish before protected pages mount. Refresh once in either case.
          try {
            await get().refreshTokenAction();
          } catch {
            // refreshTokenAction clears the rejected session.
          }
          return;
        }

        if (!token) {
          set({ isAuthenticated: false, isLoading: false });
          return;
        }
        set({ isLoading: true });

        try {
          // Parse token to check validity
          const { user: parsedUser, expiresAt } = parseToken(token);

          if (parsedUser && expiresAt && expiresAt > Date.now()) {
            // Token is valid — keep existing user data if available (login response has
            // full user info; JWT may only have sub/id with no username/email)
            const user = existingUser && existingUser.username ? existingUser : parsedUser;
            set({
              user,
              isAuthenticated: true,
              isLoading: false,
            });

            // Refresh role/is_verified from server in cloud mode — persisted
            // user state is stale across role changes (admin promotions, email
            // verification) since the JWT carries no role claim.
            const hydrated = await hydrateUserFromServer(user);
            if (generation !== authGeneration) return;
            set({ user: hydrated });
            // Start token refresh if not already started
            get().startTokenRefresh();
          } else if (refreshToken) {
            // Token expired, try to refresh
            try {
              await get().refreshTokenAction();
            } catch {
              // refreshTokenAction already clears the rejected session.
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
          const updatedUser = await saveUserProfile(get().user, userData);

          set({
            user: updatedUser,
            isLoading: false,
          });
        } catch (error) {
          set({ isLoading: false });
          const errorMessage = error instanceof Error ? error.message : 'Profile update failed';
          logger.error('Profile update failed', errorMessage);
          throw new Error(errorMessage);
        }
      },

      setAuthenticated: (user: User, token: string, refreshToken?: string) => {
        ++authGeneration;
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
              const { expiresAt } = parseToken(token);
              const timeUntilExpiry = expiresAt ? (expiresAt - Date.now()) / 1000 : 0;

              // Refresh if token expires in less than 5 minutes
              if (timeUntilExpiry < 300) {
                await get().refreshTokenAction();
              }
            } catch {
              // A rejected refresh already clears the session.
              if (get().token === token && get().isAuthenticated) void get().logout();
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
      version: 1,
      migrate: (persisted) => {
        const state = persisted as Partial<AuthState>;
        return authApi.isCloud() ? { user: state.user ?? null } : { token: state.token ?? null, refreshToken: state.refreshToken ?? null, user: state.user ?? null };
      },
      partialize: (state) => ({
        ...(authApi.isCloud() ? {} : { token: state.token, refreshToken: state.refreshToken }),
        user: state.user,
        // Do NOT persist isAuthenticated — derive it from token via checkAuth()
        // to prevent stale auth state from showing the dashboard before validation
      }),
    }
  )
);

// Sync token through the central storage module so non-store consumers see it.
useAuthStore.subscribe((state) => {
  if (state.token) {
    setAuthToken(state.token);
  } else {
    clearAuthToken();
  }
});
