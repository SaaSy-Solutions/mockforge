import { useState, type FormEvent } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { authApi } from '../../../services/authApi';
import { Button } from '../../ui/button';
import { Input } from '../../ui/input';
import { recoveryPasswordValidation } from './recoveryValidation';
import { useRecoverySubmission } from './useRecoverySubmission';

export function ResetPasswordForm() {
  const [params] = useSearchParams();
  const token = params.get('token');
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const { pending, complete, error, run } = useRecoverySubmission();
  const { mismatch, invalid } = recoveryPasswordValidation(password, confirmation);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (invalid || !token) return;
    if (await run(() => authApi.resetPassword(token, password))) { setPassword(''); setConfirmation(''); }
  }
  if (complete) return <p role="status">Your password has been reset. Sign in with your new password.</p>;
  return <>
    {!token ? <p role="alert" className="text-destructive">This reset link is incomplete. Request a new link below.</p> :
      <form onSubmit={submit} className="space-y-4">
        <label htmlFor="new-password" className="block text-sm font-medium">New password</label>
        <Input id="new-password" type="password" autoComplete="new-password" required minLength={8}
          value={password} onChange={event => setPassword(event.target.value)} aria-describedby="password-help" />
        <p id="password-help" className="text-sm text-muted-foreground">Use at least 8 characters.</p>
        <label htmlFor="confirm-password" className="block text-sm font-medium">Confirm password</label>
        <Input id="confirm-password" type="password" autoComplete="new-password" required value={confirmation}
          onChange={event => setConfirmation(event.target.value)} aria-invalid={mismatch} aria-describedby={mismatch ? 'password-mismatch' : undefined} />
        {mismatch && <p id="password-mismatch" role="alert" className="text-sm text-destructive">Passwords must match.</p>}
        {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
        <Button type="submit" className="w-full" disabled={pending || invalid}>{pending ? 'Submitting…' : 'Reset password'}</Button>
      </form>}
    <Link to="/forgot-password" className="block text-sm text-primary hover:underline">Request a new reset link</Link>
  </>;
}
