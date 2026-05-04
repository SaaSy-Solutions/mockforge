import React, { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Alert } from '@/components/ui/alert';
import { CheckCircle2, XCircle, Mail, Loader2 } from 'lucide-react';
import { apiErrorMessage } from '@/utils/errorHandling';

type Phase = 'verifying' | 'success' | 'failed' | 'resend';

export function EmailVerificationPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const token = searchParams.get('token');

  const [phase, setPhase] = useState<Phase>(token ? 'verifying' : 'resend');
  const [error, setError] = useState<string | null>(null);
  const [resendEmail, setResendEmail] = useState('');
  const [resending, setResending] = useState(false);
  const [resendSent, setResendSent] = useState(false);

  useEffect(() => {
    if (!token) return;

    const verify = async () => {
      try {
        const response = await fetch(
          `/api/v1/auth/verify-email?token=${encodeURIComponent(token)}`,
        );
        if (!response.ok) {
          const errorData = await response.json().catch(() => ({ error: 'Verification failed' }));
          throw new Error(apiErrorMessage(response, errorData, 'Verification failed'));
        }
        setPhase('success');
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Verification failed');
        setPhase('failed');
      }
    };

    verify();
  }, [token]);

  const handleResend = async (e: React.FormEvent) => {
    e.preventDefault();
    setResending(true);
    setError(null);
    setResendSent(false);

    try {
      const authToken = localStorage.getItem('auth_token');
      const response = await fetch('/api/v1/auth/verify-email/resend', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(authToken && { Authorization: `Bearer ${authToken}` }),
        },
        body: JSON.stringify({ email: resendEmail || undefined }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Failed to resend verification' }));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to resend verification'));
      }

      setResendSent(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to resend verification');
    } finally {
      setResending(false);
    }
  };

  return (
    <div className="container mx-auto px-4 py-12 max-w-md">
      {phase === 'verifying' && (
        <Card>
          <CardContent className="pt-8 pb-8 text-center">
            <Loader2 className="mx-auto mb-4 h-12 w-12 animate-spin text-primary" />
            <h2 className="mb-2 text-2xl font-bold">Verifying your email…</h2>
            <p className="text-muted-foreground">This only takes a moment.</p>
          </CardContent>
        </Card>
      )}

      {phase === 'success' && (
        <Card>
          <CardContent className="pt-8 pb-8 text-center">
            <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-success-100 dark:bg-success-900/30">
              <CheckCircle2 className="h-8 w-8 text-success-600 dark:text-success-400" />
            </div>
            <h2 className="mb-2 text-2xl font-bold">Email verified</h2>
            <p className="mb-6 text-muted-foreground">
              Your email is confirmed. You can now access all features of MockForge.
            </p>
            <Button onClick={() => navigate('/dashboard')} className="w-full">
              Continue to dashboard
            </Button>
          </CardContent>
        </Card>
      )}

      {phase === 'failed' && (
        <Card>
          <CardHeader className="text-center">
            <div className="mx-auto mb-2 flex h-16 w-16 items-center justify-center rounded-full bg-danger-100 dark:bg-danger-900/30">
              <XCircle className="h-8 w-8 text-danger-600 dark:text-danger-400" />
            </div>
            <CardTitle>Verification link invalid</CardTitle>
            <CardDescription>
              {error || 'The link may have expired or already been used.'}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button
              variant="outline"
              className="w-full"
              onClick={() => {
                setPhase('resend');
                setError(null);
              }}
            >
              Request a new verification email
            </Button>
          </CardContent>
        </Card>
      )}

      {phase === 'resend' && (
        <Card>
          <CardHeader className="text-center">
            <div className="mx-auto mb-2 flex h-16 w-16 items-center justify-center rounded-full bg-primary/10">
              <Mail className="h-8 w-8 text-primary" />
            </div>
            <CardTitle>Check your email</CardTitle>
            <CardDescription>
              We'll send a verification link. Enter your email if you're not signed in.
            </CardDescription>
          </CardHeader>
          <CardContent>
            {resendSent && (
              <Alert className="mb-4 bg-success-50 dark:bg-success-900/20 border-success-200 dark:border-success-800">
                <span className="text-success-700 dark:text-success-200">
                  Verification email sent. Check your inbox (and spam folder).
                </span>
              </Alert>
            )}
            {error && (
              <Alert className="mb-4 bg-danger-50 dark:bg-danger-900/20 border-danger-200 dark:border-danger-800">
                <span className="text-danger-700 dark:text-danger-200">{error}</span>
              </Alert>
            )}
            <form onSubmit={handleResend} className="space-y-4">
              <div>
                <label htmlFor="resend-email" className="block text-sm font-medium mb-2">
                  Email
                </label>
                <Input
                  id="resend-email"
                  type="email"
                  value={resendEmail}
                  onChange={(e) => setResendEmail(e.target.value)}
                  placeholder="you@company.com"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Leave blank to resend to your signed-in account.
                </p>
              </div>
              <Button type="submit" className="w-full" disabled={resending}>
                {resending ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Sending…
                  </>
                ) : (
                  'Send verification email'
                )}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
