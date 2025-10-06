import { logger } from '@/utils/logger';
import { useState } from 'react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Logo } from '../ui/Logo';
import { useAuthStore } from '../../stores/useAuthStore';

interface LoginFormProps {
  onSuccess?: () => void;
}

export function LoginForm({ onSuccess }: LoginFormProps) {
  const [credentials, setCredentials] = useState({
    username: '',
    password: '',
  });
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');

  const { login } = useAuthStore();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError('');

    try {
      await login(credentials.username, credentials.password);
      onSuccess?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleDemoLogin = (role: 'admin' | 'viewer') => {
    const demoCredentials = {
      admin: { username: 'admin', password: 'admin123' },
      viewer: { username: 'viewer', password: 'viewer123' },
    };
    
    setCredentials(demoCredentials[role]);
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-background">
      <div className="w-full max-w-md space-y-8">
        <div className="text-center space-y-4">
          <div className="flex justify-center">
            <Logo variant="full" size="xl" />
          </div>
          <div>
            <h2 className="text-3xl font-bold">Admin Dashboard</h2>
            <p className="mt-2 text-muted-foreground">
              Sign in to access the admin dashboard
            </p>
          </div>
        </div>

        <div className="bg-card border rounded-lg p-6 space-y-6">
          <form onSubmit={handleSubmit} className="space-y-4">
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
                autoComplete="current-password"
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
              disabled={isLoading || !credentials.username || !credentials.password}
            >
              {isLoading ? 'Signing in...' : 'Sign In'}
            </Button>
          </form>

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
        </div>

        <div className="text-center text-xs text-muted-foreground">
          MockForge Admin UI v2.0 • Powered by React & Shadcn UI
        </div>
      </div>
    </div>
  );
}