// Re-export Alert from DesignSystem
export { Alert } from './DesignSystem';

// Alert sub-components
import React from 'react';

export function AlertTitle({ children, className }: { children: React.ReactNode; className?: string }) {
  return <h4 className={className}>{children}</h4>;
}

export function AlertDescription({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={className}>{children}</div>;
}
