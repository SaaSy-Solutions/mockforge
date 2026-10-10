import { Link } from 'react-router-dom';
import { ResetPasswordForm } from '../components/auth/recovery/ResetPasswordForm';
import { RequestPasswordResetForm } from '../components/auth/recovery/RequestPasswordResetForm';

/** Public recovery routes match the links sent by the registry's reset email. */
export function PasswordRecoveryPage({ reset = false }: { reset?: boolean }) {
  return <main className="min-h-screen flex items-center justify-center bg-background p-6">
    <div className="w-full max-w-md space-y-6 bg-card border rounded-lg p-6">
      <h1 className="text-2xl font-semibold">{reset ? 'Reset password' : 'Forgot password?'}</h1>
      {reset ? <ResetPasswordForm /> : <RequestPasswordResetForm />}
      <Link to="/login" className="block text-sm text-primary hover:underline">Back to sign in</Link>
    </div>
  </main>;
}
