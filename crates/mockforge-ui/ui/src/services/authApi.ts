// Authentication API service
// Handles login, logout, and token refresh with the backend

import type { User } from '../types';

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  token: string;
  refresh_token: string;
  user: User;
  expires_in: number;
}

export interface RefreshTokenRequest {
  refresh_token: string;
}

export interface ApiResponse<T> {
  success: boolean;
  data: T;
  error: string | null;
  timestamp: string;
}

class AuthApiService {
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
      throw new Error(errorData.error || `HTTP error! status: ${response.status}`);
    }

    const json: ApiResponse<T> = await response.json();
    if (!json.success) {
      throw new Error(json.error || 'Request failed');
    }

    return json.data;
  }

  async login(username: string, password: string): Promise<LoginResponse> {
    return this.fetchJson<LoginResponse>('/__mockforge/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password } as LoginRequest),
    });
  }

  async refreshToken(refreshToken: string): Promise<LoginResponse> {
    return this.fetchJson<LoginResponse>('/__mockforge/auth/refresh', {
      method: 'POST',
      body: JSON.stringify({ refresh_token: refreshToken } as RefreshTokenRequest),
    });
  }

  async logout(): Promise<void> {
    try {
      await this.fetchJson<{ message: string }>('/__mockforge/auth/logout', {
        method: 'POST',
      });
    } catch (error) {
      // Logout should not fail even if the server request fails
      console.warn('Logout request failed:', error);
    }
  }
}

export const authApi = new AuthApiService();
