import React from 'react';
import { useAuthStore } from '../../stores/useAuthStore';

interface RoleGuardProps {
  children: React.ReactNode;
  allowedRoles: ('admin' | 'user' | 'viewer')[];
  fallback?: React.ReactNode;
}

export function RoleGuard({ children, allowedRoles, fallback }: RoleGuardProps) {
  const { user, isAuthenticated } = useAuthStore();

  if (!isAuthenticated || !user) {
    return fallback || null;
  }

  const hasPermission = allowedRoles.includes(user.role);

  if (!hasPermission) {
    return fallback || (
      <div className="text-center p-4 text-muted-foreground">
        <div className="text-4xl mb-2">🔒</div>
        <div>You don't have permission to access this feature.</div>
        <div className="text-xs mt-1">Required: {allowedRoles.join(' or ')} • Your role: {user.role}</div>
      </div>
    );
  }

  return <>{children}</>;
}
