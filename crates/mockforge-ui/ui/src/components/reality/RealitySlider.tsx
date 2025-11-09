/**
 * Reality Slider Component
 *
 * A sleek, modern slider component for adjusting mock environment realism
 * from simple static stubs (level 1) to full production chaos (level 5).
 * Follows Apple's Human Interface Guidelines with smooth animations and
 * intuitive visual feedback.
 */

import React, { useState, useCallback, useEffect } from 'react';
import { Gauge, Zap, Shield, Brain, AlertTriangle, Activity } from 'lucide-react';
import { cn } from '../../utils/cn';
import {
  useRealityLevel,
  useSetRealityLevel,
} from '../../hooks/useApi';
import { useRealityShortcuts } from '../../hooks/useRealityShortcuts';
import { Slider } from '../ui/slider';
import { Card } from '../ui/Card';
import { Badge } from '../ui/Badge';
import { Tooltip } from '../ui/Tooltip';
import { Button } from '../ui/button';
import { toast } from 'sonner';

/**
 * Reality level configuration with visual metadata
 */
const REALITY_LEVELS = [
  {
    value: 1,
    name: 'Static Stubs',
    description: 'Simple, instant responses with no chaos',
    icon: Shield,
    color: 'text-gray-500',
    bgColor: 'bg-gray-100 dark:bg-gray-800',
    borderColor: 'border-gray-300 dark:border-gray-700',
    features: ['No chaos', '0ms latency', 'No AI'],
  },
  {
    value: 2,
    name: 'Light Simulation',
    description: 'Minimal latency, basic intelligence',
    icon: Activity,
    color: 'text-blue-500',
    bgColor: 'bg-blue-50 dark:bg-blue-900/20',
    borderColor: 'border-blue-300 dark:border-blue-700',
    features: ['No chaos', '10-50ms latency', 'Basic AI'],
  },
  {
    value: 3,
    name: 'Moderate Realism',
    description: 'Some chaos, moderate latency, full intelligence',
    icon: Gauge,
    color: 'text-green-500',
    bgColor: 'bg-green-50 dark:bg-green-900/20',
    borderColor: 'border-green-300 dark:border-green-700',
    features: ['5% errors, 10% delays', '50-200ms latency', 'Full AI'],
  },
  {
    value: 4,
    name: 'High Realism',
    description: 'Increased chaos, realistic latency, session state',
    icon: AlertTriangle,
    color: 'text-orange-500',
    bgColor: 'bg-orange-50 dark:bg-orange-900/20',
    borderColor: 'border-orange-300 dark:border-orange-700',
    features: ['10% errors, 20% delays', '100-500ms latency', 'AI + Sessions'],
  },
  {
    value: 5,
    name: 'Production Chaos',
    description: 'Maximum chaos, production-like latency, full features',
    icon: Zap,
    color: 'text-red-500',
    bgColor: 'bg-red-50 dark:bg-red-900/20',
    borderColor: 'border-red-300 dark:border-red-700',
    features: ['15% errors, 30% delays', '200-2000ms latency', 'Full AI + Mutations'],
  },
] as const;

interface RealitySliderProps {
  className?: string;
  compact?: boolean;
}

