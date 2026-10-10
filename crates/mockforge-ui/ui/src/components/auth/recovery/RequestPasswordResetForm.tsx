import { useState, type FormEvent } from 'react';
import { authApi } from '../../../services/authApi';
import { Button } from '../../ui/button';
import { Input } from '../../ui/input';
import { isInvalidRecoveryEmail } from './recoveryValidation';
import { useRecoverySubmission } from './useRecoverySubmission';

export function RequestPasswordResetForm() {
  const [email, setEmail] = useState('');
  const [invalid, setInvalid] = useState(false);
  const { pending, complete, error, run } = useRecoverySubmission();
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!email || invalid) return;
    void run(() => authApi.requestPasswordReset(email));
  }
  if (complete) return <p role="status">If an account exists for that email address, we have sent a password reset link. Check your inbox.</p>;
  return <form onSubmit={submit} className="space-y-4">
    <p className="text-sm text-muted-foreground">Enter your account email to request a reset link.</p>
    <label htmlFor="recovery-email" className="block text-sm font-medium">Email</label>
    <Input id="recovery-email" type="email" autoComplete="email" required value={email}
      onChange={event => { setEmail(event.target.value); setInvalid(isInvalidRecoveryEmail(event.target)); }}
      aria-invalid={invalid} aria-describedby={invalid ? 'recovery-email-error' : undefined} />
    {invalid && <p id="recovery-email-error" role="alert" className="text-sm text-destructive">Enter a valid email address.</p>}
    {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
    <Button type="submit" className="w-full" disabled={pending || !email || invalid}>{pending ? 'Submitting…' : 'Send reset link'}</Button>
  </form>;
}
