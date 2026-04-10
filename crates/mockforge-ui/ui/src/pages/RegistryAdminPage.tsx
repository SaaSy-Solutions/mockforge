import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert } from '@/components/ui/alert';
import {
  registryMe,
  findUserByEmail,
  findOrgBySlug,
  createOrg,
  createInvitation,
  clearStoredToken,
  getStoredToken,
  isCloudMode,
  type RegistryUser,
  type RegistryOrg,
} from '@/services/registryAdminApi';

/// Admin dashboard for registry management. In **cloud mode** it lives
/// inside the standard AuthGuard (SaaS login) and reads the SaaS JWT.
/// In **self-hosted mode** it uses its own JWT from /registry-login.
export function RegistryAdminPage() {
  const navigate = useNavigate();
  const [me, setMe] = useState<RegistryUser | null>(null);
  const [meError, setMeError] = useState<string | null>(null);
  const [tab, setTab] = useState<'self' | 'lookup' | 'create'>('self');
  const cloud = isCloudMode();

  React.useEffect(() => {
    // In cloud mode, AuthGuard already verified the user is logged in
    // via the SaaS flow — no separate token check needed.
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
      // In cloud mode, go back to the main app — AuthGuard will show
      // the SaaS login if the session expired.
      navigate('/dashboard');
    } else {
      clearStoredToken();
      navigate('/registry-login');
    }
  }

  return (
    <div style={{ maxWidth: 880, margin: '2rem auto' }}>
      <Card>
        <CardHeader>
          <CardTitle>Registry admin</CardTitle>
          <CardDescription>
            SQLite-backed OSS admin for users, organizations, members, and API
            tokens. All operations dispatch through the shared{' '}
            <code>RegistryStore</code> trait and talk to{' '}
            <code>/api/admin/registry/*</code>.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
            <Button
              variant={tab === 'self' ? 'default' : 'outline'}
              onClick={() => setTab('self')}
            >
              Signed in as
            </Button>
            <Button
              variant={tab === 'lookup' ? 'default' : 'outline'}
              onClick={() => setTab('lookup')}
            >
              Look up
            </Button>
            <Button
              variant={tab === 'create' ? 'default' : 'outline'}
              onClick={() => setTab('create')}
            >
              Create
            </Button>
            <div style={{ marginLeft: 'auto' }}>
              <Button variant="outline" onClick={logout}>
                Sign out
              </Button>
            </div>
          </div>

          {tab === 'self' && <SelfTab user={me} error={meError} />}
          {tab === 'lookup' && <LookupTab />}
          {tab === 'create' && <CreateTab currentUser={me} />}
        </CardContent>
      </Card>
    </div>
  );
}

function SelfTab({ user, error }: { user: RegistryUser | null; error: string | null }) {
  if (error) {
    return <Alert variant="destructive">{error}</Alert>;
  }
  if (!user) {
    return <p>Loading…</p>;
  }
  return (
    <div style={{ lineHeight: 1.8 }}>
      <p>
        <strong>ID:</strong> <code>{user.id}</code>
      </p>
      <p>
        <strong>Username:</strong> {user.username}
      </p>
      <p>
        <strong>Email:</strong> {user.email}
      </p>
      <p>
        <strong>Verified:</strong> {user.is_verified ? 'yes' : 'no'}
      </p>
      <p>
        <strong>Admin:</strong> {user.is_admin ? 'yes' : 'no'}
      </p>
    </div>
  );
}

function LookupTab() {
  const [email, setEmail] = useState('');
  const [slug, setSlug] = useState('');
  const [user, setUser] = useState<RegistryUser | null>(null);
  const [org, setOrg] = useState<RegistryOrg | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function lookupUser(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setUser(null);
    try {
      setUser(await findUserByEmail(email));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function lookupOrg(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setOrg(null);
    try {
      setOrg(await findOrgBySlug(slug));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div>
      <form onSubmit={lookupUser} style={{ marginBottom: '1.5rem' }}>
        <Label htmlFor="lookup-email">Find user by email</Label>
        <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.25rem' }}>
          <Input
            id="lookup-email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
          <Button type="submit">Look up</Button>
        </div>
        {user && (
          <div style={{ marginTop: '0.5rem' }}>
            <code>{JSON.stringify(user, null, 2)}</code>
          </div>
        )}
      </form>

      <form onSubmit={lookupOrg}>
        <Label htmlFor="lookup-slug">Find org by slug</Label>
        <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.25rem' }}>
          <Input
            id="lookup-slug"
            value={slug}
            onChange={(e) => setSlug(e.target.value)}
          />
          <Button type="submit">Look up</Button>
        </div>
        {org && (
          <div style={{ marginTop: '0.5rem' }}>
            <code>{JSON.stringify(org, null, 2)}</code>
          </div>
        )}
      </form>

      {error && (
        <Alert variant="destructive" style={{ marginTop: '1rem' }}>
          {error}
        </Alert>
      )}
    </div>
  );
}

function CreateTab({ currentUser }: { currentUser: RegistryUser | null }) {
  const [orgName, setOrgName] = useState('');
  const [orgSlug, setOrgSlug] = useState('');
  const [created, setCreated] = useState<RegistryOrg | null>(null);
  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteToken, setInviteToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function onCreate(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setCreated(null);
    if (!currentUser) {
      setError('not signed in');
      return;
    }
    try {
      setCreated(await createOrg(orgName, orgSlug, currentUser.id, 'free'));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function onInvite(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setInviteToken(null);
    if (!created) {
      setError('create an org first');
      return;
    }
    try {
      const inv = await createInvitation(created.id, inviteEmail, 'member');
      setInviteToken(inv.token ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div>
      <form onSubmit={onCreate} style={{ marginBottom: '2rem' }}>
        <h3>Create organization</h3>
        <div style={{ marginBottom: '0.75rem' }}>
          <Label htmlFor="org-name">Name</Label>
          <Input
            id="org-name"
            value={orgName}
            onChange={(e) => setOrgName(e.target.value)}
            required
          />
        </div>
        <div style={{ marginBottom: '0.75rem' }}>
          <Label htmlFor="org-slug">Slug</Label>
          <Input
            id="org-slug"
            value={orgSlug}
            onChange={(e) => setOrgSlug(e.target.value)}
            required
          />
        </div>
        <Button type="submit">Create</Button>
        {created && (
          <Alert style={{ marginTop: '1rem' }}>
            Created org <code>{created.slug}</code> ({created.id})
          </Alert>
        )}
      </form>

      <form onSubmit={onInvite}>
        <h3>Invite a user</h3>
        <p style={{ fontSize: '0.9em', opacity: 0.8 }}>
          You must create an org above first; invitations are tied to it.
        </p>
        <div style={{ marginBottom: '0.75rem' }}>
          <Label htmlFor="invite-email">Email</Label>
          <Input
            id="invite-email"
            type="email"
            value={inviteEmail}
            onChange={(e) => setInviteEmail(e.target.value)}
            required
          />
        </div>
        <Button type="submit" disabled={!created}>
          Create invitation
        </Button>
        {inviteToken && (
          <Alert style={{ marginTop: '1rem' }}>
            Invitation token (share with invitee):
            <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}>
              {inviteToken}
            </pre>
          </Alert>
        )}
      </form>

      {error && (
        <Alert variant="destructive" style={{ marginTop: '1rem' }}>
          {error}
        </Alert>
      )}
    </div>
  );
}

export default RegistryAdminPage;
