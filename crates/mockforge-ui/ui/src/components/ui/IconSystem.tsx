import { logger } from '@/utils/logger';
import React from 'react';
import { cn } from '../../utils/cn';
import {
  // Status Icons
  CheckCircle,
  XCircle,
  AlertTriangle,
  Info,
  Clock,

  // System Icons
  Cpu,
  HardDrive,
  Activity,
  Zap,

  // Navigation Icons
  Home,
  Settings,
  FileText,
  BarChart3,
  Database,
  Shield,
  Users,

  // Action Icons
  Plus,
  Minus,
  Edit,
  Trash2,
  Download,
  Upload,
  Copy,
  Eye,
  EyeOff,

  // UI Icons
  ChevronDown,
  ChevronUp,
  ChevronLeft,
  ChevronRight,
  X,
  Menu,
  Search,
  Filter,
  MoreHorizontal,

  // Communication Icons
  Mail,
  Bell,
  MessageSquare,
  Phone,

  type LucideIcon
} from 'lucide-react';

// Standardized icon sizes
export const iconSizes = {
  xs: 'h-3 w-3',
  sm: 'h-4 w-4',
  md: 'h-5 w-5',
  lg: 'h-6 w-6',
  xl: 'h-8 w-8',
  '2xl': 'h-10 w-10',
  '3xl': 'h-12 w-12',
} as const;

// Semantic color mappings
export const iconColors = {
  default: 'text-secondary',
  primary: 'text-primary',
  brand: 'text-brand',
  success: 'text-success',
  warning: 'text-warning',
  danger: 'text-danger',
  muted: 'text-tertiary',
} as const;

interface IconProps {
  icon: LucideIcon;
  size?: keyof typeof iconSizes;
  color?: keyof typeof iconColors;
  className?: string;
  onClick?: () => void;
  'aria-label'?: string;
}

export function Icon({
  icon: IconComponent,
  size = 'md',
  color = 'default',
  className,
  onClick,
  'aria-label': ariaLabel,
  ...props
}: IconProps) {
  return (
    <IconComponent
      className={cn(
        iconSizes[size],
        iconColors[color],
        onClick && 'cursor-pointer hover:opacity-75 transition-opacity',
        className
      )}
      onClick={onClick}
      aria-label={ariaLabel}
      {...props}
    />
  );
}

// Status Icons with predefined styles
export function StatusIcon({
  status,
  size = 'md',
  className
}: {
  status: 'success' | 'error' | 'warning' | 'info' | 'pending';
  size?: keyof typeof iconSizes;
  className?: string;
}) {
  const statusConfig = {
    success: { icon: CheckCircle, color: 'success' as const },
    error: { icon: XCircle, color: 'danger' as const },
    warning: { icon: AlertTriangle, color: 'warning' as const },
    info: { icon: Info, color: 'brand' as const },
    pending: { icon: Clock, color: 'muted' as const },
  };

  const config = statusConfig[status];

  return (
    <Icon
      icon={config.icon}
      size={size}
      color={config.color}
      className={className}
    />
  );
}

// System Metric Icons
export function MetricIcon({
  metric,
  size = 'lg',
  className
}: {
  metric: 'cpu' | 'memory' | 'activity' | 'uptime' | 'performance';
  size?: keyof typeof iconSizes;
  className?: string;
}) {
  const metricConfig = {
    cpu: { icon: Cpu, color: 'primary' as const },
    memory: { icon: HardDrive, color: 'primary' as const },
    activity: { icon: Activity, color: 'primary' as const },
    uptime: { icon: Clock, color: 'primary' as const },
    performance: { icon: Zap, color: 'primary' as const },
  };

  const config = metricConfig[metric];

  return (
    <Icon
      icon={config.icon}
      size={size}
      color={config.color}
      className={className}
    />
  );
}

// Navigation Icons
export function NavIcon({
  nav,
  size = 'md',
  className
}: {
  nav: 'dashboard' | 'services' | 'logs' | 'metrics' | 'fixtures' | 'config' | 'users';
  size?: keyof typeof iconSizes;
  className?: string;
}) {
  const navConfig = {
    dashboard: { icon: Home },
    services: { icon: Database },
    logs: { icon: FileText },
    metrics: { icon: BarChart3 },
    fixtures: { icon: Settings },
    config: { icon: Settings },
    users: { icon: Users },
  };

  const config = navConfig[nav];

  return (
    <Icon
      icon={config.icon}
      size={size}
      color="default"
      className={className}
    />
  );
}

// Action Icons with consistent styling
export function ActionIcon({
  action,
  size = 'sm',
  onClick,
  className,
  'aria-label': ariaLabel
}: {
  action: 'add' | 'remove' | 'edit' | 'delete' | 'download' | 'upload' | 'copy' | 'view' | 'hide';
  size?: keyof typeof iconSizes;
  onClick?: () => void;
  className?: string;
  'aria-label'?: string;
}) {
  const actionConfig = {
    add: { icon: Plus, color: 'success' as const },
    remove: { icon: Minus, color: 'warning' as const },
    edit: { icon: Edit, color: 'primary' as const },
    delete: { icon: Trash2, color: 'danger' as const },
    download: { icon: Download, color: 'primary' as const },
    upload: { icon: Upload, color: 'primary' as const },
    copy: { icon: Copy, color: 'primary' as const },
    view: { icon: Eye, color: 'primary' as const },
    hide: { icon: EyeOff, color: 'muted' as const },
  };

  const config = actionConfig[action];

  return (
    <Icon
      icon={config.icon}
      size={size}
      color={config.color}
      onClick={onClick}
      className={cn('interactive-pulse', className)}
      aria-label={ariaLabel || `${action} action`}
    />
  );
}

// Chevron Icons for directional navigation
export function ChevronIcon({
  direction,
  size = 'sm',
  onClick,
  className
}: {
  direction: 'up' | 'down' | 'left' | 'right';
  size?: keyof typeof iconSizes;
  onClick?: () => void;
  className?: string;
}) {
  const chevronConfig = {
    up: ChevronUp,
    down: ChevronDown,
    left: ChevronLeft,
    right: ChevronRight,
  };

  return (
    <Icon
      icon={chevronConfig[direction]}
      size={size}
      color="default"
      onClick={onClick}
      className={cn(
        'transition-transform duration-200',
        onClick && 'hover:scale-110',
        className
      )}
    />
  );
}

// Export commonly used icons for direct use
export const Icons = {
  // Status
  Success: CheckCircle,
  Error: XCircle,
  Warning: AlertTriangle,
  Info: Info,
  Clock: Clock,

  // System
  Cpu: Cpu,
  Memory: HardDrive,
  Activity: Activity,
  Performance: Zap,

  // Navigation
  Dashboard: Home,
  Services: Database,
  Logs: FileText,
  Metrics: BarChart3,
  Settings: Settings,
  Users: Users,
  Security: Shield,

  // Actions
  Add: Plus,
  Remove: Minus,
  Edit: Edit,
  Delete: Trash2,
  Download: Download,
  Upload: Upload,
  Copy: Copy,
  View: Eye,
  Hide: EyeOff,

  // UI
  Close: X,
  Menu: Menu,
  Search: Search,
  Filter: Filter,
  More: MoreHorizontal,

  // Communication
  Mail: Mail,
  Notification: Bell,
  Message: MessageSquare,
  Phone: Phone,

  // Directional
  ChevronUp: ChevronUp,
  ChevronDown: ChevronDown,
  ChevronLeft: ChevronLeft,
  ChevronRight: ChevronRight,
} as const;
