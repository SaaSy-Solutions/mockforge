/**
 * Time Travel Widget Component
 *
 * A sleek, modern widget for the dashboard that provides quick controls
 * for time travel functionality. Follows Apple's Human Interface Guidelines
 * with smooth animations and intuitive interactions.
 */

import React, { useState, useEffect, useMemo } from 'react';
import { Clock, Play, Pause, RotateCcw, FastForward, Settings, Calendar, User, ArrowRight } from 'lucide-react';
import { cn } from '../../utils/cn';
import {
  useTimeTravelStatus,
  useEnableTimeTravel,
  useDisableTimeTravel,
  useAdvanceTime,
  useSetTime,
  useSetTimeScale,
  useResetTimeTravel,
  useLivePreviewLifecycleUpdates,
} from '../../hooks/useApi';
import { Button } from '../ui/button';
import { Card } from '../ui/Card';
import { Badge } from '../ui/Badge';
import { Tooltip } from '../ui/Tooltip';
import { Input } from '../ui/input';
import { Slider } from '../ui/slider';

const QUICK_ADVANCE_OPTIONS = [
  { label: '+1h', value: '1h' },
  { label: '+1d', value: '1d' },
  { label: '+1 week', value: '1week' },
  { label: '+1 month', value: '1month' },
];

interface TimeTravelWidgetProps {
  workspace?: string;
}

