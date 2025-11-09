/**
 * Reality Indicator Component
 *
 * A compact badge component that displays the current reality level
 * with visual styling. Can be used in headers, toolbars, or anywhere
 * a quick reality level indicator is needed.
 */

import React from 'react';
import { Gauge, Zap, Shield, AlertTriangle, Activity } from 'lucide-react';
import { cn } from '../../utils/cn';
import { useRealityLevel } from '../../hooks/useApi';
import { Badge } from '../ui/Badge';
import { Tooltip } from '../ui/Tooltip';

const REALITY_LEVEL_CONFIG = [
  {
    value: 1,
    name: 'Static Stubs',
    icon: Shield,
    color: 'text-gray-500',
    bgColor: 'bg-gray-100 dark:bg-gray-800',
    borderColor: 'border-gray-300 dark:border-gray-700',
  },
  {
    value: 2,
    name: 'Light Simulation',
    icon: Activity,
    color: 'text-blue-500',
    bgColor: 'bg-blue-50 dark:bg-blue-900/20',
    borderColor: 'border-blue-300 dark:border-blue-700',
  },
  {
    value: 3,
    name: 'Moderate Realism',
    icon: Gauge,
    color: 'text-green-500',
    bgColor: 'bg-green-50 dark:bg-green-900/20',
    borderColor: 'border-green-300 dark:border-green-700',
  },
  {
    value: 4,
    name: 'High Realism',
    icon: AlertTriangle,
    color: 'text-orange-500',
    bgColor: 'bg-orange-50 dark:bg-orange-900/20',
    borderColor: 'border-orange-300 dark:border-orange-700',
  },
  {
    value: 5,
    name: 'Production Chaos',
    icon: Zap,
    color: 'text-red-500',
    bgColor: 'bg-red-50 dark:bg-red-900/20',
    borderColor: 'border-red-300 dark:border-red-700',
  },
] as const;

interface RealityIndicatorProps {
  className?: string;
  showIcon?: boolean;
  showLabel?: boolean;
  variant?: 'default' | 'compact' | 'minimal';
}

export function RealityIndicator({
  className,
  showIcon = true,
  showLabel = false,
  variant = 'default',
}: RealityIndicatorProps) {
  const { data: realityData, isLoading } = useRealityLevel();

  if (isLoading || !realityData) {
    return (
      <Badge variant="outline" className={cn('animate-pulse', className)}>
        <div className="h-3 w-8 bg-gray-200 dark:bg-gray-700 rounded" />
      </Badge>
    );
  }

  const level = realityData.level;
  const levelConfig = REALITY_LEVEL_CONFIG.find(l => l.value === level) || REALITY_LEVEL_CONFIG[2];
  const Icon = levelConfig.icon;

  const content = (
    <Badge
      variant="outline"
      className={cn(
        'flex items-center gap-1.5 transition-all duration-200',
        levelConfig.bgColor,
        levelConfig.borderColor,
        className
      )}
    >
      {showIcon && <Icon className={cn('h-3.5 w-3.5', levelConfig.color)} />}
      <span className={cn('font-semibold tabular-nums', levelConfig.color)}>
        {variant === 'minimal' ? level : `L${level}`}
      </span>
      {showLabel && (
        <span className={cn('text-xs', levelConfig.color)}>
          {levelConfig.name}
        </span>
      )}
    </Badge>
  );

  if (variant === 'minimal') {
    return content;
  }

  return (
    <Tooltip
      content={
        <div>
          <div className="font-semibold mb-1">
            Reality Level {level}: {levelConfig.name}
          </div>
          <div className="text-xs text-gray-300">
            {realityData.description}
          </div>
          <div className="mt-2 text-xs space-y-1">
            <div>
              <strong>Chaos:</strong>{' '}
              {realityData.chaos.enabled
                ? `${Math.round(realityData.chaos.error_rate * 100)}% errors, ${Math.round(realityData.chaos.delay_rate * 100)}% delays`
                : 'Disabled'}
            </div>
            <div>
              <strong>Latency:</strong> {realityData.latency.base_ms}ms
              {realityData.latency.jitter_ms > 0 && ` ±${realityData.latency.jitter_ms}ms`}
            </div>
            <div>
              <strong>MockAI:</strong>{' '}
              {realityData.mockai.enabled ? 'Enabled' : 'Disabled'}
            </div>
          </div>
        </div>
      }
    >
      {content}
    </Tooltip>
  );
}
