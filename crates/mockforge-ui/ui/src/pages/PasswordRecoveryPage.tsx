import { useState, type FormEvent } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { authApi } from '../services/authApi';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';

/** Public recovery routes match the links sent by the registry's reset email. */
export function PasswordRecoveryPage({ reset = false }: { reset?: boolean }) {
  const [params] = useSearchParams();
  const token = params.get('token');
  const [email, setEmail] = useState('');
  const [emailInvalid, setEmailInvalid] = useState(false);
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [pending, setPending] = useState(false);
  const [complete, setComplete] = useState(false);
  const [error, setError] = useState('');
  const missingToken = reset && !token;
  const mismatch = confirmation.length > 0 && password !== confirmation;

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (pending || missingToken || (reset && (password.length < 8 || password !== confirmation))) return;
    setPending(true);
    setError('');
    try {
      if (reset) await authApi.resetPassword(token!, password);
      else await authApi.requestPasswordReset(email);
      setComplete(true);
      setPassword('');
      setConfirmation('');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Could not complete your request. Please try again.');
    } finally {
      setPending(false);
    }
  }

  return (
    <main className="min-h-screen flex items-center justify-center bg-background p-6">
      <div className="w-full max-w-md space-y-6 bg-card border rounded-lg p-6">
        <h1 className="text-2xl font-semibold">{reset ? 'Reset password' : 'Forgot password?'}</h1>
        {complete ? (
          <p role="status">{reset ? 'Your password has been reset. Sign in with your new password.' : 'If an account exists for that email address, we have sent a password reset link. Check your inbox.'}</p>
        ) : missingToken ? (
          <p role="alert" className="text-destructive">This reset link is incomplete. Request a new link below.</p>
        ) : (
          <form onSubmit={submit} className="space-y-4">
            {reset ? <>
              <label htmlFor="new-password" className="block text-sm font-medium">New password</label>
              <Input id="new-password" type="password" autoComplete="new-password" required minLength={8}
                value={password} onChange={e => setPassword(e.target.value)} aria-describedby="password-help" />
              <p id="password-help" className="text-sm text-muted-foreground">Use at least 8 characters.</p>
              <label htmlFor="confirm-password" className="block text-sm font-medium">Confirm password</label>
              <Input id="confirm-password" type="password" autoComplete="new-password" required
                value={confirmation} onChange={e => setConfirmation(e.target.value)} aria-invalid={mismatch} aria-describedby={mismatch ? 'password-mismatch' : undefined} />
              {mismatch && <p id="password-mismatch" role="alert" className="text-sm text-destructive">Passwords must match.</p>}
            </> : <>
              <p className="text-sm text-muted-foreground">Enter your account email to request a reset link.</p>
              <label htmlFor="recovery-email" className="block text-sm font-medium">Email</label>
              <Input id="recovery-email" type="email" autoComplete="email" required value={email}
                onChange={e => { setEmail(e.target.value); setEmailInvalid(e.target.value.length > 0 && !e.target.validity.valid); }}
                aria-invalid={emailInvalid} aria-describedby={emailInvalid ? 'recovery-email-error' : undefined} />
              {emailInvalid && <p id="recovery-email-error" role="alert" className="text-sm text-destructive">Enter a valid email address.</p>}
            </>}
            {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
            <Button type="submit" className="w-full" disabled={pending || (reset ? password.length < 8 || password !== confirmation : !email || emailInvalid)}>
              {pending ? 'Submitting…' : reset ? 'Reset password' : 'Send reset link'}
            </Button>
          </form>
        )}
        {reset && !complete && <Link to="/forgot-password" className="block text-sm text-primary hover:underline">Request a new reset link</Link>}
        <Link to="/login" className="block text-sm text-primary hover:underline">Back to sign in</Link>
      </div>
    </main>
  );
}
