import { logger } from '@/utils/logger';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { User, AuthState, AuthActions } from '../types';

interface AuthStore extends AuthState, AuthActions {
  checkAuth: () => Promise<void>;
  checkTokenExpiry: () => boolean;
  startTokenRefresh: () => void;
  stopTokenRefresh: () => void;
}

// Mock user database
const mockUsers: Record<string, { password: string; user: User }> = {
  admin: {
    password: 'admin123',
    user: {
      id: 'admin-001',
      username: 'admin',
      role: 'admin',
      email: 'admin@mockforge.dev',
    },
  },
  viewer: {
    password: 'viewer123',
    user: {
      id: 'viewer-001',
      username: 'viewer',
      role: 'viewer',
      email: 'viewer@mockforge.dev',
    },
  },
};

// Mock JWT token generation
const generateToken = (user: User): string => {
  const header = { alg: 'HS256', typ: 'JWT' };
  const payload = {
    sub: user.id,
    username: user.username,
    role: user.role,
    iat: Math.floor(Date.now() / 1000),
    exp: Math.floor(Date.now() / 1000) + (60 * 60 * 24), // 24 hours
  };

  // In a real app, this would be properly signed
  return `mock.${btoa(JSON.stringify(header))}.${btoa(JSON.stringify(payload))}`;
};

// Mock token validation
const validateToken = (token: string): User | null => {
  try {
    if (!token.startsWith('mock.')) return null;

    const parts = token.split('.');
    if (parts.length !== 3) return null;

    const payload = JSON.parse(atob(parts[2]));

    // Check expiration
    if (payload.exp < Math.floor(Date.now() / 1000)) {
      return null;
    }

    // Return user data
    return {
      id: payload.sub,
      username: payload.username,
      email: payload.email || '',
      role: payload.role,
    };
  } catch {
    return null;
  }
};

// Simulate API delay
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

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
          // Simulate API call delay
          await delay(800);

          const userRecord = mockUsers[username];
          if (!userRecord || userRecord.password !== password) {
            throw new Error('Invalid username or password');
          }

          const token = generateToken(userRecord.user);
          const refreshToken = `refresh_${token}`;

          set({
            user: userRecord.user,
            token,
            refreshToken,
            isAuthenticated: true,
            isLoading: false,
          });
        } catch (error) {
          set({ isLoading: false });
          throw error;
        }
      },

      logout: async () => {
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
          await delay(300);

          // In a real app, this would validate the refresh token and return a new access token
          const currentUser = get().user;
          if (!currentUser) throw new Error('No user found');

          const newToken = generateToken(currentUser);
          const newRefreshToken = `refresh_${newToken}`;

          set({
            token: newToken,
            refreshToken: newRefreshToken,
          });
        } catch (error) {
          // If refresh fails, logout
          get().logout();
          throw error;
        }
      },

      checkTokenExpiry: () => {
        const { token } = get();
        if (!token) return false;

        try {
          const payload = JSON.parse(atob(token.split('.')[1]));
          const timeUntilExpiry = payload.exp - Math.floor(Date.now() / 1000);
          return timeUntilExpiry > 0;
        } catch {
          return false;
        }
      },

      checkAuth: async () => {
        const { token } = get();
        if (!token) {
          set({ isAuthenticated: false, isLoading: false });
          return;
        }

        set({ isLoading: true });

        try {
          await delay(200);

          const user = validateToken(token);
          if (user) {
            set({
              user,
              isAuthenticated: true,
              isLoading: false,
            });
          } else {
            // Token is invalid, try to refresh
            try {
              await get().refreshTokenAction();
            } catch {
              get().logout();
            }
          }
        } catch {
          get().logout();
        }
      },

      updateProfile: async (userData: User) => {
        set({ isLoading: true });

        try {
          // Simulate API call delay
          await delay(500);

          // In a real app, this would make an API call to update the user profile
          // For now, we'll just update the local state
          set({
            user: userData,
            isLoading: false,
          });

          // Update the token to reflect the new user data
          const newToken = generateToken(userData);
          const newRefreshToken = `refresh_${newToken}`;

          set({
            token: newToken,
            refreshToken: newRefreshToken,
          });
        } catch (error) {
          set({ isLoading: false });
          throw error;
        }
      },

      setAuthenticated: (user: User, token: string) => {
        const refreshToken = `refresh_${token}`;
        set({
          user,
          token,
          refreshToken,
          isAuthenticated: true,
          isLoading: false,
        });
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
