// Registry admin API client.
//
// In **cloud mode** (VITE_API_BASE_URL set): calls go to the SaaS
// backend at /api/v1/* and use the existing SaaS JWT from useAuthStore
// (stored under `auth_token` in localStorage).
//
// In **self-hosted mode** (no VITE_API_BASE_URL): calls go to the
// embedded SQLite-backed endpoints at /api/admin/registry/* and use a
// separate JWT stored under `mockforge_registry_admin_token`.

const TOKEN_STORAGE_KEY = 'mockforge_registry_admin_token';
const SAAS_TOKEN_KEY = 'auth_token'; // matches useAuthStore's persist key

/** True when the frontend is served by Vercel with a cloud API backend. */
export const isCloudMode = (): boolean => {
  const apiBase = import.meta.env.VITE_API_BASE_URL;
  return !!apiBase && apiBase !== '';
};

export interface RegistryUser {
  id: string;
  username: string;
  email: string;
  is_verified: boolean;
  is_admin: boolean;
  created_at?: string;
  updated_at?: string;
}

export interface RegistryOrg {
  id: string;
  name: string;
  slug: string;
  owner_id: string;
  plan: string;
  created_at?: string;
  updated_at?: string;
}

export interface RegistryOrgMember {
  id: string;
  org_id: string;
  user_id: string;
  role: string;
  created_at?: string;
  updated_at?: string;
}

export interface LoginResponse {
  user: RegistryUser;
  token: string;
}

export interface CreateApiTokenResponse {
  token: string; // plaintext, shown once
  id: string;
  org_id: string;
  user_id: string | null;
  name: string;
  token_prefix: string;
  scopes: string[];
  created_at: string;
}

/** Load the JWT from localStorage, or null if not logged in.
 *  In cloud mode, reads the SaaS token; in self-hosted mode, the
 *  registry admin's own token. */
export function getStoredToken(): string | null {
  try {
    if (isCloudMode()) {
      // SaaS auth store persists a JSON blob; extract the token field.
      const raw = localStorage.getItem(SAAS_TOKEN_KEY);
      if (!raw) return null;
      // useAuthStore stores just the JWT string directly under auth_token
      return raw;
    }
    return localStorage.getItem(TOKEN_STORAGE_KEY);
  } catch {
    return null;
  }
}

/** Save the JWT to localStorage (self-hosted only; cloud mode uses
 *  the SaaS auth store which manages its own storage). */
export function setStoredToken(token: string | null): void {
  if (isCloudMode()) return; // cloud auth is managed by useAuthStore
  try {
    if (token === null) {
      localStorage.removeItem(TOKEN_STORAGE_KEY);
    } else {
      localStorage.setItem(TOKEN_STORAGE_KEY, token);
    }
  } catch {
    // ignore quota / private-mode errors
  }
}

/** Clear the stored JWT (logout). In cloud mode this is a no-op —
 *  use `useAuthStore().logout()` instead. */
export function clearStoredToken(): void {
  if (isCloudMode()) return;
  setStoredToken(null);
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  token?: string | null,
): Promise<T> {
  const headers: Record<string, string> = {};
  if (body !== undefined) {
    headers['content-type'] = 'application/json';
  }
  const bearer = token ?? getStoredToken();
  if (bearer) {
    headers['authorization'] = `Bearer ${bearer}`;
  }
  const resp = await fetch(path, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!resp.ok) {
    let msg = `HTTP ${resp.status}`;
    try {
      const j = await resp.json();
      if (typeof j?.error === 'string') msg = j.error;
    } catch {
      // body wasn't JSON
    }
    throw new Error(msg);
  }
  // 204 No Content
  if (resp.status === 204) {
    return undefined as T;
  }
  return (await resp.json()) as T;
}

// --- Helpers ---------------------------------------------------------------

/** Build the right path for the current mode. */
function p(cloudPath: string, ossPath: string): string {
  return isCloudMode() ? cloudPath : ossPath;
}

// --- Health ----------------------------------------------------------------

