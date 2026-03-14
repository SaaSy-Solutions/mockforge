import { logger } from '@/utils/logger';
import { useState } from 'react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Logo } from '../ui/Logo';
import { useAuthStore } from '../../stores/useAuthStore';
import { authApi } from '../../services/authApi';

interface LoginFormProps {
  onSuccess?: () => void;
}

const isCloud = authApi.isCloud();

export function LoginForm({ onSuccess }: LoginFormProps) {
  const [mode, setMode] = useState<'login' | 'register'>('login');
  const [credentials, setCredentials] = useState({
    username: '',
    email: '',
    password: '',
  });
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');

  const { login, setAuthenticated } = useAuthStore();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError('');

    try {
      if (mode === 'register' && isCloud) {
        // Cloud registration
        const response = await authApi.register(
          credentials.username,
          credentials.email,
          credentials.password,
        );
        setAuthenticated(response.user, response.token, response.refresh_token);
        onSuccess?.();
      } else {
        // Login: use email in cloud mode, username in local mode
        const identifier = isCloud ? credentials.email : credentials.username;
        await login(identifier, credentials.password);
        onSuccess?.();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Authentication failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleDemoLogin = (role: 'admin' | 'viewer') => {
    const demoCredentials = {
      admin: { username: 'admin', email: '', password: 'admin123' },
      viewer: { username: 'viewer', email: '', password: 'viewer123' },
    };
    setCredentials({ ...credentials, ...demoCredentials[role] });
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-background">
      <div className="w-full max-w-md space-y-8">
        <div className="text-center space-y-4">
          <div className="flex justify-center">
            <Logo variant="full" size="xl" />
          </div>
          <div>
            <h2 className="text-3xl font-bold">
              {isCloud ? 'MockForge Cloud' : 'Admin Dashboard'}
            </h2>
            <p className="mt-2 text-muted-foreground">
              {isCloud
                ? mode === 'register'
                  ? 'Create your account to get started'
                  : 'Sign in to manage your mock APIs'
                : 'Sign in to access the admin dashboard'}
            </p>
          </div>
        </div>

        <div className="bg-card border rounded-lg p-6 space-y-6">
          <form onSubmit={handleSubmit} className="space-y-4">
            {isCloud && mode === 'register' && (
              <div className="space-y-2">
                <label htmlFor="username" className="text-sm font-medium">
                  Username
                </label>
                <Input
                  id="username"
                  type="text"
                  value={credentials.username}
                  onChange={(e) => setCredentials(prev => ({ ...prev, username: e.target.value }))}
                  placeholder="Choose a username"
                  required
                  autoComplete="username"
                />
              </div>
            )}

            {isCloud ? (
              <div className="space-y-2">
                <label htmlFor="email" className="text-sm font-medium">
                  Email
                </label>
                <Input
                  id="email"
                  type="email"
                  value={credentials.email}
                  onChange={(e) => setCredentials(prev => ({ ...prev, email: e.target.value }))}
                  placeholder="Enter your email"
                  required
                  autoComplete="email"
                />
              </div>
            ) : (
              <div className="space-y-2">
                <label htmlFor="username" className="text-sm font-medium">
                  Username
                </label>
                <Input
                  id="username"
                  type="text"
                  value={credentials.username}
                  onChange={(e) => setCredentials(prev => ({ ...prev, username: e.target.value }))}
                  placeholder="Enter your username"
                  required
                  autoComplete="username"
                />
              </div>
            )}

            <div className="space-y-2">
              <label htmlFor="password" className="text-sm font-medium">
                Password
              </label>
              <Input
                id="password"
                type="password"
                value={credentials.password}
                onChange={(e) => setCredentials(prev => ({ ...prev, password: e.target.value }))}
                placeholder="Enter your password"
                required
                autoComplete={mode === 'register' ? 'new-password' : 'current-password'}
              />
            </div>

            {error && (
              <div className="text-sm text-destructive bg-destructive/10 border border-destructive/20 rounded p-3">
                {error}
              </div>
            )}

            <Button
              type="submit"
              className="w-full"
              disabled={isLoading || (isCloud ? !credentials.email : !credentials.username) || !credentials.password || (mode === 'register' && !credentials.username)}
            >
              {isLoading
                ? mode === 'register' ? 'Creating account...' : 'Signing in...'
                : mode === 'register' ? 'Create Account' : 'Sign In'}
            </Button>
          </form>

          {isCloud && (
            <div className="text-center text-sm">
              {mode === 'login' ? (
                <p className="text-muted-foreground">
                  Don&apos;t have an account?{' '}
                  <button
                    type="button"
                    onClick={() => { setMode('register'); setError(''); }}
                    className="text-primary hover:underline font-medium"
                  >
                    Sign up
                  </button>
                </p>
              ) : (
                <p className="text-muted-foreground">
                  Already have an account?{' '}
                  <button
                    type="button"
                    onClick={() => { setMode('login'); setError(''); }}
                    className="text-primary hover:underline font-medium"
                  >
                    Sign in
                  </button>
                </p>
              )}
            </div>
          )}

          {!isCloud && (
            <>
              <div className="relative">
                <div className="absolute inset-0 flex items-center">
                  <span className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs uppercase">
                  <span className="bg-card px-2 text-muted-foreground">Demo Accounts</span>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <Button
                  variant="outline"
                  onClick={() => handleDemoLogin('admin')}
                  className="w-full"
                >
                  Demo Admin
                </Button>
                <Button
                  variant="outline"
                  onClick={() => handleDemoLogin('viewer')}
                  className="w-full"
                >
                  Demo Viewer
                </Button>
              </div>

              <div className="text-xs text-muted-foreground text-center space-y-2">
                <div>
                  <strong>Admin:</strong> admin / admin123 (Full access)
                </div>
                <div>
                  <strong>Viewer:</strong> viewer / viewer123 (Read-only)
                </div>
              </div>
            </>
          )}
        </div>

        <div className="text-center text-xs text-muted-foreground">
          {isCloud
            ? 'MockForge Cloud — Mock any API in seconds'
            : 'MockForge Admin UI v2.0 — Powered by React & Shadcn UI'}
        </div>
      </div>
    </div>
  );
}
