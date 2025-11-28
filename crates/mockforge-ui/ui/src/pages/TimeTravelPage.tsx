/**
 * Time Travel Page
 *
 * Comprehensive page for managing all temporal simulation features including:
 * - Time travel controls (enable, disable, advance, scale)
 * - Cron job management
 * - Mutation rule management
 * - Scenario management
 *
 * Follows Apple's Human Interface Guidelines with smooth animations and intuitive UX.
 */

import React, { useState } from 'react';
import { Clock, Play, Pause, RotateCcw, FastForward, Settings, Calendar, Zap, RefreshCw } from 'lucide-react';
import { PageHeader, Section, Alert } from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { Card } from '../components/ui/Card';
import { Badge } from '../components/ui/Badge';
import { Input } from '../components/ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/Tabs';
import {
  useTimeTravelStatus,
  useEnableTimeTravel,
  useDisableTimeTravel,
  useAdvanceTime,
  useSetTimeScale,
  useResetTimeTravel,
  useCronJobs,
  useMutationRules,
} from '../hooks/useApi';
import { cn } from '../utils/cn';

export function TimeTravelPage() {
  const { data: status, isLoading: statusLoading } = useTimeTravelStatus();
  const { data: cronData, isLoading: cronLoading } = useCronJobs();
  const { data: mutationData, isLoading: mutationLoading } = useMutationRules();

  const enableMutation = useEnableTimeTravel();
  const disableMutation = useDisableTimeTravel();
  const advanceMutation = useAdvanceTime();
  const scaleMutation = useSetTimeScale();
  const resetMutation = useResetTimeTravel();

  const [advanceDuration, setAdvanceDuration] = useState('1h');
  const [timeScale, setTimeScale] = useState('1.0');
  const [initialTime, setInitialTime] = useState('');

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
        second: '2-digit',
      });
    } catch {
      return timeStr;
    }
  };

  const handleEnable = () => {
    enableMutation.mutate({
      time: initialTime || undefined,
      scale: timeScale ? parseFloat(timeScale) : undefined,
    });
  };

  const handleAdvance = () => {
    if (advanceDuration) {
      advanceMutation.mutate(advanceDuration);
    }
  };

  const handleSetScale = () => {
    const scale = parseFloat(timeScale);
    if (!isNaN(scale) && scale > 0) {
      scaleMutation.mutate(scale);
    }
  };

  if (statusLoading) {
    return (
      <div className="content-width space-y-8">
        <PageHeader title="Time Travel" subtitle="Temporal simulation controls" />
        <div className="flex items-center justify-center py-12">
          <div className="text-center">
            <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600"></div>
            <p className="mt-4 text-gray-600 dark:text-gray-400">Loading...</p>
          </div>
        </div>
      </div>
    );
  }

  const isEnabled = status?.enabled ?? false;
  const virtualTime = status?.current_time;
  const scaleFactor = status?.scale_factor ?? 1.0;

  return (
    <div className="content-width space-y-8">
      <PageHeader
        title="Time Travel"
        subtitle="Control virtual time for testing time-dependent applications"
        className="space-section"
      />

      {/* Status Card */}
      <Card className="p-6">
        <div className="flex items-start justify-between mb-6">
          <div className="flex items-center gap-3">
            <div
              className={cn(
                'p-3 rounded-xl transition-all duration-200',
                isEnabled
                  ? 'bg-brand-100 text-brand-600 dark:bg-brand-900/30 dark:text-brand-400'
                  : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'
              )}
            >
              <Clock className="h-6 w-6" />
            </div>
            <div>
              <h3 className="text-xl font-semibold text-gray-900 dark:text-gray-100">
                Time Travel Status
              </h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                {isEnabled ? 'Virtual time is active' : 'Using real time'}
              </p>
            </div>
          </div>
          {isEnabled && (
            <Badge variant="success" className="animate-fade-in">
              Active
            </Badge>
          )}
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
          <div className="p-4 rounded-lg bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700">
            <p className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1">
              {isEnabled ? 'Virtual Time' : 'Real Time'}
            </p>
            <p className="text-2xl font-bold text-gray-900 dark:text-gray-100 tabular-nums">
              {formatTime(virtualTime || status?.real_time)}
            </p>
          </div>
          {isEnabled && (
            <>
              <div className="p-4 rounded-lg bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700">
                <p className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1">
                  Time Scale
                </p>
                <p className="text-2xl font-bold text-brand-600 dark:text-brand-400">
                  {scaleFactor.toFixed(1)}x
                </p>
              </div>
              <div className="p-4 rounded-lg bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700">
                <p className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1">
                  Real Time
                </p>
                <p className="text-2xl font-bold text-gray-900 dark:text-gray-100 tabular-nums">
                  {formatTime(status?.real_time)}
                </p>
              </div>
            </>
          )}
        </div>

        {/* Controls */}
        <div className="space-y-4">
          {!isEnabled ? (
            <div className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    Initial Time (ISO 8601, optional)
                  </label>
                  <Input
                    type="text"
                    placeholder="2025-01-01T00:00:00Z"
                    value={initialTime}
                    onChange={(e) => setInitialTime(e.target.value)}
                    className="w-full"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    Time Scale (1.0 = real time)
                  </label>
                  <Input
                    type="number"
                    step="0.1"
                    min="0.1"
                    placeholder="1.0"
                    value={timeScale}
                    onChange={(e) => setTimeScale(e.target.value)}
                    className="w-full"
                  />
                </div>
              </div>
              <Button
                onClick={handleEnable}
                disabled={enableMutation.isPending}
                className="w-full bg-brand-600 hover:bg-brand-700 text-white"
              >
                <Play className="h-4 w-4 mr-2" />
                Enable Time Travel
              </Button>
            </div>
          ) : (
            <div className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    Advance Duration (e.g., "1h", "+1 week", "2d")
                  </label>
                  <div className="flex gap-2">
                    <Input
                      type="text"
                      placeholder="1h"
                      value={advanceDuration}
                      onChange={(e) => setAdvanceDuration(e.target.value)}
                      className="flex-1"
                    />
                    <Button
                      onClick={handleAdvance}
                      disabled={advanceMutation.isPending || !advanceDuration}
                      className="bg-brand-600 hover:bg-brand-700 text-white"
                    >
                      <FastForward className="h-4 w-4 mr-2" />
                      Advance
                    </Button>
                  </div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    Time Scale
                  </label>
                  <div className="flex gap-2">
                    <Input
                      type="number"
                      step="0.1"
                      min="0.1"
                      placeholder="1.0"
                      value={timeScale}
                      onChange={(e) => setTimeScale(e.target.value)}
                      className="flex-1"
                    />
                    <Button
                      onClick={handleSetScale}
                      disabled={scaleMutation.isPending || !timeScale}
                      className="bg-brand-600 hover:bg-brand-700 text-white"
                    >
                      <Zap className="h-4 w-4 mr-2" />
                      Set Scale
                    </Button>
                  </div>
                </div>
              </div>
              <div className="flex gap-2">
                <Button
                  onClick={() => disableMutation.mutate()}
                  disabled={disableMutation.isPending}
                  variant="outline"
                  className="flex-1"
                >
                  <Pause className="h-4 w-4 mr-2" />
                  Disable
                </Button>
                <Button
                  onClick={() => resetMutation.mutate()}
                  disabled={resetMutation.isPending}
                  variant="outline"
                  className="flex-1"
                >
                  <RotateCcw className="h-4 w-4 mr-2" />
                  Reset to Real Time
                </Button>
              </div>
            </div>
          )}
        </div>
      </Card>

      {/* Advanced Features Tabs */}
      <Tabs defaultValue="cron" className="space-y-6">
        <TabsList className="grid w-full grid-cols-3">
          <TabsTrigger value="cron">
            <Calendar className="h-4 w-4 mr-2" />
            Cron Jobs
          </TabsTrigger>
          <TabsTrigger value="mutations">
            <RefreshCw className="h-4 w-4 mr-2" />
            Mutation Rules
          </TabsTrigger>
          <TabsTrigger value="scenarios">
            <Settings className="h-4 w-4 mr-2" />
            Scenarios
          </TabsTrigger>
        </TabsList>

        <TabsContent value="cron" className="space-y-4">
          <Card className="p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
              Scheduled Cron Jobs
            </h3>
            {cronLoading ? (
              <div className="text-center py-8">
                <div className="inline-block animate-spin rounded-full h-6 w-6 border-b-2 border-brand-600"></div>
              </div>
            ) : cronData?.jobs && cronData.jobs.length > 0 ? (
              <div className="space-y-3">
                {(cronData.jobs as any[]).map((job: any) => (
                  <div
                    key={job.id}
                    className="p-4 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50"
                  >
                    <div className="flex items-center justify-between">
                      <div>
                        <h4 className="font-semibold text-gray-900 dark:text-gray-100">{job.name}</h4>
                        <p className="text-sm text-gray-600 dark:text-gray-400">{job.schedule}</p>
                        {job.description && (
                          <p className="text-sm text-gray-500 dark:text-gray-500 mt-1">{job.description}</p>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        <Badge variant={job.enabled ? 'success' : 'default'}>
                          {job.enabled ? 'Enabled' : 'Disabled'}
                        </Badge>
                        <span className="text-xs text-gray-500 dark:text-gray-500">
                          {job.execution_count || 0} executions
                        </span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <Alert type="info" title="No cron jobs" message="Create cron jobs to schedule recurring events." />
            )}
          </Card>
        </TabsContent>

        <TabsContent value="mutations" className="space-y-4">
          <Card className="p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
              Data Mutation Rules
            </h3>
            {mutationLoading ? (
              <div className="text-center py-8">
                <div className="inline-block animate-spin rounded-full h-6 w-6 border-b-2 border-brand-600"></div>
              </div>
            ) : mutationData?.rules && mutationData.rules.length > 0 ? (
              <div className="space-y-3">
                {(mutationData.rules as any[]).map((rule: any) => (
                  <div
                    key={rule.id}
                    className="p-4 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50"
                  >
                    <div className="flex items-center justify-between">
                      <div>
                        <h4 className="font-semibold text-gray-900 dark:text-gray-100">{rule.id}</h4>
                        <p className="text-sm text-gray-600 dark:text-gray-400">
                          Entity: {rule.entity_name}
                        </p>
                        {rule.description && (
                          <p className="text-sm text-gray-500 dark:text-gray-500 mt-1">{rule.description}</p>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        <Badge variant={rule.enabled ? 'success' : 'default'}>
                          {rule.enabled ? 'Enabled' : 'Disabled'}
                        </Badge>
                        <span className="text-xs text-gray-500 dark:text-gray-500">
                          {rule.execution_count || 0} executions
                        </span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <Alert
                type="info"
                title="No mutation rules"
                message="Create mutation rules to automatically modify data based on time triggers."
              />
            )}
          </Card>
        </TabsContent>

        <TabsContent value="scenarios" className="space-y-4">
          <Card className="p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
              Time Travel Scenarios
            </h3>
            <Alert
              type="info"
              title="Scenario Management"
              message="Save and load time travel scenarios to quickly restore specific time states. Use the CLI or API to manage scenarios."
            />
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