export function registryHealth(): Promise<{ status: string }> {
  return request('GET', p('/health', '/api/admin/registry/health'));
}

// --- Auth ------------------------------------------------------------------

export async function registryLogin(
  identifier: string,
  password: string,
): Promise<LoginResponse> {
  if (isCloudMode()) {
    // SaaS login uses {email, password} and returns AuthResponseV2
    const resp = await request<{
      access_token: string;
      user_id: string;
      username: string;
    }>('POST', '/api/v1/auth/login', { email: identifier, password });
    // Store the SaaS token so getStoredToken() picks it up.
    try { localStorage.setItem('auth_token', resp.access_token); } catch { /* */ }
    return {
      token: resp.access_token,
      user: { id: resp.user_id, username: resp.username, email: identifier, is_verified: true, is_admin: false },
    };
  }
  return request('POST', '/api/admin/registry/auth/login', { identifier, password });
}

export async function registryRegister(
  username: string,
  email: string,
  password: string,
): Promise<LoginResponse> {
  if (isCloudMode()) {
    const resp = await request<{
      access_token: string;
      user_id: string;
      username: string;
    }>('POST', '/api/v1/auth/register', { username, email, password });
    try { localStorage.setItem('auth_token', resp.access_token); } catch { /* */ }
    return {
      token: resp.access_token,
      user: { id: resp.user_id, username: resp.username, email, is_verified: false, is_admin: false },
    };
  }
  return request('POST', '/api/admin/registry/auth/register', { username, email, password });
}

export async function registryMe(): Promise<RegistryUser & { claims_exp?: number }> {
  if (isCloudMode()) {
    const d = await request<Record<string, unknown>>(
      'GET',
      '/api/v1/auth/me',
    );
    return {
      id: String(d.user_id ?? d.id ?? ''),
      username: String(d.username ?? ''),
      email: String(d.email ?? ''),
      is_verified: Boolean(d.is_verified ?? true),
      is_admin: Boolean(d.is_admin ?? false),
      created_at: d.created_at as string | undefined,
    };
  }
  return request('GET', '/api/admin/registry/auth/me');
}

// --- Users -----------------------------------------------------------------

export function findUserByEmail(email: string): Promise<RegistryUser> {
  return request('GET', p(
    `/api/v1/users/email/${encodeURIComponent(email)}`,
    `/api/admin/registry/users/email/${encodeURIComponent(email)}`,
  ));
}

export function findUserByUsername(username: string): Promise<RegistryUser> {
  return request('GET', p(
    `/api/v1/users/username/${encodeURIComponent(username)}`,
    `/api/admin/registry/users/username/${encodeURIComponent(username)}`,
  ));
}

export function markUserVerified(userId: string): Promise<RegistryUser> {
  return request('POST', p(
    `/api/v1/users/${userId}/verify`,
    `/api/admin/registry/users/${userId}/verify`,
  ));
}

// --- Orgs ------------------------------------------------------------------

export function findOrgBySlug(slug: string): Promise<RegistryOrg> {
  return request('GET', p(
    `/api/v1/organizations/slug/${encodeURIComponent(slug)}`,
    `/api/admin/registry/orgs/slug/${encodeURIComponent(slug)}`,
  ));
}

export async function createOrg(
  name: string,
  slug: string,
  ownerId: string,
  plan: 'free' | 'pro' | 'team' = 'free',
): Promise<RegistryOrg> {
  if (isCloudMode()) {
    const resp = await request<Record<string, unknown>>(
      'POST', '/api/v1/organizations', { name, slug, plan },
    );
    return {
      id: String(resp.id), name: String(resp.name), slug: String(resp.slug),
      owner_id: String(resp.owner_id), plan: String(resp.plan),
      created_at: resp.created_at as string | undefined,
    };
  }
  return request('POST', '/api/admin/registry/orgs', {
    name, slug, owner_id: ownerId, plan,
  });
}

// --- Org members -----------------------------------------------------------

