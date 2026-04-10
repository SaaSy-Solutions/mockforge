import React, { useEffect, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { useAuthStore } from '../../stores/useAuthStore';
import { LoginForm } from './LoginForm';

interface AuthGuardProps {
  children: React.ReactNode;
  requiredRole?: 'admin' | 'viewer';
}

/**
 * Paths that carry their own auth flow and must bypass the SaaS
 * AuthGuard entirely. The registry-admin module uses a separate JWT
 * signed against the SqliteRegistryStore — the SaaS auth store knows
 * nothing about it, so forcing SaaS login on these pages would block
 * the user from ever reaching the registry-admin UI.
 */
const SELF_AUTHED_PREFIXES = ['/registry-login', '/registry-admin'];

export function AuthGuard({ children, requiredRole }: AuthGuardProps) {
  const location = useLocation();
  const { isAuthenticated, user, isLoading, checkAuth } = useAuthStore();
  const [hasCheckedAuth, setHasCheckedAuth] = useState(false);

  // Bypass: let the registry-admin pages handle their own auth.
  const isSelfAuthed = SELF_AUTHED_PREFIXES.some((p) =>
    location.pathname.startsWith(p),
  );

  // Check authentication on mount (always called — React hooks
  // must not be conditional). The check is skipped for self-authed
  // paths in the render logic below, not here.
  useEffect(() => {
    if (!isSelfAuthed) {
      checkAuth().finally(() => setHasCheckedAuth(true));
    } else {
      setHasCheckedAuth(true);
    }
  }, [checkAuth, isSelfAuthed]);

  // Registry-admin pages carry their own JWT auth — render them
  // directly without waiting for the SaaS auth check.
  if (isSelfAuthed) {
    return <>{children}</>;
  }

  // Show loading spinner until initial auth check completes
  if (!hasCheckedAuth || isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="text-center space-y-4">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto" data-testid="loading-spinner"></div>
          <div className="text-muted-foreground">Checking authentication...</div>
        </div>
      </div>
    );
  }

  // Show login form if not authenticated
  if (!isAuthenticated || !user) {
    return <LoginForm />;
  }

  // Check role permissions
  if (requiredRole && !(user.role === 'admin' || user.role === requiredRole)) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="text-center space-y-4">
          <div className="text-6xl">🔒</div>
          <div>
            <h2 className="text-2xl font-bold">Access Denied</h2>
            <p className="text-muted-foreground mt-2">
              You don't have permission to access this resource.
            </p>
            <p className="text-sm text-muted-foreground mt-1">
              Required role: {requiredRole} • Your role: {user.role}
            </p>
          </div>
        </div>
      </div>
    );
  }

  return <>{children}</>;
}
