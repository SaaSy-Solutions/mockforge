/**
 * Error Pattern Editor Component
 * 
 * Allows users to configure error injection patterns:
 * - Burst: Inject N errors within a time interval
 * - Random: Inject errors with a probability
 * - Sequential: Inject errors in a specific sequence
 */

import React, { useState, useEffect } from 'react';
import { ModernCard, Section } from '../ui/DesignSystem';
import { Button } from '../ui/button';
import { Slider } from '../ui/slider';
import { useUpdateErrorPattern } from '../../hooks/useApi';
import { toast } from 'sonner';
import { Zap, AlertCircle, List } from 'lucide-react';

export type ErrorPatternType = 'burst' | 'random' | 'sequential';

export interface ErrorPatternConfig {
  type: ErrorPatternType;
  count?: number;
  interval_ms?: number;
  probability?: number;
  sequence?: number[];
}

interface ErrorPatternEditorProps {
  /** Current pattern configuration */
  currentPattern?: ErrorPatternConfig | null;
  /** Callback when pattern is updated */
  onPatternChange?: (pattern: ErrorPatternConfig | null) => void;
  /** Whether the editor is disabled */
  disabled?: boolean;
}

export function ErrorPatternEditor({
  currentPattern,
  onPatternChange,
  disabled = false,
}: ErrorPatternEditorProps) {
  const [patternType, setPatternType] = useState<ErrorPatternType>(
    currentPattern?.type || 'random'
  );
  const [burstCount, setBurstCount] = useState(currentPattern?.count || 5);
  const [burstInterval, setBurstInterval] = useState(currentPattern?.interval_ms || 1000);
  const [randomProbability, setRandomProbability] = useState(
    currentPattern?.probability || 0.1
  );
  const [sequenceCodes, setSequenceCodes] = useState<string>(
    currentPattern?.sequence?.join(',') || '500,502,503'
  );

  const updatePattern = useUpdateErrorPattern();

  // Update local state when currentPattern changes
  useEffect(() => {
    if (currentPattern) {
      setPatternType(currentPattern.type);
      if (currentPattern.type === 'burst') {
        setBurstCount(currentPattern.count || 5);
        setBurstInterval(currentPattern.interval_ms || 1000);
      } else if (currentPattern.type === 'random') {
        setRandomProbability(currentPattern.probability || 0.1);
      } else if (currentPattern.type === 'sequential') {
        setSequenceCodes(currentPattern.sequence?.join(',') || '500,502,503');
      }
    }
  }, [currentPattern]);

  const handleSave = async () => {
    try {
      let pattern: ErrorPatternConfig;

      switch (patternType) {
        case 'burst':
          pattern = {
            type: 'burst',
            count: burstCount,
            interval_ms: burstInterval,
          };
          break;
        case 'random':
          pattern = {
            type: 'random',
            probability: randomProbability,
          };
          break;
        case 'sequential':
          const codes = sequenceCodes
            .split(',')
            .map((s) => parseInt(s.trim(), 10))
            .filter((n) => !isNaN(n) && n >= 100 && n < 600);
          if (codes.length === 0) {
            toast.error('Please provide at least one valid HTTP status code (100-599)');
            return;
          }
          pattern = {
            type: 'sequential',
            sequence: codes,
          };
          break;
      }

      await updatePattern.mutateAsync(pattern);
      onPatternChange?.(pattern);
      toast.success('Error pattern updated successfully');
    } catch (error: any) {
      toast.error(`Failed to update error pattern: ${error.message || 'Unknown error'}`);
    }
  };

  const handleClear = () => {
    setPatternType('random');
    setBurstCount(5);
    setBurstInterval(1000);
    setRandomProbability(0.1);
    setSequenceCodes('500,502,503');
    onPatternChange?.(null);
  };

  return (
    <ModernCard>
      <div className="space-y-6">
        {/* Pattern Type Selector */}
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
            Pattern Type
          </label>
          <div className="grid grid-cols-3 gap-3">
            <button
              type="button"
              onClick={() => setPatternType('burst')}
              disabled={disabled}
              className={`px-4 py-3 rounded-lg border-2 transition-all ${
                patternType === 'burst'
                  ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20 dark:border-blue-400'
                  : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
              } ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
            >
              <div className="flex flex-col items-center gap-2">
                <Zap className="h-5 w-5" />
                <span className="text-sm font-medium">Burst</span>
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  N errors in interval
                </span>
              </div>
            </button>
            <button
              type="button"
              onClick={() => setPatternType('random')}
              disabled={disabled}
              className={`px-4 py-3 rounded-lg border-2 transition-all ${
                patternType === 'random'
                  ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20 dark:border-blue-400'
                  : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
              } ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
            >
              <div className="flex flex-col items-center gap-2">
                <AlertCircle className="h-5 w-5" />
                <span className="text-sm font-medium">Random</span>
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  Probability-based
                </span>
              </div>
            </button>
            <button
              type="button"
              onClick={() => setPatternType('sequential')}
              disabled={disabled}
              className={`px-4 py-3 rounded-lg border-2 transition-all ${
                patternType === 'sequential'
                  ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20 dark:border-blue-400'
                  : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
              } ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
            >
              <div className="flex flex-col items-center gap-2">
                <List className="h-5 w-5" />
                <span className="text-sm font-medium">Sequential</span>
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  Ordered sequence
                </span>
              </div>
            </button>
          </div>
        </div>

        {/* Pattern-Specific Controls */}
        {patternType === 'burst' && (
          <div className="space-y-4">
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Errors per Burst
                </label>
                <span className="text-sm text-gray-500 dark:text-gray-400">{burstCount}</span>
              </div>
              <Slider
                min={1}
                max={50}
                step={1}
                value={burstCount}
                onChange={(value) => setBurstCount(value)}
                disabled={disabled}
                description="Number of errors to inject in each burst"
              />
            </div>
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Burst Interval
                </label>
                <span className="text-sm text-gray-500 dark:text-gray-400">
                  {burstInterval}ms
                </span>
              </div>
              <Slider
                min={100}
                max={10000}
                step={100}
                value={burstInterval}
                onChange={(value) => setBurstInterval(value)}
                disabled={disabled}
                unit="ms"
                description="Time window for the burst"
              />
            </div>
            <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg text-sm text-gray-700 dark:text-gray-300">
              <strong>Preview:</strong> Will inject {burstCount} errors within{' '}
              {burstInterval}ms, then wait for the next interval.
            </div>
          </div>
        )}

        {patternType === 'random' && (
          <div className="space-y-4">
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Error Probability
                </label>
                <span className="text-sm text-gray-500 dark:text-gray-400">
                  {(randomProbability * 100).toFixed(1)}%
                </span>
              </div>
              <Slider
                min={0}
                max={100}
                step={0.1}
                value={randomProbability * 100}
                onChange={(value) => setRandomProbability(value / 100)}
                disabled={disabled}
                unit="%"
                description="Probability of injecting an error on each request"
              />
            </div>
            <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg text-sm text-gray-700 dark:text-gray-300">
              <strong>Preview:</strong> Each request has a{' '}
              {(randomProbability * 100).toFixed(1)}% chance of receiving an error.
            </div>
          </div>
        )}

        {patternType === 'sequential' && (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Status Code Sequence
              </label>
              <input
                type="text"
                value={sequenceCodes}
                onChange={(e) => setSequenceCodes(e.target.value)}
                disabled={disabled}
                placeholder="500,502,503,504"
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
              />
              <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                Comma-separated HTTP status codes (100-599). Errors will be injected in this order,
                then repeat.
              </p>
            </div>
            <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg text-sm text-gray-700 dark:text-gray-300">
              <strong>Preview:</strong> Will inject errors in sequence:{' '}
              {sequenceCodes
                .split(',')
                .map((s) => s.trim())
                .filter((s) => s)
                .join(' → ') || 'No valid codes'}
              , then repeat.
            </div>
          </div>
        )}

        {/* Action Buttons */}
        <div className="flex items-center gap-3 pt-4 border-t border-gray-200 dark:border-gray-700">
          <Button
            onClick={handleSave}
            disabled={disabled || updatePattern.isPending}
            className="flex-1"
          >
            {updatePattern.isPending ? 'Saving...' : 'Save Pattern'}
          </Button>
          <Button
            variant="outline"
            onClick={handleClear}
            disabled={disabled}
          >
            Clear
          </Button>
        </div>
      </div>
    </ModernCard>
  );
}