export async function listOrgMembers(orgId: string): Promise<{ members: RegistryOrgMember[] }> {
  if (isCloudMode()) {
    // SaaS returns a flat array, OSS wraps in {members: [...]}
    const arr = await request<RegistryOrgMember[]>(
      'GET', `/api/v1/organizations/${orgId}/members`,
    );
    return { members: Array.isArray(arr) ? arr : [] };
  }
  return request('GET', `/api/admin/registry/orgs/${orgId}/members`);
}

export function addOrgMember(
  orgId: string,
  userId: string,
  role: 'owner' | 'admin' | 'member' = 'member',
): Promise<RegistryOrgMember> {
  return request('POST', p(
    `/api/v1/organizations/${orgId}/members`,
    `/api/admin/registry/orgs/${orgId}/members`,
  ), { user_id: userId, role });
}

export function updateOrgMemberRole(
  orgId: string,
  userId: string,
  role: 'owner' | 'admin' | 'member',
): Promise<RegistryOrgMember> {
  return request('PATCH', p(
    `/api/v1/organizations/${orgId}/members/${userId}`,
    `/api/admin/registry/orgs/${orgId}/members/${userId}`,
  ), { role });
}

export function removeOrgMember(orgId: string, userId: string): Promise<void> {
  return request('DELETE', p(
    `/api/v1/organizations/${orgId}/members/${userId}`,
    `/api/admin/registry/orgs/${orgId}/members/${userId}`,
  ));
}

// --- Quota -----------------------------------------------------------------

export function getOrgQuota(
  orgId: string,
): Promise<{ org_id: string; quota: Record<string, unknown> }> {
  return request('GET', p(
    `/api/v1/organizations/${orgId}/quota`,
    `/api/admin/registry/orgs/${orgId}/quota`,
  ));
}

export function setOrgQuota(
  orgId: string,
  quota: Record<string, unknown>,
): Promise<{ org_id: string; quota: Record<string, unknown> }> {
  return request('PUT', p(
    `/api/v1/organizations/${orgId}/quota`,
    `/api/admin/registry/orgs/${orgId}/quota`,
  ), quota);
}

// --- API tokens ------------------------------------------------------------

export async function createApiToken(
  orgId: string,
  name: string,
  scopes: string[],
  userId?: string,
): Promise<CreateApiTokenResponse> {
  if (isCloudMode()) {
    // SaaS POST /api/v1/tokens takes {name, scopes, expires_at} and
    // returns {token, token_id, token_prefix, name, scopes, ...}.
    const resp = await request<Record<string, unknown>>(
      'POST', '/api/v1/tokens', { name, scopes },
    );
    return {
      token: String(resp.token),
      id: String(resp.token_id),
      org_id: orgId,
      user_id: userId ?? null,
      name: String(resp.name),
      token_prefix: String(resp.token_prefix),
      scopes: resp.scopes as string[],
      created_at: String(resp.created_at),
    };
  }
  return request('POST', `/api/admin/registry/orgs/${orgId}/tokens`, {
    name, scopes, user_id: userId,
  });
}

// --- Invitations -----------------------------------------------------------

export interface Invitation {
  org_id: string;
  email: string;
  role: string;
  token?: string;
}

export function createInvitation(
  orgId: string,
  email: string,
  role: 'owner' | 'admin' | 'member' = 'member',
): Promise<Invitation> {
  return request('POST', p(
    `/api/v1/organizations/${orgId}/invitations`,
    `/api/admin/registry/orgs/${orgId}/invitations`,
  ), { email, role });
}

export function getInvitation(token: string): Promise<Invitation> {
  return request('GET', p(
    `/api/v1/invitations/${encodeURIComponent(token)}`,
    `/api/admin/registry/invitations/${encodeURIComponent(token)}`,
  ));
}

export function acceptInvitation(
  token: string,
  username: string,
  password: string,
): Promise<LoginResponse & { org_id: string; role: string }> {
  return request(
    'POST',
    p(
      `/api/v1/invitations/${encodeURIComponent(token)}/accept`,
      `/api/admin/registry/invitations/${encodeURIComponent(token)}/accept`,
    ),
    { username, password },
  );
}
