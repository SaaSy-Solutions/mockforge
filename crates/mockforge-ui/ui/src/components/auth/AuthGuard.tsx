import React from 'react';
import { useAuthStore } from '../../stores/useAuthStore';
import { LoginForm } from './LoginForm';

interface AuthGuardProps {
  children: React.ReactNode;
  requiredRole?: 'admin' | 'viewer';
}

export function AuthGuard({ children, requiredRole }: AuthGuardProps) {
  const { isAuthenticated, user, loading } = useAuthStore();

  // Show loading spinner while checking authentication
  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="text-center space-y-4">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
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
