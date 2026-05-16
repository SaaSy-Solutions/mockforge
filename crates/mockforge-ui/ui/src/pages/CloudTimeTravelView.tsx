/**
 * Cloud Time Travel view (#466 Phase 2).
 *
 * Rendered by `TimeTravelPage` when `isCloudMode()`. Drives the deployment
 * selector + the clock-control surface against `cloudTimeTravelApi` (which
 * proxies over Fly 6PN to the hosted mock's main HTTP port — see
 * `cloudTimeTravel.ts` and `handlers::time_travel`). Cron jobs and mutation
 * rules stay local-only (see #466 Phase 1 commit), so this view exposes
 * only the 7 clock-control endpoints.
 *
 * Pattern mirrors `ResiliencePage`'s cloud branch: useState/useEffect with
 * direct `await cloudTimeTravelApi.*` calls plus a deployment dropdown
 * scoped to the org's `/api/v1/hosted-mocks`. No TanStack Query — that's
 * intentional. The local TimeTravelPage uses query hooks against a
 * singleton runtime; the cloud view needs to re-bind every call to the
 * currently-selected deployment, which Query's queryKey machinery doesn't
 * help with.
 */

import React, { useEffect, useState, useCallback } from 'react';
import { Clock, Play, FastForward, RefreshCw, RotateCcw } from 'lucide-react';
import { PageHeader, Alert } from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { Card } from '../components/ui/Card';
import { Badge } from '../components/ui/Badge';
import { Input } from '../components/ui/input';
import {
  cloudTimeTravelApi,
  type CloudTimeTravelStatus,
  type CloudTimeTravelStatusEnvelope,
  type TimeTravelRuntimeState,
} from '../services/api/cloudTimeTravel';
import { cn } from '../utils/cn';

interface DeploymentSummary {
  id: string;
  name: string;
  status: string;
}

const POLL_MS = 5000;

function formatTime(iso?: string): string {
  if (!iso) return 'Real Time';
  try {
    const d = new Date(iso);
    return d.toLocaleString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  } catch {
    return iso;
  }
}

