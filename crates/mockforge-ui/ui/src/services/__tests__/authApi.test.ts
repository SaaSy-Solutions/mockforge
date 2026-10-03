import { beforeEach, describe, expect, it, vi } from 'vitest';
import { authApi } from '../authApi';

vi.mock('../../utils/cloudMode', () => ({ isCloudMode: () => true }));

describe('cloud auth API', () => {
  beforeEach(() => vi.mocked(fetch).mockReset());

  it('accepts the backend refresh response without inventing user fields', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify({
      access_token: 'new-access', refresh_token: 'new-refresh',
      access_token_expires_at: Math.floor(Date.now() / 1000) + 900,
      refresh_token_expires_at: Math.floor(Date.now() / 1000) + 86400,
    })));
    const result = await authApi.refreshToken('old-refresh');
    expect(result).toMatchObject({ token: 'new-access', refresh_token: 'new-refresh' });
    expect(result).not.toHaveProperty('user');
    expect(fetch).toHaveBeenCalledWith('/api/v1/auth/token/refresh', expect.objectContaining({
      credentials: 'include', method: 'POST', body: JSON.stringify({ refresh_token: 'old-refresh' }),
    }));
  });

  it('allows the server to use its HttpOnly refresh cookie', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify({
      access_token: 'access', refresh_token: 'refresh', access_token_expires_at: 9999999999,
    })));
    await authApi.refreshToken();
    expect(fetch).toHaveBeenCalledWith('/api/v1/auth/token/refresh', expect.objectContaining({
      credentials: 'include', body: '{}',
    }));
  });

  it('explicitly logs out the cloud cookie session', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response('{}'));
    await authApi.logout();
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledWith('/api/v1/auth/logout', expect.objectContaining({
      credentials: 'include', method: 'POST',
    }));
  });
});
