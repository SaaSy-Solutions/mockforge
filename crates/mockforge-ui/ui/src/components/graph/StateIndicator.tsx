import React from 'react';
import { CheckCircle2, XCircle, Clock, AlertCircle, Loader2 } from 'lucide-react';

interface StateIndicatorProps {
  state: string;
  size?: 'sm' | 'md' | 'lg';
  animated?: boolean;
}

const stateConfig: Record<string, { color: string; icon: React.ReactNode; bgColor: string }> = {
  pending: {
    color: 'text-warning-600',
    icon: <Clock className="h-4 w-4" />,
    bgColor: 'bg-warning-100 dark:bg-warning-900',
  },
  active: {
    color: 'text-success-600',
    icon: <CheckCircle2 className="h-4 w-4" />,
    bgColor: 'bg-success-100 dark:bg-success-900',
  },
  inactive: {
    color: 'text-gray-600',
    icon: <XCircle className="h-4 w-4" />,
    bgColor: 'bg-muted',
  },
  error: {
    color: 'text-danger-600',
    icon: <AlertCircle className="h-4 w-4" />,
    bgColor: 'bg-danger-100 dark:bg-danger-900',
  },
  processing: {
    color: 'text-info-600',
    icon: <Loader2 className="h-4 w-4 animate-spin" />,
    bgColor: 'bg-info-100 dark:bg-info-900',
  },
};

export function StateIndicator({ state, size = 'md', animated = true }: StateIndicatorProps) {
  const normalizedState = state.toLowerCase();
  const config = stateConfig[normalizedState] || stateConfig.inactive;

  const sizeClasses = {
    sm: 'h-3 w-3',
    md: 'h-4 w-4',
    lg: 'h-5 w-5',
  };

  return (
    <div className={`inline-flex items-center gap-1.5 px-2 py-1 rounded-full ${config.bgColor} ${config.color}`}>
      <div className={animated && normalizedState === 'processing' ? 'animate-spin' : ''}>
        {React.cloneElement(config.icon as React.ReactElement, {
          className: sizeClasses[size],
        })}
      </div>
      <span className={`text-xs font-medium capitalize ${size === 'sm' ? 'text-[10px]' : ''}`}>
        {state}
      </span>
    </div>
  );
}