export const CloudTimeTravelView: React.FC = () => {
  const [deployments, setDeployments] = useState<DeploymentSummary[]>([]);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<string | null>(null);
  const [status, setStatus] = useState<CloudTimeTravelStatus | null>(null);
  const [runtimeState, setRuntimeState] = useState<TimeTravelRuntimeState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionPending, setActionPending] = useState(false);

  // Form state — mirrors local TimeTravelPage so users coming from local
  // mode see the same controls.
  const [initialTime, setInitialTime] = useState('');
  const [initialTimeError, setInitialTimeError] = useState<string | null>(null);
  const [timeScale, setTimeScale] = useState('1.0');
  const [advanceDuration, setAdvanceDuration] = useState('1h');

  // Load deployments once. The first active deployment is auto-selected
  // so the page works without an extra click in the common
  // one-deployment-per-org case.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const token = localStorage.getItem('auth_token');
        const resp = await fetch('/api/v1/hosted-mocks', {
          headers: token ? { Authorization: `Bearer ${token}` } : {},
        });
        if (!resp.ok) {
          if (!cancelled) {
            setError(`Failed to load deployments: ${resp.status}`);
            setLoading(false);
          }
          return;
        }
        const list = (await resp.json()) as DeploymentSummary[];
        if (cancelled) return;
        const items = Array.isArray(list) ? list : [];
        setDeployments(items);
        const active = items.find((d) => d.status === 'active') ?? items[0] ?? null;
        if (active) {
          setSelectedDeploymentId(active.id);
        } else {
          setLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to load deployments');
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Poll status whenever the selected deployment changes. 5s cadence
  // matches the local TimeTravelPage's `refetchInterval` and keeps the
  // page responsive without hammering Fly 6PN.
  useEffect(() => {
    if (!selectedDeploymentId) return;
    let cancelled = false;

    const fetchStatus = async () => {
      try {
        const env: CloudTimeTravelStatusEnvelope =
          await cloudTimeTravelApi.getStatus(selectedDeploymentId);
        if (cancelled) return;
        setRuntimeState(env.runtime_state);
        setStatus(env.data);
        setError(null);
      } catch (err) {
        if (cancelled) return;
        setRuntimeState('unreachable');
        setStatus(null);
        setError(err instanceof Error ? err.message : 'Failed to load status');
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    fetchStatus();
    const t = setInterval(fetchStatus, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, [selectedDeploymentId]);

  const refresh = useCallback(async () => {
    if (!selectedDeploymentId) return;
    try {
      const env = await cloudTimeTravelApi.getStatus(selectedDeploymentId);
      setRuntimeState(env.runtime_state);
      setStatus(env.data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to refresh');
    }
  }, [selectedDeploymentId]);

  // --- Mutation handlers -------------------------------------------------

  const guardDeployment = (): string | null => {
    if (!selectedDeploymentId) {
      setError('Select a deployment first');
      return null;
    }
    return selectedDeploymentId;
  };

  const handleEnable = async () => {
    const id = guardDeployment();
    if (!id) return;
    if (initialTime) {
      const d = new Date(initialTime);
      if (isNaN(d.getTime())) {
        setInitialTimeError('Invalid ISO-8601 date (e.g., 2025-01-01T00:00:00Z)');
        return;
      }
    }
    setInitialTimeError(null);
    setActionPending(true);
    try {
      await cloudTimeTravelApi.enable(
        id,
        initialTime || undefined,
        timeScale ? parseFloat(timeScale) : undefined,
      );
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to enable');
    } finally {
      setActionPending(false);
    }
  };

  const handleDisable = async () => {
    const id = guardDeployment();
    if (!id) return;
    setActionPending(true);
    try {
      await cloudTimeTravelApi.disable(id);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to disable');
    } finally {
      setActionPending(false);
    }
  };

  const handleAdvance = async () => {
    const id = guardDeployment();
    if (!id || !advanceDuration) return;
    setActionPending(true);
    try {
      await cloudTimeTravelApi.advance(id, advanceDuration);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to advance');
    } finally {
      setActionPending(false);
    }
  };

  const handleSetScale = async () => {
    const id = guardDeployment();
    if (!id) return;
    const scale = parseFloat(timeScale);
    if (isNaN(scale) || scale <= 0) {
      setError('Time scale must be a positive number');
      return;
    }
    setActionPending(true);
    try {
      await cloudTimeTravelApi.setScale(id, scale);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to set scale');
    } finally {
      setActionPending(false);
    }
  };

  const handleReset = async () => {
    const id = guardDeployment();
    if (!id) return;
    setActionPending(true);
    try {
      await cloudTimeTravelApi.reset(id);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to reset');
    } finally {
      setActionPending(false);
    }
  };

  // --- Render ------------------------------------------------------------

  if (deployments.length === 0 && !loading) {
    return (
      <div className="content-width space-y-8">
        <PageHeader title="Time Travel" subtitle="Cloud temporal simulation" />
        <Alert
          variant="info"
          title="No deployments yet"
          description="Create a hosted mock first to control its virtual clock from the cloud dashboard."
        />
      </div>
    );
  }

  const isEnabled = status?.enabled ?? false;
  const virtualTime = status?.current_time;
  const scaleFactor = status?.scale_factor ?? 1.0;
  const unreachable = runtimeState === 'unreachable';

  return (
    <div className="content-width space-y-8">
      <PageHeader
        title="Time Travel"
        subtitle="Control virtual time on a hosted mock deployment"
        className="space-section"
      />

      {/* Deployment selector — single-deployment orgs see a passive label;
          multi-deployment orgs get a dropdown. Same pattern as ResiliencePage. */}
      {deployments.length > 1 ? (
        <Card className="p-4">
          <label className="block text-sm font-medium text-foreground mb-2">
            Deployment
          </label>
          <select
            className="w-full px-3 py-2 rounded-lg border border-border bg-background"
            value={selectedDeploymentId ?? ''}
            onChange={(e) => setSelectedDeploymentId(e.target.value)}
          >
            {deployments.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name} ({d.status})
              </option>
            ))}
          </select>
        </Card>
      ) : (
        deployments[0] && (
          <p className="text-sm text-muted-foreground">
            Deployment: <span className="font-medium text-foreground">{deployments[0].name}</span>
            <Badge variant="outline" className="ml-2">{deployments[0].status}</Badge>
          </p>
        )
      )}

      {/* Runtime-unreachable banner — distinct from a hard error so users can
          tell "deployment didn't answer the proxy" from "request failed". */}
      {unreachable && (
        <Alert
          variant="warning"
          title="Deployment not reachable"
          description="The registry can't reach this deployment's runtime over Fly 6PN. Showing last-known disabled state. Retry will keep polling — if this persists, check the deployment's status."
        />
      )}

      {error && !unreachable && (
        <Alert variant="error" title="Error" description={error} />
      )}

      {loading && (
        <div className="flex items-center justify-center py-12">
          <div className="text-center">
            <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600"></div>
            <p className="mt-4 text-muted-foreground">Loading…</p>
          </div>
        </div>
      )}

      {!loading && (
        <Card className="p-6">
          <div className="flex items-start justify-between mb-6">
            <div className="flex items-center gap-3">
              <div
                className={cn(
                  'p-3 rounded-xl transition-all duration-200',
                  isEnabled
                    ? 'bg-brand-100 text-brand-600 dark:bg-brand-900/30 dark:text-brand-400'
                    : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400',
                )}
              >
                <Clock className="h-6 w-6" />
              </div>
              <div>
                <h3 className="text-xl font-semibold text-foreground">Time Travel Status</h3>
                <p className="text-sm text-muted-foreground">
                  {unreachable
                    ? 'Deployment unreachable'
                    : isEnabled
                      ? 'Virtual time is active'
                      : 'Using real time'}
                </p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              {isEnabled && !unreachable && (
                <Badge variant="success">Active</Badge>
              )}
              <Button
                variant="outline"
                size="sm"
                onClick={refresh}
                disabled={!selectedDeploymentId}
                title="Refresh"
              >
                <RefreshCw className="h-4 w-4" />
              </Button>
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
            <div className="p-4 rounded-lg bg-muted/50 border border-border">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-1">
                {isEnabled ? 'Virtual Time' : 'Real Time'}
              </p>
              <p className="text-2xl font-bold text-foreground tabular-nums">
                {formatTime(virtualTime || status?.real_time)}
              </p>
            </div>
            {isEnabled && (
              <>
                <div className="p-4 rounded-lg bg-muted/50 border border-border">
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-1">
                    Time Scale
                  </p>
                  <p className="text-2xl font-bold text-brand-600 dark:text-brand-400">
                    {scaleFactor.toFixed(1)}x
                  </p>
                </div>
                <div className="p-4 rounded-lg bg-muted/50 border border-border">
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-1">
                    Real Time
                  </p>
                  <p className="text-2xl font-bold text-foreground tabular-nums">
                    {formatTime(status?.real_time)}
                  </p>
                </div>
              </>
            )}
          </div>

          {/* Controls — same shape as local TimeTravelPage but bound to the
              cloud handlers. We disable the entire control block when the
              deployment is unreachable so users don't fire mutations
              that'll just bounce back unreachable. */}
          <fieldset
            disabled={unreachable || actionPending || !selectedDeploymentId}
            className="space-y-4 disabled:opacity-60 disabled:pointer-events-none"
          >
            {!isEnabled ? (
              <div className="space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-2">
                      Initial Time (ISO 8601, optional)
                    </label>
                    <Input
                      type="text"
                      placeholder="2025-01-01T00:00:00Z"
                      value={initialTime}
                      onChange={(e) => setInitialTime(e.target.value)}
                      className="w-full"
                    />
                    {initialTimeError && (
                      <p className="mt-1 text-sm text-danger-600 dark:text-danger-400">
                        {initialTimeError}
                      </p>
                    )}
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-2">
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
                    <label className="block text-sm font-medium text-foreground mb-2">
                      Advance Duration (e.g., "1h", "1week", "2d")
                    </label>
                    <div className="flex gap-2">
                      <Input
                        type="text"
                        placeholder="1h"
                        value={advanceDuration}
                        onChange={(e) => setAdvanceDuration(e.target.value)}
                        className="flex-1"
                      />
                      <Button onClick={handleAdvance} variant="outline">
                        <FastForward className="h-4 w-4 mr-1" />
                        Advance
                      </Button>
                    </div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-2">
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
                      <Button onClick={handleSetScale} variant="outline">
                        Set
                      </Button>
                    </div>
                  </div>
                </div>
                <div className="flex gap-2">
                  <Button onClick={handleDisable} variant="outline" className="flex-1">
                    Disable
                  </Button>
                  <Button onClick={handleReset} variant="outline" className="flex-1">
                    <RotateCcw className="h-4 w-4 mr-2" />
                    Reset
                  </Button>
                </div>
              </div>
            )}
          </fieldset>
        </Card>
      )}

      {/* Cron jobs + mutation rules are intentionally NOT exposed in cloud
          mode — see #466 Phase 1 commit. They belong to a local dev
          session's scenario state, not a hosted mock's single-process
          clock. Users who need them should run a local mockforge. */}
      <Alert
        variant="info"
        title="Cron jobs and mutation rules are local-only"
        description="These features manage local scenario state and aren't exposed on hosted-mock deployments. Run a local MockForge instance to use them."
      />
    </div>
  );
};
