import { logger } from '@/utils/logger';
import React from 'react';
import { cn } from '../../utils/cn';

export type StatusType = 'running' | 'warning' | 'error' | 'stopped' | 'info' | 'loading' | 'neutral';

interface StatusBadgeProps {
  status: StatusType;
  className?: string;
  showDot?: boolean;
  size?: 'sm' | 'md' | 'lg';
}

export function StatusBadge({ status, className, showDot = true, size = 'md' }: StatusBadgeProps) {
  const statusConfig = {
    // Include design tokens first, then vanilla Tailwind fallbacks to survive purge/config issues
    running: {
      label: 'Running',
      bg: 'bg-success-50 bg-green-100',
      color: 'text-success-600 text-green-700',
      ring: 'ring-success-600/20 ring-green-300/40',
      dot: 'bg-success-500 bg-green-500',
    },
    warning: {
      label: 'Warning',
      bg: 'bg-warning-50 bg-yellow-100',
      color: 'text-warning-600 text-yellow-700',
      ring: 'ring-warning-600/20 ring-yellow-300/40',
      dot: 'bg-warning-500 bg-yellow-500',
    },
    error: {
      label: 'Error',
      bg: 'bg-danger-50 bg-red-100',
      color: 'text-danger-600 text-red-700',
      ring: 'ring-danger-600/20 ring-red-300/40',
      dot: 'bg-danger-500 bg-red-500',
    },
    stopped: { label: 'Stopped', bg: 'bg-neutral-100', color: 'text-neutral-600', ring: 'ring-neutral-200', dot: 'bg-neutral-400' },
    info: {
      label: 'Info',
      bg: 'bg-info-50 bg-blue-100',
      color: 'text-info-600 text-blue-700',
      ring: 'ring-info-600/20 ring-blue-300/40',
      dot: 'bg-info-500 bg-blue-500',
    },
    loading: { label: 'Loading', bg: 'bg-neutral-100', color: 'text-neutral-600', ring: 'ring-neutral-200', dot: 'bg-neutral-400' },
    neutral: { label: 'Neutral', bg: 'bg-neutral-100', color: 'text-neutral-600', ring: 'ring-neutral-200', dot: 'bg-neutral-400' },
  } as const;

  const cfg = statusConfig[status];
  const sizes = { sm: 'px-2 py-0.5 text-[10px] tracking-wide', md: 'px-2.5 py-1 text-xs', lg: 'px-3 py-1.5 text-sm' } as const;

  // Inline color fallbacks (ensures visible color even if Tailwind classes are purged/miscompiled)
  const fallbackStyle: Record<StatusType, { bg: string; color: string; dot: string }> = {
    running: { bg: '#DCFCE7', color: '#166534', dot: '#10B981' },
    warning: { bg: '#FEF3C7', color: '#92400E', dot: '#F59E0B' },
    error:   { bg: '#FEE2E2', color: '#991B1B', dot: '#EF4444' },
    stopped: { bg: '#F3F4F6', color: '#4B5563', dot: '#9CA3AF' },
    info:    { bg: '#DBEAFE', color: '#1D4ED8', dot: '#3B82F6' },
    loading: { bg: '#F3F4F6', color: '#4B5563', dot: '#9CA3AF' },
    neutral: { bg: '#F3F4F6', color: '#4B5563', dot: '#9CA3AF' },
  };

  return (
    <div
      className={cn(
        'inline-flex items-center rounded-full font-medium transition-all duration-200 ease-in-out',
        'ring-1 shadow-sm',
        cfg.bg,
        cfg.color,
        cfg.ring,
        sizes[size],
        'dark:ring-opacity-20',
        className
      )}
      role="status"
      aria-label={`${cfg.label} status`}
      style={{ backgroundColor: fallbackStyle[status].bg, color: fallbackStyle[status].color }}
    >
      {showDot && (
        <div
          className={cn('mr-1.5 h-2 w-2 rounded-full', status === 'loading' ? 'animate-pulse' : '', cfg.dot)}
          style={{ backgroundColor: fallbackStyle[status].dot }}
        />
      )}
      <span className="font-semibold uppercase" style={{ letterSpacing: size === 'sm' ? '0.04em' : '0.06em' }}>{cfg.label}</span>
    </div>
  );
}
