// Registry admin API client.
//
// Talks to the /api/admin/registry/* endpoints exposed by
// crates/mockforge-ui/src/registry_admin.rs (SQLite-backed OSS admin).
// Kept intentionally separate from the existing authApi.ts so the two
// auth flows (local admin UI vs registry admin) don't cross-contaminate.

const TOKEN_STORAGE_KEY = 'mockforge_registry_admin_token';

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

/** Load the JWT from localStorage, or null if not logged in. */
export function getStoredToken(): string | null {
  try {
    return localStorage.getItem(TOKEN_STORAGE_KEY);
  } catch {
    return null;
  }
}

/** Save the JWT to localStorage. */
export function setStoredToken(token: string | null): void {
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

/** Clear the stored JWT (logout). */
export function clearStoredToken(): void {
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

// --- Health ----------------------------------------------------------------

export function registryHealth(): Promise<{ status: string }> {
  return request('GET', '/api/admin/registry/health');
}

// --- Auth ------------------------------------------------------------------

export function registryLogin(
  identifier: string,
  password: string,
): Promise<LoginResponse> {
  return request('POST', '/api/admin/registry/auth/login', { identifier, password });
}

export function registryRegister(
  username: string,
  email: string,
  password: string,
): Promise<LoginResponse> {
  return request('POST', '/api/admin/registry/auth/register', { username, email, password });
}

export function registryMe(): Promise<RegistryUser & { claims_exp?: number }> {
  return request('GET', '/api/admin/registry/auth/me');
}

// --- Users -----------------------------------------------------------------

export function findUserByEmail(email: string): Promise<RegistryUser> {
  return request('GET', `/api/admin/registry/users/email/${encodeURIComponent(email)}`);
}

export function findUserByUsername(username: string): Promise<RegistryUser> {
  return request(
    'GET',
    `/api/admin/registry/users/username/${encodeURIComponent(username)}`,
  );
}

export function markUserVerified(userId: string): Promise<RegistryUser> {
  return request('POST', `/api/admin/registry/users/${userId}/verify`);
}

// --- Orgs ------------------------------------------------------------------

export function findOrgBySlug(slug: string): Promise<RegistryOrg> {
  return request('GET', `/api/admin/registry/orgs/slug/${encodeURIComponent(slug)}`);
}

export function createOrg(
  name: string,
  slug: string,
  ownerId: string,
  plan: 'free' | 'pro' | 'team' = 'free',
): Promise<RegistryOrg> {
  return request('POST', '/api/admin/registry/orgs', {
    name,
    slug,
    owner_id: ownerId,
    plan,
  });
}

// --- Org members -----------------------------------------------------------

export function listOrgMembers(orgId: string): Promise<{ members: RegistryOrgMember[] }> {
  return request('GET', `/api/admin/registry/orgs/${orgId}/members`);
}

export function addOrgMember(
  orgId: string,
  userId: string,
  role: 'owner' | 'admin' | 'member' = 'member',
): Promise<RegistryOrgMember> {
  return request('POST', `/api/admin/registry/orgs/${orgId}/members`, {
    user_id: userId,
    role,
  });
}

export function updateOrgMemberRole(
  orgId: string,
  userId: string,
  role: 'owner' | 'admin' | 'member',
): Promise<RegistryOrgMember> {
  return request('PATCH', `/api/admin/registry/orgs/${orgId}/members/${userId}`, { role });
}

export function removeOrgMember(orgId: string, userId: string): Promise<void> {
  return request('DELETE', `/api/admin/registry/orgs/${orgId}/members/${userId}`);
}

// --- Quota -----------------------------------------------------------------

export function getOrgQuota(
  orgId: string,
): Promise<{ org_id: string; quota: Record<string, unknown> }> {
  return request('GET', `/api/admin/registry/orgs/${orgId}/quota`);
}

export function setOrgQuota(
  orgId: string,
  quota: Record<string, unknown>,
): Promise<{ org_id: string; quota: Record<string, unknown> }> {
  return request('PUT', `/api/admin/registry/orgs/${orgId}/quota`, quota);
}

// --- API tokens ------------------------------------------------------------

export function createApiToken(
  orgId: string,
  name: string,
  scopes: string[],
  userId?: string,
): Promise<CreateApiTokenResponse> {
  return request('POST', `/api/admin/registry/orgs/${orgId}/tokens`, {
    name,
    scopes,
    user_id: userId,
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
  return request('POST', `/api/admin/registry/orgs/${orgId}/invitations`, { email, role });
}

export function getInvitation(token: string): Promise<Invitation> {
  return request('GET', `/api/admin/registry/invitations/${encodeURIComponent(token)}`);
}

export function acceptInvitation(
  token: string,
  username: string,
  password: string,
): Promise<LoginResponse & { org_id: string; role: string }> {
  return request(
    'POST',
    `/api/admin/registry/invitations/${encodeURIComponent(token)}/accept`,
    { username, password },
  );
}
