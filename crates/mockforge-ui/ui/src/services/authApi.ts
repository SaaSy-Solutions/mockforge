// Authentication API service
// Handles login, logout, and token refresh with the backend
// Supports both local admin mode (/__mockforge/) and cloud registry mode (/api/v1/)

import type { User } from '../types';

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

// Detect cloud mode: VITE_API_BASE_URL is set in .env.production
const isCloudMode = (): boolean => {
  const apiBase = import.meta.env.VITE_API_BASE_URL;
  return !!apiBase && apiBase !== '';
};

class AuthApiService {
  private cloud = isCloudMode();

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
      throw new Error(errorData.error || errorData.details?.message || `HTTP error! status: ${response.status}`);
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

  isCloud(): boolean {
    return this.cloud;
  }
}

export const authApi = new AuthApiService();
