/**
 * Time Travel Widget Component
 *
 * A sleek, modern widget for the dashboard that provides quick controls
 * for time travel functionality. Follows Apple's Human Interface Guidelines
 * with smooth animations and intuitive interactions.
 */

import React, { useState } from 'react';
import { Clock, Play, Pause, RotateCcw, FastForward, Settings } from 'lucide-react';
import { cn } from '../../utils/cn';
import {
  useTimeTravelStatus,
  useEnableTimeTravel,
  useDisableTimeTravel,
  useAdvanceTime,
  useResetTimeTravel,
} from '../../hooks/useApi';
import { Button } from '../ui/button';
import { Card } from '../ui/Card';
import { Badge } from '../ui/Badge';
import { Tooltip } from '../ui/Tooltip';

const QUICK_ADVANCE_OPTIONS = [
  { label: '+1h', value: '1h' },
  { label: '+1d', value: '1d' },
  { label: '+1 week', value: '1week' },
  { label: '+1 month', value: '1month' },
];

export function TimeTravelWidget() {
  const { data: status, isLoading } = useTimeTravelStatus();
  const enableMutation = useEnableTimeTravel();
  const disableMutation = useDisableTimeTravel();
  const advanceMutation = useAdvanceTime();
  const resetMutation = useResetTimeTravel();
  const [isAdvancing, setIsAdvancing] = useState(false);

  const handleEnable = () => {
    enableMutation.mutate({});
  };

  const handleDisable = () => {
    disableMutation.mutate();
  };

  const handleAdvance = async (duration: string) => {
    setIsAdvancing(true);
    try {
      await advanceMutation.mutateAsync(duration);
    } finally {
      setIsAdvancing(false);
    }
  };

  const handleReset = () => {
    resetMutation.mutate();
  };

  const formatTime = (timeStr?: string) => {
    if (!timeStr) return 'Real Time';
    try {
      const date = new Date(timeStr);
      return date.toLocaleString('en-US', {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return timeStr;
    }
  };

  if (isLoading) {
    return (
      <Card className="p-6 animate-pulse">
        <div className="h-20 bg-gray-200 dark:bg-gray-700 rounded-lg" />
      </Card>
    );
  }

  const isEnabled = status?.enabled ?? false;
  const virtualTime = status?.current_time;
  const scaleFactor = status?.scale_factor ?? 1.0;

  return (
    <Card
      className={cn(
        'p-6 transition-all duration-300 ease-out',
        'hover:shadow-lg hover:-translate-y-0.5',
        isEnabled && 'border-brand-300 dark:border-brand-600 bg-brand-50/50 dark:bg-brand-900/10'
      )}
    >
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <div
            className={cn(
              'p-2.5 rounded-xl transition-all duration-200',
              isEnabled
                ? 'bg-brand-100 text-brand-600 dark:bg-brand-900/30 dark:text-brand-400'
                : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'
            )}
          >
            <Clock className="h-5 w-5" />
          </div>
          <div>
            <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
              Time Travel
            </h3>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              {isEnabled ? 'Virtual time active' : 'Using real time'}
            </p>
          </div>
        </div>
        {isEnabled && (
          <Badge
            variant="success"
            className="animate-fade-in"
          >
            Active
          </Badge>
        )}
      </div>

      {/* Time Display */}
      <div className="mb-4 p-4 rounded-lg bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700">
        <div className="flex items-center justify-between">
          <div className="flex-1">
            <p className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1">
              {isEnabled ? 'Virtual Time' : 'Real Time'}
            </p>
            <p className="text-xl font-bold text-gray-900 dark:text-gray-100 tabular-nums">
              {formatTime(virtualTime || status?.real_time)}
            </p>
          </div>
          {isEnabled && scaleFactor !== 1.0 && (
            <div className="text-right">
              <p className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1">
                Speed
              </p>
              <p className="text-lg font-semibold text-brand-600 dark:text-brand-400">
                {scaleFactor.toFixed(1)}x
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Controls */}
      <div className="space-y-3">
        {/* Enable/Disable Toggle */}
        <div className="flex gap-2">
          {!isEnabled ? (
            <Button
              onClick={handleEnable}
              disabled={enableMutation.isPending}
              className="flex-1 bg-brand-600 hover:bg-brand-700 text-white transition-all duration-200 hover:scale-[1.02] active:scale-[0.98]"
            >
              <Play className="h-4 w-4 mr-2" />
              Enable Time Travel
            </Button>
          ) : (
            <Button
              onClick={handleDisable}
              disabled={disableMutation.isPending}
              variant="outline"
              className="flex-1 border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-800 transition-all duration-200 hover:scale-[1.02] active:scale-[0.98]"
            >
              <Pause className="h-4 w-4 mr-2" />
              Disable
            </Button>
          )}
          {isEnabled && (
            <Tooltip content="Reset to real time">
              <Button
                onClick={handleReset}
                disabled={resetMutation.isPending}
                variant="outline"
                size="icon"
                className="border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-800 transition-all duration-200 hover:scale-[1.02] active:scale-[0.98]"
              >
                <RotateCcw className="h-4 w-4" />
              </Button>
            </Tooltip>
          )}
        </div>

        {/* Quick Advance Buttons */}
        {isEnabled && (
          <div className="grid grid-cols-4 gap-2 animate-fade-in">
            {QUICK_ADVANCE_OPTIONS.map((option) => (
              <Tooltip key={option.value} content={`Advance by ${option.label}`}>
                <Button
                  onClick={() => handleAdvance(option.value)}
                  disabled={isAdvancing || advanceMutation.isPending}
                  variant="outline"
                  size="sm"
                  className="border-gray-300 dark:border-gray-600 hover:bg-brand-50 hover:border-brand-300 dark:hover:bg-brand-900/20 dark:hover:border-brand-600 transition-all duration-200 hover:scale-[1.05] active:scale-[0.95]"
                >
                  <FastForward className="h-3 w-3 mr-1" />
                  {option.label}
                </Button>
              </Tooltip>
            ))}
          </div>
        )}

        {/* Link to Advanced Controls */}
        {isEnabled && (
          <div className="pt-2 border-t border-gray-200 dark:border-gray-700">
            <Button
              variant="ghost"
              size="sm"
              className="w-full text-gray-600 dark:text-gray-400 hover:text-brand-600 dark:hover:text-brand-400 transition-colors duration-200"
              onClick={() => {
                // Navigate to time travel page
                window.location.href = '/time-travel';
              }}
            >
              <Settings className="h-4 w-4 mr-2" />
              Advanced Controls
            </Button>
          </div>
        )}
      </div>
    </Card>
  );
}