export function TimeTravelWidget({ workspace = 'default' }: TimeTravelWidgetProps) {
  const { data: status, isLoading } = useTimeTravelStatus();
  const enableMutation = useEnableTimeTravel();
  const disableMutation = useDisableTimeTravel();
  const advanceMutation = useAdvanceTime();
  const setTimeMutation = useSetTime();
  const setScaleMutation = useSetTimeScale();
  const resetMutation = useResetTimeTravel();
  const [isAdvancing, setIsAdvancing] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [dateTimeInput, setDateTimeInput] = useState('');
  const [timeScale, setTimeScale] = useState(1.0);
  const [sliderValue, setSliderValue] = useState(0);
  const [lifecycleUpdates, setLifecycleUpdates] = useState<Array<{ personaId: string; oldState: string; newState: string; time: string }>>([]);

  // Enable live preview of lifecycle updates when time changes
  useLivePreviewLifecycleUpdates(workspace, status?.enabled ?? false);

  // Extract values before early return - hooks must be called before any conditional returns
  const isEnabled = status?.enabled ?? false;
  const virtualTime = status?.current_time;
  const realTime = status?.real_time;
  const scaleFactor = status?.scale_factor ?? 1.0;

  // Calculate time range for slider (30 days before/after current time)
  // This useMemo MUST be called before any early returns to follow Rules of Hooks
  const timeRange = useMemo(() => {
    if (!virtualTime || !realTime) return { min: 0, max: 100, current: 50 };
    const current = new Date(virtualTime).getTime();
    const real = new Date(realTime).getTime();
    const range = 30 * 24 * 60 * 60 * 1000; // 30 days in ms
    const min = real - range;
    const max = real + range;
    const normalized = ((current - min) / (max - min)) * 100;
    return { min: 0, max: 100, current: Math.max(0, Math.min(100, normalized)) };
  }, [virtualTime, realTime]);

  // Sync slider with virtual time
  // This useEffect MUST be called before any early returns to follow Rules of Hooks
  useEffect(() => {
    if (isEnabled && virtualTime) {
      setSliderValue(timeRange.current);
    }
  }, [isEnabled, virtualTime, timeRange.current]);

  // Sync scale with status
  // This useEffect MUST be called before any early returns to follow Rules of Hooks
  useEffect(() => {
    if (status?.scale_factor !== undefined) {
      setTimeScale(status.scale_factor);
    }
  }, [status?.scale_factor]);

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

  // Early return AFTER all hooks have been called
  if (isLoading) {
    return (
      <Card className="p-6 animate-pulse">
        <div className="h-20 bg-gray-200 dark:bg-gray-700 rounded-lg" />
      </Card>
    );
  }

  // Format datetime-local input value
  const getDateTimeLocalValue = (isoString?: string) => {
    if (!isoString) return '';
    try {
      const date = new Date(isoString);
      // Convert to local datetime-local format (YYYY-MM-DDTHH:mm)
      const year = date.getFullYear();
      const month = String(date.getMonth() + 1).padStart(2, '0');
      const day = String(date.getDate()).padStart(2, '0');
      const hours = String(date.getHours()).padStart(2, '0');
      const minutes = String(date.getMinutes()).padStart(2, '0');
      return `${year}-${month}-${day}T${hours}:${minutes}`;
    } catch {
      return '';
    }
  };

  const handleSetTime = async () => {
    if (!dateTimeInput) return;
    try {
      const date = new Date(dateTimeInput);
      if (isNaN(date.getTime())) {
        alert('Invalid date/time');
        return;
      }
      await setTimeMutation.mutateAsync(date.toISOString());
      setDateTimeInput('');
    } catch (error) {
      console.error('Failed to set time:', error);
    }
  };

  const handleSliderChange = async (value: number) => {
    if (!isEnabled || !virtualTime || !realTime) return;
    setSliderValue(value);

    // Calculate new time based on slider position
    const range = 30 * 24 * 60 * 60 * 1000; // 30 days
    const real = new Date(realTime).getTime();
    const min = real - range;
    const max = real + range;
    const newTime = min + (value / 100) * (max - min);

    try {
      await setTimeMutation.mutateAsync(new Date(newTime).toISOString());
    } catch (error) {
      console.error('Failed to set time:', error);
    }
  };

  const handleScaleChange = async (value: number) => {
    setTimeScale(value);
    try {
      await setScaleMutation.mutateAsync(value);
    } catch (error) {
      console.error('Failed to set scale:', error);
    }
  };

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

      {/* Lifecycle State Changes */}
      {isEnabled && (
        <div className="mb-4 p-3 rounded-lg bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 animate-fade-in">
          <div className="flex items-center gap-2 mb-2">
            <User className="h-4 w-4 text-blue-600 dark:text-blue-400" />
            <p className="text-xs font-medium text-blue-700 dark:text-blue-300 uppercase tracking-wide">
              Lifecycle Updates
            </p>
          </div>
          <p className="text-xs text-blue-600 dark:text-blue-400">
            Persona lifecycle states are automatically updated when virtual time advances.
            Check the persona configuration to see current lifecycle states.
          </p>
        </div>
      )}

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

        {/* Advanced Controls Toggle */}
        {isEnabled && (
          <div className="pt-2 border-t border-gray-200 dark:border-gray-700">
            <Button
              variant="ghost"
              size="sm"
              className="w-full text-gray-600 dark:text-gray-400 hover:text-brand-600 dark:hover:text-brand-400 transition-colors duration-200"
              onClick={() => setShowAdvanced(!showAdvanced)}
            >
              <Settings className="h-4 w-4 mr-2" />
              {showAdvanced ? 'Hide' : 'Show'} Advanced Controls
            </Button>
          </div>
        )}

        {/* Advanced Controls */}
        {isEnabled && showAdvanced && (
          <div className="pt-4 space-y-4 animate-fade-in border-t border-gray-200 dark:border-gray-700">
            {/* Time Slider */}
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Time Travel Slider
                </label>
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  ±30 days from real time
                </span>
              </div>
              <Slider
                min={0}
                max={100}
                step={0.1}
                value={sliderValue}
                onChange={handleSliderChange}
                label=""
                showValue={false}
                disabled={setTimeMutation.isPending}
              />
              <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-1">
                <span>Past</span>
                <span>Real Time</span>
                <span>Future</span>
              </div>
            </div>

            {/* Date/Time Picker */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                <Calendar className="h-3 w-3 inline mr-1" />
                Set Specific Time
              </label>
              <div className="flex gap-2">
                <Input
                  type="datetime-local"
                  value={dateTimeInput || getDateTimeLocalValue(virtualTime)}
                  onChange={(e) => setDateTimeInput(e.target.value)}
                  className="flex-1"
                  placeholder="Select date and time"
                />
                <Button
                  onClick={handleSetTime}
                  disabled={!dateTimeInput || setTimeMutation.isPending}
                  size="sm"
                  variant="outline"
                >
                  Set
                </Button>
              </div>
            </div>

            {/* Speed Control */}
            <div>
              <Slider
                min={0.1}
                max={10}
                step={0.1}
                value={timeScale}
                onChange={handleScaleChange}
                label="Time Speed"
                unit="x"
                description="1.0x = real time, 2.0x = 2x speed, etc."
                disabled={setScaleMutation.isPending}
              />
            </div>

            {/* Link to Full Page */}
            <div className="pt-2">
              <Button
                variant="ghost"
                size="sm"
                className="w-full text-gray-600 dark:text-gray-400 hover:text-brand-600 dark:hover:text-brand-400 transition-colors duration-200"
                onClick={() => {
                  // Dispatch navigation event to change active tab
                  // This prevents full page refresh and uses client-side navigation
                  window.dispatchEvent(new CustomEvent('navigate-tab', { detail: { tab: 'time-travel' } }));
                }}
              >
                Open Full Time Travel Page
              </Button>
            </div>
          </div>
        )}
      </div>
    </Card>
  );
}
