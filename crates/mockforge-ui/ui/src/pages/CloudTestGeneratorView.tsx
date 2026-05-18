/**
 * Cloud Test Generator view (#469 Phase 2).
 *
 * Rendered by `TestGeneratorPage` when `isCloudMode()`. Drives the
 * `cloudTestGeneratorApi` (Phase 1 data plane) for the active workspace
 * — list, create, get, cancel. The actual LLM execution is **not yet
 * implemented** — the background worker that turns 'queued' jobs into
 * 'succeeded' or 'failed' lands in a follow-up PR. This view is honest
 * about that limitation with an inline notice; users can still create
 * jobs (they'll sit in 'queued' until the worker exists) and see the
 * full data shape.
 *
 * Polling: 5s cadence while any job is in a non-terminal state; pauses
 * when all visible jobs are terminal so we don't hammer the registry
 * for no reason.
 */

import React, { useEffect, useState, useCallback, useMemo } from 'react';
import { PageHeader, Alert, Section } from '../components/ui/DesignSystem';
import { Card } from '../components/ui/Card';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { RefreshCw, Play, X, ChevronDown, ChevronRight } from 'lucide-react';
import { useWorkspaceStore } from '../store/workspaceStore';
import {
  cloudTestGeneratorApi,
  subscribeToJobStream,
  type CloudTestGenerationJob,
  type TestGenerationJobStatus,
} from '../services/api/cloudTestGenerator';
import { cn } from '../utils/cn';

const POLL_MS = 5000;
const TERMINAL_STATUSES: TestGenerationJobStatus[] = ['succeeded', 'failed', 'cancelled'];

function statusBadgeVariant(
  status: TestGenerationJobStatus,
): 'default' | 'success' | 'destructive' | 'outline' | 'secondary' {
  switch (status) {
    case 'succeeded':
      return 'success';
    case 'failed':
      return 'destructive';
    case 'cancelled':
      return 'secondary';
    case 'running':
      return 'default';
    case 'queued':
    default:
      return 'outline';
  }
}

function formatRelative(iso: string | null): string {
  if (!iso) return '—';
  try {
    const ts = new Date(iso).getTime();
    const ageMs = Date.now() - ts;
    if (ageMs < 60_000) return `${Math.floor(ageMs / 1000)}s ago`;
    if (ageMs < 3_600_000) return `${Math.floor(ageMs / 60_000)}m ago`;
    if (ageMs < 86_400_000) return `${Math.floor(ageMs / 3_600_000)}h ago`;
    return new Date(iso).toLocaleDateString();
  } catch {
    return iso;
  }
}