export function RealitySlider({ className, compact = false }: RealitySliderProps) {
  const { data: realityData, isLoading } = useRealityLevel();
  const setLevelMutation = useSetRealityLevel();
  const [localLevel, setLocalLevel] = useState<number>(3);
  const [isDragging, setIsDragging] = useState(false);

  // Enable keyboard shortcuts for quick level changes
  useRealityShortcuts({
    enabled: !compact, // Only enable shortcuts in full mode
  });

  // Sync local state with server data
  useEffect(() => {
    if (realityData?.level) {
      setLocalLevel(realityData.level);
    }
  }, [realityData]);

  const currentLevel = realityData?.level ?? localLevel;
  const levelConfig = REALITY_LEVELS.find(l => l.value === currentLevel) || REALITY_LEVELS[2];
  const Icon = levelConfig.icon;

  // Debounce timer for committing level changes
  const commitTimerRef = React.useRef<NodeJS.Timeout | null>(null);

  const handleLevelChange = useCallback((newLevel: number) => {
    setLocalLevel(newLevel);
    setIsDragging(true);

    // Clear existing timer
    if (commitTimerRef.current) {
      clearTimeout(commitTimerRef.current);
    }

    // Commit after user stops dragging (300ms delay)
    commitTimerRef.current = setTimeout(() => {
      setIsDragging(false);
      if (newLevel === currentLevel) return;

      const levelConfig = REALITY_LEVELS.find(l => l.value === newLevel) || REALITY_LEVELS[2];
      setLevelMutation.mutate(newLevel, {
        onSuccess: () => {
          toast.success(`Reality level set to ${newLevel}: ${levelConfig.name}`, {
            description: levelConfig.description,
          });
        },
        onError: (error) => {
          toast.error('Failed to set reality level', {
            description: error instanceof Error ? error.message : 'Unknown error',
          });
          // Revert to previous level
          setLocalLevel(currentLevel);
        },
      });
    }, 300);
  }, [currentLevel, setLevelMutation]);

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (commitTimerRef.current) {
        clearTimeout(commitTimerRef.current);
      }
    };
  }, []);

  const handleLevelCommit = useCallback((newLevel: number) => {
    if (commitTimerRef.current) {
      clearTimeout(commitTimerRef.current);
    }
    setIsDragging(false);
    if (newLevel === currentLevel) return;

    const levelConfig = REALITY_LEVELS.find(l => l.value === newLevel) || REALITY_LEVELS[2];
    setLevelMutation.mutate(newLevel, {
      onSuccess: () => {
        toast.success(`Reality level set to ${newLevel}: ${levelConfig.name}`, {
          description: levelConfig.description,
        });
      },
      onError: (error) => {
        toast.error('Failed to set reality level', {
          description: error instanceof Error ? error.message : 'Unknown error',
        });
        // Revert to previous level
        setLocalLevel(currentLevel);
      },
    });
  }, [currentLevel, setLevelMutation]);

  const handleQuickSet = useCallback((level: number) => {
    if (level === currentLevel) return;
    handleLevelCommit(level);
  }, [currentLevel, handleLevelCommit]);

  if (isLoading && !realityData) {
    return (
      <Card className={cn('p-6 animate-pulse', className)}>
        <div className="h-20 bg-gray-200 dark:bg-gray-700 rounded-lg" />
      </Card>
    );
  }

  if (compact) {
    return (
      <div className={cn('flex items-center gap-3', className)}>
        <div className="flex items-center gap-2">
          <Icon className={cn('h-5 w-5', levelConfig.color)} />
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Level {currentLevel}
          </span>
        </div>
        <Slider
          min={1}
          max={5}
          step={1}
          value={localLevel}
          onChange={handleLevelChange}
          className="w-32"
          showValue={false}
        />
      </div>
    );
  }

  return (
    <Card
      className={cn(
        'p-6 transition-all duration-300 ease-out',
        'hover:shadow-lg hover:-translate-y-0.5',
        levelConfig.bgColor,
        `border-2 ${levelConfig.borderColor}`,
        className
      )}
    >
      {/* Header */}
      <div className="flex items-start justify-between mb-6">
        <div className="flex items-center gap-3">
          <div
            className={cn(
              'p-3 rounded-xl transition-all duration-200',
              levelConfig.bgColor,
              levelConfig.color
            )}
          >
            <Icon className="h-6 w-6" />
          </div>
          <div>
            <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
              Reality Slider
            </h3>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              {levelConfig.name}
            </p>
          </div>
        </div>
        <Badge
          variant="default"
          className={cn('text-sm font-semibold', levelConfig.color)}
        >
          Level {currentLevel}
        </Badge>
      </div>

      {/* Main Slider */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-4">
          <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Realism Level
          </label>
          <span className="text-lg font-bold text-gray-900 dark:text-gray-100 tabular-nums">
            {localLevel} / 5
          </span>
        </div>
        <Slider
          min={1}
          max={5}
          step={1}
          value={localLevel}
          onChange={handleLevelChange}
          disabled={setLevelMutation.isPending}
          description={levelConfig.description}
        />

        {/* Level Indicators */}
        <div className="flex items-center justify-between mt-4 px-1">
          {REALITY_LEVELS.map((level) => {
            const LevelIcon = level.icon;
            const isActive = level.value === currentLevel;
            const isHovered = level.value === localLevel && isDragging;

            return (
              <Tooltip
                key={level.value}
                content={
                  <div>
                    <div className="font-semibold mb-1">{level.name}</div>
                    <div className="text-xs text-gray-300">{level.description}</div>
                    <div className="mt-2 text-xs">
                      {level.features.map((feature, idx) => (
                        <div key={idx}>• {feature}</div>
                      ))}
                    </div>
                  </div>
                }
              >
                <button
                  type="button"
                  onClick={() => handleQuickSet(level.value)}
                  disabled={setLevelMutation.isPending}
                  className={cn(
                    'flex flex-col items-center gap-1.5 p-2 rounded-lg transition-all duration-200',
                    'hover:bg-white/50 dark:hover:bg-gray-800/50',
                    'disabled:opacity-50 disabled:cursor-not-allowed',
                    isActive && 'bg-white dark:bg-gray-800 shadow-sm',
                    isHovered && !isActive && 'bg-white/30 dark:bg-gray-800/30'
                  )}
                >
                  <LevelIcon
                    className={cn(
                      'h-5 w-5 transition-all duration-200',
                      isActive ? level.color : 'text-gray-400 dark:text-gray-500',
                      isHovered && !isActive && 'scale-110'
                    )}
                  />
                  <span
                    className={cn(
                      'text-xs font-medium transition-all duration-200',
                      isActive
                        ? 'text-gray-900 dark:text-gray-100'
                        : 'text-gray-500 dark:text-gray-400'
                    )}
                  >
                    {level.value}
                  </span>
                </button>
              </Tooltip>
            );
          })}
        </div>
      </div>

      {/* Current Configuration Display */}
      {realityData && (
        <div className="mt-6 p-4 rounded-lg bg-white/50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700">
          <div className="grid grid-cols-3 gap-4 text-sm">
            <div>
              <p className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1">
                Chaos
              </p>
              <p className="text-sm font-semibold text-gray-900 dark:text-gray-100">
                {realityData.chaos.enabled ? (
                  <>
                    {Math.round(realityData.chaos.error_rate * 100)}% errors
                    <br />
                    {Math.round(realityData.chaos.delay_rate * 100)}% delays
                  </>
                ) : (
                  'Disabled'
                )}
              </p>
            </div>
            <div>
              <p className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1">
                Latency
              </p>
              <p className="text-sm font-semibold text-gray-900 dark:text-gray-100">
                {realityData.latency.base_ms}ms
                {realityData.latency.jitter_ms > 0 && (
                  <> ±{realityData.latency.jitter_ms}ms</>
                )}
              </p>
            </div>
            <div>
              <p className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1">
                MockAI
              </p>
              <p className="text-sm font-semibold text-gray-900 dark:text-gray-100">
                {realityData.mockai.enabled ? (
                  <span className="text-green-600 dark:text-green-400">Enabled</span>
                ) : (
                  <span className="text-gray-500">Disabled</span>
                )}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Loading State */}
      {setLevelMutation.isPending && (
        <div className="mt-4 flex items-center justify-center gap-2 text-sm text-gray-600 dark:text-gray-400">
          <div className="h-4 w-4 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 dark:border-gray-600 dark:border-t-gray-300" />
          <span>Applying reality level...</span>
        </div>
      )}
    </Card>
  );
}
