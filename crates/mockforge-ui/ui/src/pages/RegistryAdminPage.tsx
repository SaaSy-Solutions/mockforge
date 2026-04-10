import React, { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert } from '@/components/ui/alert';
import {
  registryMe,
  findUserByEmail,
  findUserByUsername,
  findOrgBySlug,
  markUserVerified,
  createOrg,
  createInvitation,
  listOrgMembers,
  addOrgMember,
  updateOrgMemberRole,
  removeOrgMember,
  getOrgQuota,
  setOrgQuota,
  createApiToken,
  clearStoredToken,
  getStoredToken,
  isCloudMode,
  type RegistryUser,
  type RegistryOrg,
  type RegistryOrgMember,
  type CreateApiTokenResponse,
} from '@/services/registryAdminApi';

type Tab = 'self' | 'lookup' | 'members' | 'tokens' | 'quota' | 'create';

export function RegistryAdminPage() {
  const navigate = useNavigate();
  const [me, setMe] = useState<RegistryUser | null>(null);
  const [meError, setMeError] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>('self');
  const cloud = isCloudMode();

  React.useEffect(() => {
    if (!cloud && !getStoredToken()) {
      navigate('/registry-login');
      return;
    }
    registryMe()
      .then(setMe)
      .catch((e: unknown) => setMeError(e instanceof Error ? e.message : String(e)));
  }, [navigate, cloud]);

  function logout() {
    if (cloud) {
      navigate('/dashboard');
    } else {
      clearStoredToken();
      navigate('/registry-login');
    }
  }

  const tabs: { key: Tab; label: string }[] = [
    { key: 'self', label: 'Profile' },
    { key: 'lookup', label: 'Look up' },
    { key: 'members', label: 'Members' },
    { key: 'tokens', label: 'Tokens' },
    { key: 'quota', label: 'Quota' },
    { key: 'create', label: 'Create' },
  ];

  return (
    <div style={{ maxWidth: 960, margin: '2rem auto' }}>
      <Card>
        <CardHeader>
          <CardTitle>Registry admin</CardTitle>
          <CardDescription>
            Manage users, organizations, members, API tokens, quotas, and
            invitations. {cloud ? 'Connected to the cloud Postgres backend.' : 'Connected to the local SQLite backend.'}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.4rem', marginBottom: '1.2rem' }}>
            {tabs.map((t) => (
              <Button
                key={t.key}
                variant={tab === t.key ? 'default' : 'outline'}
                size="sm"
                onClick={() => setTab(t.key)}
              >
                {t.label}
              </Button>
            ))}
            <div style={{ marginLeft: 'auto' }}>
              <Button variant="outline" size="sm" onClick={logout}>
                {cloud ? 'Back to dashboard' : 'Sign out'}
              </Button>
            </div>
          </div>

          {tab === 'self' && <SelfTab user={me} error={meError} />}
          {tab === 'lookup' && <LookupTab />}
          {tab === 'members' && <MembersTab currentUser={me} />}
          {tab === 'tokens' && <TokensTab currentUser={me} />}
          {tab === 'quota' && <QuotaTab />}
          {tab === 'create' && <CreateTab currentUser={me} />}
        </CardContent>
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Profile tab
// ---------------------------------------------------------------------------
function SelfTab({ user, error }: { user: RegistryUser | null; error: string | null }) {
  const [verifyMsg, setVerifyMsg] = useState<string | null>(null);

  if (error) return <Alert variant="destructive">{error}</Alert>;
  if (!user) return <p>Loading…</p>;

  async function handleVerify() {
    if (!user) return;
    try {
      await markUserVerified(user.id);
      setVerifyMsg('User marked as verified');
    } catch (e) {
      setVerifyMsg(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div style={{ lineHeight: 1.8 }}>
      <p><strong>ID:</strong> <code>{user.id}</code></p>
      <p><strong>Username:</strong> {user.username}</p>
      <p><strong>Email:</strong> {user.email}</p>
      <p><strong>Verified:</strong> {user.is_verified ? 'yes' : 'no'}
        {!user.is_verified && (
          <Button variant="outline" size="sm" style={{ marginLeft: '0.5rem' }} onClick={handleVerify}>
            Mark verified
          </Button>
        )}
      </p>
      <p><strong>Admin:</strong> {user.is_admin ? 'yes' : 'no'}</p>
      {verifyMsg && <Alert style={{ marginTop: '0.5rem' }}>{verifyMsg}</Alert>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Look up tab — user by email OR username, org by slug
// ---------------------------------------------------------------------------
function LookupTab() {
  const [email, setEmail] = useState('');
  const [username, setUsername] = useState('');
  const [slug, setSlug] = useState('');
  const [user, setUser] = useState<RegistryUser | null>(null);
  const [org, setOrg] = useState<RegistryOrg | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function lookupByEmail(e: React.FormEvent) {
    e.preventDefault(); setError(null); setUser(null);
    try { setUser(await findUserByEmail(email)); } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  }
  async function lookupByUsername(e: React.FormEvent) {
    e.preventDefault(); setError(null); setUser(null);
    try { setUser(await findUserByUsername(username)); } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  }
  async function lookupOrg(e: React.FormEvent) {
    e.preventDefault(); setError(null); setOrg(null);
    try { setOrg(await findOrgBySlug(slug)); } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  }

  return (
    <div>
      <form onSubmit={lookupByEmail} style={{ marginBottom: '1.2rem' }}>
        <Label>Find user by email</Label>
        <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.25rem' }}>
          <Input type="email" value={email} onChange={(e) => setEmail(e.target.value)} placeholder="alice@example.com" />
          <Button type="submit">Look up</Button>
        </div>
      </form>

      <form onSubmit={lookupByUsername} style={{ marginBottom: '1.2rem' }}>
        <Label>Find user by username</Label>
        <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.25rem' }}>
          <Input value={username} onChange={(e) => setUsername(e.target.value)} placeholder="alice" />
          <Button type="submit">Look up</Button>
        </div>
      </form>

      {user && (
        <Alert style={{ marginBottom: '1.2rem' }}>
          <pre style={{ whiteSpace: 'pre-wrap', fontSize: '0.85em' }}>{JSON.stringify(user, null, 2)}</pre>
        </Alert>
      )}

      <form onSubmit={lookupOrg}>
        <Label>Find org by slug</Label>
        <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.25rem' }}>
          <Input value={slug} onChange={(e) => setSlug(e.target.value)} placeholder="acme" />
          <Button type="submit">Look up</Button>
        </div>
      </form>
      {org && (
        <Alert style={{ marginTop: '0.75rem' }}>
          <pre style={{ whiteSpace: 'pre-wrap', fontSize: '0.85em' }}>{JSON.stringify(org, null, 2)}</pre>
        </Alert>
      )}

      {error && <Alert variant="destructive" style={{ marginTop: '1rem' }}>{error}</Alert>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Members tab — list, add, change role, remove
// ---------------------------------------------------------------------------
function MembersTab({ currentUser }: { currentUser: RegistryUser | null }) {
  const [orgId, setOrgId] = useState('');
  const [members, setMembers] = useState<RegistryOrgMember[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [addUserId, setAddUserId] = useState('');
  const [addRole, setAddRole] = useState('member');

  const load = useCallback(async () => {
    if (!orgId) return;
    setError(null);
    try {
      const resp = await listOrgMembers(orgId);
      setMembers(resp.members);
      setLoaded(true);
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
  }, [orgId]);

  async function handleAdd(e: React.FormEvent) {
    e.preventDefault(); setError(null);
    try {
      await addOrgMember(orgId, addUserId, addRole as 'owner' | 'admin' | 'member');
      setAddUserId('');
      await load();
    } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  }

  async function handleRoleChange(userId: string, newRole: string) {
    setError(null);
    try {
      await updateOrgMemberRole(orgId, userId, newRole as 'owner' | 'admin' | 'member');
      await load();
    } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  }

  async function handleRemove(userId: string) {
    setError(null);
    try {
      await removeOrgMember(orgId, userId);
      await load();
    } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  }

  return (
    <div>
      <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
        <Input placeholder="Organization ID (UUID)" value={orgId} onChange={(e) => setOrgId(e.target.value)} />
        <Button onClick={load}>Load members</Button>
      </div>

      {loaded && members.length === 0 && <p style={{ opacity: 0.7 }}>No members found for this org.</p>}

      {members.length > 0 && (
        <table style={{ width: '100%', borderCollapse: 'collapse', marginBottom: '1rem', fontSize: '0.9em' }}>
          <thead>
            <tr style={{ borderBottom: '1px solid var(--border, #ddd)' }}>
              <th style={{ textAlign: 'left', padding: '0.5rem' }}>User ID</th>
              <th style={{ textAlign: 'left', padding: '0.5rem' }}>Role</th>
              <th style={{ textAlign: 'right', padding: '0.5rem' }}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {members.map((m) => (
              <tr key={m.id} style={{ borderBottom: '1px solid var(--border, #eee)' }}>
                <td style={{ padding: '0.5rem' }}><code style={{ fontSize: '0.8em' }}>{m.user_id}</code></td>
                <td style={{ padding: '0.5rem' }}>
                  <select
                    value={m.role}
                    onChange={(e) => handleRoleChange(m.user_id, e.target.value)}
                    style={{ padding: '0.25rem', borderRadius: '4px', border: '1px solid var(--border, #ccc)' }}
                  >
                    <option value="owner">owner</option>
                    <option value="admin">admin</option>
                    <option value="member">member</option>
                  </select>
                </td>
                <td style={{ padding: '0.5rem', textAlign: 'right' }}>
                  <Button variant="outline" size="sm" onClick={() => handleRemove(m.user_id)}>
                    Remove
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {loaded && (
        <form onSubmit={handleAdd} style={{ display: 'flex', gap: '0.5rem', alignItems: 'end' }}>
          <div style={{ flex: 1 }}>
            <Label>Add member (User ID)</Label>
            <Input value={addUserId} onChange={(e) => setAddUserId(e.target.value)} placeholder="UUID of user to add" required />
          </div>
          <div>
            <Label>Role</Label>
            <select value={addRole} onChange={(e) => setAddRole(e.target.value)}
              style={{ display: 'block', padding: '0.5rem', borderRadius: '4px', border: '1px solid var(--border, #ccc)' }}>
              <option value="member">member</option>
              <option value="admin">admin</option>
            </select>
          </div>
          <Button type="submit">Add</Button>
        </form>
      )}

      {error && <Alert variant="destructive" style={{ marginTop: '1rem' }}>{error}</Alert>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tokens tab — create + display plaintext once
// ---------------------------------------------------------------------------
function TokensTab({ currentUser }: { currentUser: RegistryUser | null }) {
  const [orgId, setOrgId] = useState('');
  const [name, setName] = useState('');
  const [scopes, setScopes] = useState('read:packages');
  const [created, setCreated] = useState<CreateApiTokenResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault(); setError(null); setCreated(null);
    if (!orgId) { setError('Organization ID is required'); return; }
    try {
      const scopeList = scopes.split(',').map((s) => s.trim()).filter(Boolean);
      const resp = await createApiToken(orgId, name, scopeList, currentUser?.id);
      setCreated(resp);
    } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  }

  return (
    <div>
      <form onSubmit={handleCreate}>
        <div style={{ marginBottom: '0.75rem' }}>
          <Label>Organization ID</Label>
          <Input value={orgId} onChange={(e) => setOrgId(e.target.value)} placeholder="UUID of org" required />
        </div>
        <div style={{ marginBottom: '0.75rem' }}>
          <Label>Token name</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="ci-deploy" required />
        </div>
        <div style={{ marginBottom: '0.75rem' }}>
          <Label>Scopes (comma-separated)</Label>
          <Input value={scopes} onChange={(e) => setScopes(e.target.value)}
            placeholder="read:packages, publish:packages, deploy:mocks" />
          <p style={{ fontSize: '0.8em', opacity: 0.7, marginTop: '0.25rem' }}>
            Available: read:packages, publish:packages, deploy:mocks, admin:org, read:usage, manage:billing
          </p>
        </div>
        <Button type="submit">Create token</Button>
      </form>

      {created && (
        <Alert style={{ marginTop: '1rem' }}>
          <p><strong>Token created — copy it now, it won't be shown again!</strong></p>
          <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all', marginTop: '0.5rem', padding: '0.75rem', background: 'var(--muted, #f5f5f5)', borderRadius: '4px', fontSize: '0.85em' }}>
            {created.token}
          </pre>
          <p style={{ marginTop: '0.5rem', fontSize: '0.85em' }}>
            Prefix: <code>{created.token_prefix}</code> &middot;
            Scopes: {created.scopes.join(', ')} &middot;
            ID: <code>{created.id}</code>
          </p>
        </Alert>
      )}

      {error && <Alert variant="destructive" style={{ marginTop: '1rem' }}>{error}</Alert>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Quota tab — get / set org quota as JSON
// ---------------------------------------------------------------------------
function QuotaTab() {
  const [orgId, setOrgId] = useState('');
  const [quotaJson, setQuotaJson] = useState('');
  const [current, setCurrent] = useState<Record<string, unknown> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  async function handleLoad() {
    setError(null); setCurrent(null); setSaved(false);
    if (!orgId) { setError('Organization ID is required'); return; }
    try {
      const resp = await getOrgQuota(orgId);
      setCurrent(resp.quota);
      setQuotaJson(JSON.stringify(resp.quota, null, 2));
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
  }

  async function handleSave() {
    setError(null); setSaved(false);
    try {
      const parsed = JSON.parse(quotaJson);
      await setOrgQuota(orgId, parsed);
      setSaved(true);
      await handleLoad();
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
  }

  return (
    <div>
      <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
        <Input placeholder="Organization ID (UUID)" value={orgId} onChange={(e) => setOrgId(e.target.value)} />
        <Button onClick={handleLoad}>Load quota</Button>
      </div>

      {current !== null && (
        <div>
          <Label>Current quota (edit JSON below, then Save):</Label>
          <textarea
            value={quotaJson}
            onChange={(e) => setQuotaJson(e.target.value)}
            rows={8}
            style={{
              width: '100%', fontFamily: 'monospace', fontSize: '0.85em',
              padding: '0.75rem', borderRadius: '4px',
              border: '1px solid var(--border, #ccc)',
              marginTop: '0.25rem', marginBottom: '0.5rem',
            }}
          />
          <Button onClick={handleSave}>Save quota</Button>
          {saved && <span style={{ marginLeft: '0.75rem', color: 'green' }}>Saved!</span>}
        </div>
      )}

      {error && <Alert variant="destructive" style={{ marginTop: '1rem' }}>{error}</Alert>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Create tab — create org + invite
// ---------------------------------------------------------------------------
function CreateTab({ currentUser }: { currentUser: RegistryUser | null }) {
  const [orgName, setOrgName] = useState('');
  const [orgSlug, setOrgSlug] = useState('');
  const [created, setCreated] = useState<RegistryOrg | null>(null);
  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteRole, setInviteRole] = useState('member');
  const [inviteToken, setInviteToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function onCreate(e: React.FormEvent) {
    e.preventDefault(); setError(null); setCreated(null);
    if (!currentUser) { setError('not signed in'); return; }
    try { setCreated(await createOrg(orgName, orgSlug, currentUser.id, 'free')); } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  }

  async function onInvite(e: React.FormEvent) {
    e.preventDefault(); setError(null); setInviteToken(null);
    if (!created) { setError('create an org first'); return; }
    try {
      const inv = await createInvitation(created.id, inviteEmail, inviteRole as 'owner' | 'admin' | 'member');
      setInviteToken(inv.token ?? null);
    } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  }

  return (
    <div>
      <form onSubmit={onCreate} style={{ marginBottom: '2rem' }}>
        <h3 style={{ marginBottom: '0.75rem' }}>Create organization</h3>
        <div style={{ marginBottom: '0.75rem' }}>
          <Label>Name</Label>
          <Input value={orgName} onChange={(e) => setOrgName(e.target.value)} required />
        </div>
        <div style={{ marginBottom: '0.75rem' }}>
          <Label>Slug</Label>
          <Input value={orgSlug} onChange={(e) => setOrgSlug(e.target.value)} required />
        </div>
        <Button type="submit">Create</Button>
        {created && (
          <Alert style={{ marginTop: '1rem' }}>
            Created org <code>{created.slug}</code> — ID: <code>{created.id}</code>
          </Alert>
        )}
      </form>

      <form onSubmit={onInvite}>
        <h3 style={{ marginBottom: '0.75rem' }}>Invite a user</h3>
        <p style={{ fontSize: '0.85em', opacity: 0.7, marginBottom: '0.75rem' }}>
          Create an org above first — invitations are scoped to it.
        </p>
        <div style={{ marginBottom: '0.75rem' }}>
          <Label>Email</Label>
          <Input type="email" value={inviteEmail} onChange={(e) => setInviteEmail(e.target.value)} required />
        </div>
        <div style={{ marginBottom: '0.75rem' }}>
          <Label>Role</Label>
          <select value={inviteRole} onChange={(e) => setInviteRole(e.target.value)}
            style={{ display: 'block', padding: '0.5rem', borderRadius: '4px', border: '1px solid var(--border, #ccc)' }}>
            <option value="member">member</option>
            <option value="admin">admin</option>
          </select>
        </div>
        <Button type="submit" disabled={!created}>Create invitation</Button>
        {inviteToken && (
          <Alert style={{ marginTop: '1rem' }}>
            <p><strong>Invitation token (share with invitee):</strong></p>
            <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all', fontSize: '0.85em', marginTop: '0.5rem' }}>
              {inviteToken}
            </pre>
          </Alert>
        )}
      </form>

      {error && <Alert variant="destructive" style={{ marginTop: '1rem' }}>{error}</Alert>}
    </div>
  );
}

export default RegistryAdminPage;
