// Authentication API service
// Handles login, logout, and token refresh with the backend
// Supports both local admin mode (/__mockforge/) and cloud registry mode (/api/v1/)

import type { User } from '../types';
import { apiErrorMessage } from '@/utils/errorHandling';
import { isCloudMode } from '../utils/cloudMode';

export interface LoginResponse {
  token: string;
  refresh_token: string;
  user: User;
  expires_in: number;
}

export interface RefreshTokenRequest {
  refresh_token: string;
}

// Local admin API wraps responses in {success, data, error, timestamp}
interface LocalApiResponse<T> {
  success: boolean;
  data: T;
  error: string | null;
  timestamp: string;
}

class AuthApiService {
  private cloud = isCloudMode();

  private authHeader(): Record<string, string> {
    const token = localStorage.getItem('auth_token');
    return token ? { Authorization: `Bearer ${token}` } : {};
  }

  private async authedFetch<T>(path: string, options: RequestInit = {}): Promise<T> {
    const response = await fetch(path, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...this.authHeader(),
        ...options.headers,
      },
    });
    if (!response.ok) {
      const body = await response.json().catch(() => ({ error: 'Unknown error' }));
      throw new Error(apiErrorMessage(response, body, `HTTP ${response.status}`));
    }
    if (response.status === 204) return undefined as unknown as T;
    return response.json();
  }

  private async fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
    const response = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
      throw new Error(apiErrorMessage(response, errorData, `HTTP error! status: ${response.status}`));
    }

    if (this.cloud) {
      // Cloud registry returns flat JSON
      return response.json();
    } else {
      // Local admin wraps in {success, data, error}
      const json: LocalApiResponse<T> = await response.json();
      if (!json.success) {
        throw new Error(json.error || 'Request failed');
      }
      return json.data;
    }
  }

  async login(usernameOrEmail: string, password: string): Promise<LoginResponse> {
    if (this.cloud) {
      // Cloud registry: POST /api/v1/auth/login with {email, password}
      const raw = await this.fetchJson<{
        access_token: string;
        refresh_token: string;
        access_token_expires_at: number;
        refresh_token_expires_at: number;
        user_id: string;
        username: string;
      }>('/api/v1/auth/login', {
        method: 'POST',
        body: JSON.stringify({ email: usernameOrEmail, password }),
      });

      // Normalize to LoginResponse format
      return {
        token: raw.access_token,
        refresh_token: raw.refresh_token,
        user: {
          id: raw.user_id,
          username: raw.username,
          email: usernameOrEmail,
          role: 'user',
        },
        expires_in: raw.access_token_expires_at - Math.floor(Date.now() / 1000),
      };
    } else {
      // Local admin: POST /__mockforge/auth/login with {username, password}
      return this.fetchJson<LoginResponse>('/__mockforge/auth/login', {
        method: 'POST',
        body: JSON.stringify({ username: usernameOrEmail, password }),
      });
    }
  }

  async register(username: string, email: string, password: string): Promise<LoginResponse> {
    const raw = await this.fetchJson<{
      access_token: string;
      refresh_token: string;
      access_token_expires_at: number;
      refresh_token_expires_at: number;
      user_id: string;
      username: string;
    }>('/api/v1/auth/register', {
      method: 'POST',
      body: JSON.stringify({ username, email, password }),
    });

    return {
      token: raw.access_token,
      refresh_token: raw.refresh_token,
      user: {
        id: raw.user_id,
        username: raw.username,
        email,
        role: 'user',
      },
      expires_in: raw.access_token_expires_at - Math.floor(Date.now() / 1000),
    };
  }

  async refreshToken(refreshToken: string): Promise<LoginResponse> {
    if (this.cloud) {
      const raw = await this.fetchJson<{
        access_token: string;
        refresh_token: string;
        access_token_expires_at: number;
        refresh_token_expires_at: number;
        user_id: string;
        username: string;
      }>('/api/v1/auth/token/refresh', {
        method: 'POST',
        body: JSON.stringify({ refresh_token: refreshToken }),
      });

      return {
        token: raw.access_token,
        refresh_token: raw.refresh_token,
        user: {
          id: raw.user_id,
          username: raw.username,
          email: '',
          role: 'user',
        },
        expires_in: raw.access_token_expires_at - Math.floor(Date.now() / 1000),
      };
    } else {
      return this.fetchJson<LoginResponse>('/__mockforge/auth/refresh', {
        method: 'POST',
        body: JSON.stringify({ refresh_token: refreshToken } as RefreshTokenRequest),
      });
    }
  }

  async logout(): Promise<void> {
    try {
      if (!this.cloud) {
        await this.fetchJson<{ message: string }>('/__mockforge/auth/logout', {
          method: 'POST',
        });
      }
      // Cloud mode: just clear local state (no server-side logout endpoint needed)
    } catch (error) {
      console.warn('Logout request failed:', error);
    }
  }

  /**
   * Fetch the full profile of the currently-authenticated user.
   * Cloud-mode only — local admin mode doesn't persist user state.
   */
  async getMe(): Promise<UserProfile> {
    return this.authedFetch<UserProfile>('/api/v1/users/me');
  }

  async updateProfile(patch: UpdateProfilePayload): Promise<UserProfile> {
    return this.authedFetch<UserProfile>('/api/v1/users/me', {
      method: 'PATCH',
      body: JSON.stringify(patch),
    });
  }

  async changePassword(
    currentPassword: string,
    newPassword: string,
  ): Promise<{ success: boolean; message: string }> {
    return this.authedFetch<{ success: boolean; message: string }>(
      '/api/v1/auth/change-password',
      {
        method: 'POST',
        body: JSON.stringify({
          current_password: currentPassword,
          new_password: newPassword,
        }),
      },
    );
  }

  async updateNotifications(
    patch: UpdateNotificationsPayload,
  ): Promise<NotificationsPrefs> {
    return this.authedFetch<NotificationsPrefs>(
      '/api/v1/users/me/notifications',
      {
        method: 'PATCH',
        body: JSON.stringify(patch),
      },
    );
  }

  async getPreferences(): Promise<Record<string, unknown>> {
    const res = await this.authedFetch<{ preferences: Record<string, unknown> }>(
      '/api/v1/users/me/preferences',
    );
    return res.preferences ?? {};
  }

  async updatePreferences(
    patch: Record<string, unknown>,
  ): Promise<Record<string, unknown>> {
    const res = await this.authedFetch<{ preferences: Record<string, unknown> }>(
      '/api/v1/users/me/preferences',
      {
        method: 'PATCH',
        body: JSON.stringify({ preferences: patch }),
      },
    );
    return res.preferences ?? {};
  }

  async setup2FA(): Promise<TwoFactorSetup> {
    return this.authedFetch<TwoFactorSetup>('/api/v1/auth/2fa/setup');
  }

  async verify2FASetup(
    secret: string,
    code: string,
    backupCodes: string[],
  ): Promise<{ success: boolean; message: string }> {
    return this.authedFetch<{ success: boolean; message: string }>(
      '/api/v1/auth/2fa/verify-setup',
      {
        method: 'POST',
        body: JSON.stringify({ secret, code, backup_codes: backupCodes }),
      },
    );
  }

  async disable2FA(password: string): Promise<{ success: boolean; message: string }> {
    return this.authedFetch<{ success: boolean; message: string }>(
      '/api/v1/auth/2fa/disable',
      {
        method: 'POST',
        body: JSON.stringify({ password }),
      },
    );
  }

  async get2FAStatus(): Promise<{ enabled: boolean; verified_at: string | null }> {
    return this.authedFetch<{ enabled: boolean; verified_at: string | null }>(
      '/api/v1/auth/2fa/status',
    );
  }

  isCloud(): boolean {
    return this.cloud;
  }
}

export interface UserProfile {
  user_id: string;
  username: string;
  email: string;
  is_verified: boolean;
  is_admin: boolean;
  two_factor_enabled: boolean;
  email_notifications: boolean;
  security_alerts: boolean;
  preferences: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface UpdateProfilePayload {
  username?: string;
  email?: string;
}

export interface UpdateNotificationsPayload {
  email_notifications?: boolean;
  security_alerts?: boolean;
}

export interface NotificationsPrefs {
  email_notifications: boolean;
  security_alerts: boolean;
}

export interface TwoFactorSetup {
  secret: string;
  qr_code_url: string;
  backup_codes: string[];
}

export const authApi = new AuthApiService();
