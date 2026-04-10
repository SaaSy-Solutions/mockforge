import React, { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert } from '@/components/ui/alert';
import {
  getInvitation,
  acceptInvitation,
  setStoredToken,
  type Invitation,
} from '@/services/registryAdminApi';

/// Page for accepting an org invitation. Rendered at
/// /registry-admin/invite/:token — the token is the JSON-encoded
/// invitation payload that was given to the invitee.
export function RegistryInvitePage() {
  const { token } = useParams<{ token: string }>();
  const navigate = useNavigate();
  const [invite, setInvite] = useState<Invitation | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    if (!token) { setLoadError('No invitation token provided.'); return; }
    getInvitation(decodeURIComponent(token))
      .then(setInvite)
      .catch((e: unknown) => setLoadError(e instanceof Error ? e.message : String(e)));
  }, [token]);

  async function handleAccept(e: React.FormEvent) {
    e.preventDefault();
    if (!token) return;
    setError(null);
    setLoading(true);
    try {
      const resp = await acceptInvitation(decodeURIComponent(token), username, password);
      setStoredToken(resp.token);
      setSuccess(true);
      setTimeout(() => navigate('/registry-admin'), 1500);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div style={{ maxWidth: 480, margin: '4rem auto' }}>
      <Card>
        <CardHeader>
          <CardTitle>Accept invitation</CardTitle>
          <CardDescription>
            You've been invited to join an organization. Create your account
            below to accept.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loadError && (
            <Alert variant="destructive">{loadError}</Alert>
          )}

          {invite && !success && (
            <>
              <div style={{ marginBottom: '1rem', lineHeight: 1.8 }}>
                <p><strong>Organization:</strong> {invite.org_id}</p>
                <p><strong>Email:</strong> {invite.email}</p>
                <p><strong>Role:</strong> {invite.role}</p>
              </div>

              <form onSubmit={handleAccept}>
                <div style={{ marginBottom: '0.75rem' }}>
                  <Label htmlFor="inv-username">Username</Label>
                  <Input
                    id="inv-username"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    required
                  />
                </div>
                <div style={{ marginBottom: '0.75rem' }}>
                  <Label htmlFor="inv-password">Password (min 8 chars)</Label>
                  <Input
                    id="inv-password"
                    type="password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    minLength={8}
                    required
                  />
                </div>
                {error && (
                  <Alert variant="destructive" style={{ marginBottom: '0.75rem' }}>
                    {error}
                  </Alert>
                )}
                <Button type="submit" disabled={loading}>
                  {loading ? 'Accepting…' : 'Accept & create account'}
                </Button>
              </form>
            </>
          )}

          {success && (
            <Alert>
              Account created and logged in! Redirecting to the admin
              dashboard…
            </Alert>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default RegistryInvitePage;