export const CloudTestGeneratorView: React.FC = () => {
  const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);
  const workspaceId = activeWorkspace?.id;

  const [jobs, setJobs] = useState<CloudTestGenerationJob[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedJobId, setExpandedJobId] = useState<string | null>(null);

  // Create form
  const [creating, setCreating] = useState(false);
  const [prompt, setPrompt] = useState('');
  const [filterText, setFilterText] = useState('');
  const [createError, setCreateError] = useState<string | null>(null);

  // Pause polling when every visible job is terminal — the worker can't
  // resurrect a finished job, so the cadence buys nothing.
  const hasPendingJob = useMemo(
    () => jobs.some((j) => !TERMINAL_STATUSES.includes(j.status)),
    [jobs],
  );

  const fetchJobs = useCallback(async () => {
    if (!workspaceId) return;
    try {
      const list = await cloudTestGeneratorApi.listJobs(workspaceId);
      setJobs(Array.isArray(list) ? list : []);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load jobs');
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  // Initial load + poll
  useEffect(() => {
    if (!workspaceId) {
      setLoading(false);
      return;
    }
    fetchJobs();
    if (!hasPendingJob) return;
    const t = setInterval(fetchJobs, POLL_MS);
    return () => clearInterval(t);
  }, [workspaceId, fetchJobs, hasPendingJob]);

  // Phase 4: when a non-terminal job is expanded, open an SSE stream
  // for sub-second updates. The 5s list-polling still runs in parallel;
  // the SSE just makes the expanded card feel live. Closing the row or
  // unmounting tears down the EventSource.
  useEffect(() => {
    if (!workspaceId || !expandedJobId) return;
    const expandedJob = jobs.find((j) => j.id === expandedJobId);
    if (!expandedJob || TERMINAL_STATUSES.includes(expandedJob.status)) return;

    const close = subscribeToJobStream(workspaceId, expandedJobId, {
      onUpdate: (updated) => {
        setJobs((prev) => prev.map((j) => (j.id === updated.id ? updated : j)));
      },
      // Don't surface stream errors as page-level errors — the SSE is
      // a "best-effort live update" channel and the list-polling
      // fallback covers correctness. Silently let the browser retry.
      onError: () => {},
    });
    return close;
    // `jobs` intentionally not in the dep array — we only want to re-
    // subscribe when the expanded job changes, not on every poll tick.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspaceId, expandedJobId]);

  const handleCreate = async () => {
    if (!workspaceId) return;
    let parsedFilter: Record<string, unknown> | undefined;
    if (filterText.trim()) {
      try {
        parsedFilter = JSON.parse(filterText);
        if (typeof parsedFilter !== 'object' || parsedFilter === null || Array.isArray(parsedFilter)) {
          setCreateError('Filter must be a JSON object (e.g., {"status":">=400"})');
          return;
        }
      } catch (err) {
        setCreateError(`Invalid JSON: ${err instanceof Error ? err.message : err}`);
        return;
      }
    }
    setCreateError(null);
    setCreating(true);
    try {
      await cloudTestGeneratorApi.createJob(workspaceId, {
        prompt: prompt || undefined,
        captures_filter: parsedFilter,
      });
      setPrompt('');
      setFilterText('');
      await fetchJobs();
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : 'Failed to create job');
    } finally {
      setCreating(false);
    }
  };

  const handleCancel = async (jobId: string) => {
    if (!workspaceId) return;
    try {
      await cloudTestGeneratorApi.cancelJob(workspaceId, jobId);
      await fetchJobs();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to cancel job');
    }
  };

  // --- Render ------------------------------------------------------------

  if (!workspaceId) {
    return (
      <div className="content-width space-y-6">
        <PageHeader
          title="Test Generator"
          subtitle="AI-generated test scenarios from your capture corpus"
        />
        <Alert
          variant="info"
          title="Select a workspace"
          description="Pick an active workspace from the top nav to start generating tests."
        />
      </div>
    );
  }

  return (
    <div className="content-width space-y-6">
      <PageHeader
        title="Test Generator"
        subtitle="AI-generated test scenarios from your runtime_captures corpus"
        className="space-section"
      />

      <Alert
        variant="info"
        title="How this works"
        description="Jobs run against your org's BYOK provider (Settings → BYOK) — or platform credits on paid plans if no BYOK key is configured. Expanded rows live-stream via SSE; the rest of the list polls every 5 seconds."
      />

      {error && <Alert variant="error" title="Error" description={error} />}

      {/* Create form */}
      <Section>
        <Card className="p-6 space-y-4">
          <div>
            <h3 className="text-lg font-semibold text-foreground">New generation job</h3>
            <p className="text-sm text-muted-foreground">
              Describe what tests to generate. Leave both fields blank for a default
              prompt that scans all recent captures.
            </p>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                Prompt (optional, max 8 KB)
              </label>
              <Input
                type="text"
                placeholder='e.g., "focus on auth failure paths"'
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                className="w-full"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                Captures filter (JSON, max 16 KB)
              </label>
              <Input
                type="text"
                placeholder='e.g., {"status":">=400"}'
                value={filterText}
                onChange={(e) => setFilterText(e.target.value)}
                className="w-full font-mono text-sm"
              />
            </div>
          </div>
          {createError && (
            <p className="text-sm text-danger-600 dark:text-danger-400">{createError}</p>
          )}
          <div className="flex items-center justify-between">
            <p className="text-xs text-muted-foreground">
              Job will queue under workspace{' '}
              <span className="font-mono">{workspaceId.slice(0, 8)}</span>.
            </p>
            <Button onClick={handleCreate} disabled={creating}>
              <Play className="h-4 w-4 mr-2" />
              {creating ? 'Queueing…' : 'Queue job'}
            </Button>
          </div>
        </Card>
      </Section>

      {/* Job list */}
      <Section>
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-foreground">Recent jobs</h3>
          <Button variant="outline" size="sm" onClick={fetchJobs} disabled={loading}>
            <RefreshCw className={cn('h-4 w-4 mr-1', loading && 'animate-spin')} />
            Refresh
          </Button>
        </div>

        {loading && jobs.length === 0 ? (
          <Card className="p-12 text-center">
            <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600" />
            <p className="mt-4 text-muted-foreground">Loading jobs…</p>
          </Card>
        ) : jobs.length === 0 ? (
          <Card className="p-12 text-center">
            <p className="text-muted-foreground">
              No generation jobs yet. Queue one above.
            </p>
          </Card>
        ) : (
          <div className="space-y-2">
            {jobs.map((job) => {
              const expanded = expandedJobId === job.id;
              const canCancel = !TERMINAL_STATUSES.includes(job.status);
              return (
                <Card key={job.id} className="overflow-hidden">
                  <button
                    type="button"
                    className="w-full p-4 flex items-center justify-between hover:bg-muted/50 transition-colors text-left"
                    onClick={() => setExpandedJobId(expanded ? null : job.id)}
                  >
                    <div className="flex items-center gap-3">
                      {expanded ? (
                        <ChevronDown className="h-4 w-4 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-4 w-4 text-muted-foreground" />
                      )}
                      <div>
                        <div className="font-mono text-xs text-muted-foreground">
                          {job.id.slice(0, 8)}
                        </div>
                        <div className="text-sm font-medium text-foreground">
                          {job.prompt || <em className="text-muted-foreground">No prompt</em>}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      <Badge variant={statusBadgeVariant(job.status)}>{job.status}</Badge>
                      <span className="text-xs text-muted-foreground">
                        {formatRelative(job.queued_at)}
                      </span>
                    </div>
                  </button>
                  {expanded && (
                    <div className="px-4 pb-4 space-y-3 border-t border-border">
                      <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-sm">
                        <div>
                          <div className="text-xs text-muted-foreground uppercase tracking-wide">
                            Queued
                          </div>
                          <div>{formatRelative(job.queued_at)}</div>
                        </div>
                        <div>
                          <div className="text-xs text-muted-foreground uppercase tracking-wide">
                            Started
                          </div>
                          <div>{formatRelative(job.started_at)}</div>
                        </div>
                        <div>
                          <div className="text-xs text-muted-foreground uppercase tracking-wide">
                            Finished
                          </div>
                          <div>{formatRelative(job.finished_at)}</div>
                        </div>
                      </div>
                      <div>
                        <div className="text-xs text-muted-foreground uppercase tracking-wide mb-1">
                          Captures filter
                        </div>
                        <pre className="text-xs font-mono bg-muted/50 p-2 rounded overflow-x-auto">
                          {JSON.stringify(job.captures_filter, null, 2)}
                        </pre>
                      </div>
                      {job.error && (
                        <Alert variant="error" title="Job error" description={job.error} />
                      )}
                      {job.result !== null && job.result !== undefined && (
                        <div>
                          <div className="text-xs text-muted-foreground uppercase tracking-wide mb-1">
                            Result
                          </div>
                          <pre className="text-xs font-mono bg-muted/50 p-2 rounded overflow-x-auto max-h-96">
                            {JSON.stringify(job.result, null, 2)}
                          </pre>
                        </div>
                      )}
                      {canCancel && (
                        <div className="pt-2">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleCancel(job.id)}
                          >
                            <X className="h-4 w-4 mr-1" />
                            Cancel
                          </Button>
                        </div>
                      )}
                    </div>
                  )}
                </Card>
              );
            })}
          </div>
        )}
      </Section>
    </div>
  );
};
