/**
 * Pulls trace summaries and individual trace detail from the registry's
 * OTLP storage (#233). The deployment's mockforge-cli pushes spans through
 * the OTLP/HTTP-JSON exporter; the registry persists them in Postgres.
 *
 * Polling (default 5s) is fine — like captures, the data volume is bounded
 * by user traffic and the server-side aggregate query is cheap (indexed by
 * deployment_id + occurred_at).
 */

import { useEffect, useState } from 'react';
import { logger } from '@/utils/logger';

/// Mirrors `handlers::otlp::TraceSummary` in mockforge-registry-server.
export interface TraceSummary {
  trace_id: string;
  span_count: number;
  start: string;
  duration_ms: number;
  root_name: string;
  service_name: string | null;
  has_error: boolean;
}

/// Mirrors `handlers::otlp::SpanResponse`. attributes / events / links are
/// JSONB on the wire — surface as `unknown` and let consumers narrow.
export interface TraceSpan {
  trace_id: string;
  span_id: string;
  parent_span_id: string | null;
  service_name: string | null;
  name: string;
  kind: number | null;
  start_unix_nano: number;
  end_unix_nano: number;
  status_code: number | null;
  status_message: string | null;
  attributes: Record<string, unknown>;
  events: unknown[];
  links: unknown[];
}

export interface UseDeploymentTracesOptions {
  enabled?: boolean;
  intervalMs?: number;
  limit?: number;
}

export interface UseDeploymentTracesReturn {
  traces: TraceSummary[];
  loading: boolean;
  error: string | null;
  refetch: () => void;
}

export function useDeploymentTraces(
  deploymentId: string | undefined,
  options: UseDeploymentTracesOptions = {},
): UseDeploymentTracesReturn {
  const { enabled = true, intervalMs = 5000, limit = 100 } = options;

  const [traces, setTraces] = useState<TraceSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refetchTick, setRefetchTick] = useState(0);

  useEffect(() => {
    if (!enabled || !deploymentId) {
      return;
    }

    let cancelled = false;

    const poll = async () => {
      const token = localStorage.getItem('auth_token');
      if (!token) return;

      const url = `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/traces?limit=${limit}`;

      setLoading(true);
      try {
        const resp = await fetch(url, {
          headers: { Authorization: `Bearer ${token}` },
        });
        if (!resp.ok) {
          // 404 happens before any spans have been ingested — surface as
          // empty rather than an error so the UI doesn't yell at users.
          if (resp.status === 404) {
            if (!cancelled) {
              setTraces([]);
              setError(null);
            }
            return;
          }
          throw new Error(`HTTP ${resp.status}`);
        }
        const data = await resp.json();
        if (cancelled) return;
        const list: TraceSummary[] = Array.isArray(data) ? data : [];
        setTraces(list);
        setError(null);
      } catch (err) {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : 'Failed to fetch traces';
        setError(msg);
        logger.warn('Deployment traces poll failed', err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    void poll();
    const handle = setInterval(poll, intervalMs);
    return () => {
      cancelled = true;
      clearInterval(handle);
    };
  }, [enabled, deploymentId, intervalMs, limit, refetchTick]);

  return {
    traces,
    loading,
    error,
    refetch: () => setRefetchTick((t) => t + 1),
  };
}

/// Fetch every span of a single trace on demand. Separate from the list
/// endpoint so the list view stays cheap.
export async function fetchTraceSpans(
  deploymentId: string,
  traceId: string,
): Promise<TraceSpan[]> {
  const token = localStorage.getItem('auth_token');
  if (!token) return [];
  const url = `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/traces/${encodeURIComponent(traceId)}`;
  const resp = await fetch(url, {
    headers: { Authorization: `Bearer ${token}` },
  });
  if (!resp.ok) return [];
  const data = await resp.json();
  return Array.isArray(data) ? (data as TraceSpan[]) : [];
}
