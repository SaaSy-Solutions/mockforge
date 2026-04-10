import React, { useState } from 'react';
import { useNavigate, Navigate } from 'react-router-dom';

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert } from '@/components/ui/alert';
import {
  registryLogin,
  setStoredToken,
  registryHealth,
  isCloudMode,
} from '@/services/registryAdminApi';

/// Login page for the OSS registry admin (/api/admin/registry/auth/login).
/// In cloud mode this page is unnecessary — the AuthGuard shows the SaaS
/// login and then RegistryAdminPage is rendered inside it. So we redirect.
///
/// Kept intentionally separate from the existing LoginPage that talks to
/// authApi.ts — these are two different auth flows:
///
///   * authApi.ts → local admin UI user store (auth.rs + rbac.rs)
///   * registryAdminApi.ts → SqliteRegistryStore (registry_admin.rs)
///
/// When MOCKFORGE_REGISTRY_DB_URL is set at the binary, both flows are
/// live; operators pick the relevant one. This page is a no-op (404-ish
/// backend response) when the registry admin isn't enabled.
export function RegistryLoginPage() {
  const navigate = useNavigate();
  const cloud = isCloudMode();

  // In cloud mode, skip this page — the user logs in via the normal
  // SaaS LoginForm (rendered by AuthGuard) and then navigates directly
  // to /registry-admin.
  if (cloud) {
    return <Navigate to="/registry-admin" replace />;
  }
  const [identifier, setIdentifier] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [backendUnavailable, setBackendUnavailable] = useState<boolean | null>(null);

  // Check whether the registry admin backend is actually mounted. If
  // MOCKFORGE_REGISTRY_DB_URL is unset, the /health endpoint returns 404
  // and we show an informative banner instead of a generic error.
  React.useEffect(() => {
    let cancelled = false;
    registryHealth()
      .then(() => {
        if (!cancelled) setBackendUnavailable(false);
      })
      .catch(() => {
        if (!cancelled) setBackendUnavailable(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const resp = await registryLogin(identifier, password);
      setStoredToken(resp.token);
      navigate('/registry-admin');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div style={{ maxWidth: 440, margin: '4rem auto' }}>
      <Card>
        <CardHeader>
          <CardTitle>Registry admin sign in</CardTitle>
          <CardDescription>
            Sign in to manage users, organizations, members, and API tokens on
            this MockForge instance. This is a separate login from the main
            admin UI.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {backendUnavailable === true && (
            <Alert variant="destructive" style={{ marginBottom: '1rem' }}>
              The registry admin backend is not enabled on this server. Set
              <code> MOCKFORGE_REGISTRY_DB_URL</code> before starting the admin
              UI to use these endpoints.
            </Alert>
          )}
          <form onSubmit={onSubmit}>
            <div style={{ marginBottom: '1rem' }}>
              <Label htmlFor="identifier">Username or email</Label>
              <Input
                id="identifier"
                type="text"
                autoComplete="username"
                value={identifier}
                onChange={(e) => setIdentifier(e.target.value)}
                required
              />
            </div>
            <div style={{ marginBottom: '1rem' }}>
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </div>
            {error && (
              <Alert variant="destructive" style={{ marginBottom: '1rem' }}>
                {error}
              </Alert>
            )}
            <Button type="submit" disabled={loading || backendUnavailable === true}>
              {loading ? 'Signing in...' : 'Sign in'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

export default RegistryLoginPage;
