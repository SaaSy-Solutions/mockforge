/**
 * Polls the registry server's `/runtime-requests` endpoint to surface the
 * log shipper's captured pairs in the admin UI's Requests tab (#232).
 *
 * The shipper inside each hosted-mock container batches every request the
 * deployed mockforge-cli serves and POSTs to MockForge Cloud. The cloud
 * stores them in `runtime_request_logs` and exposes them at
 * `GET /api/v1/hosted-mocks/{id}/runtime-requests`. This hook polls that
 * endpoint with a `?since=` cursor so each tick only fetches new rows.
 *
 * Polling (every 4 seconds by default) is fine here — the volume per
 * deployment is bounded by how much traffic the user is sending, and the
 * cloud-side endpoint reads from a (deployment_id, occurred_at)-indexed
 * table. SSE was a real option too but polling is one less moving part
 * and matches how the metrics tab works.
 */

import { useEffect, useRef, useState } from 'react';
import { logger } from '@/utils/logger';

export interface RuntimeRequestRow {
  timestamp: string;
  method: string;
  path: string;
  status: number;
  latency_ms: number;
  matched_route?: string | null;
  client_ip?: string | null;
  user_agent?: string | null;
  request_id?: string | null;
  bytes_in?: number | null;
  bytes_out?: number | null;
}

export interface UseRuntimeRequestsOptions {
  /** Whether to poll. Defaults to true when deploymentId is set. */
  enabled?: boolean;
  /** Poll interval in milliseconds. Default 4000. */
  intervalMs?: number;
  /** Max entries to keep buffered. Default 500. */
  maxEntries?: number;
}

export interface UseRuntimeRequestsReturn {
  rows: RuntimeRequestRow[];
  /** True between request start and response. */
  loading: boolean;
  /** Last error message from the most recent poll, if any. */
  error: string | null;
  /** Manually trigger a refetch (e.g. for a refresh button). */
  refetch: () => void;
  /** Empty the local buffer without affecting server state. */
  clear: () => void;
}

export function useRuntimeRequests(
  deploymentId: string | undefined,
  options: UseRuntimeRequestsOptions = {},
): UseRuntimeRequestsReturn {
  const { enabled = true, intervalMs = 4000, maxEntries = 500 } = options;

  const [rows, setRows] = useState<RuntimeRequestRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Cursor advances past the most recent row we've seen so the server only
  // returns new entries on subsequent polls. `useRef` avoids re-renders
  // when the cursor changes.
  const cursorRef = useRef<string | null>(null);
  // Trigger that lets `refetch()` force a fresh poll without resetting the
  // interval.
  const [refetchTick, setRefetchTick] = useState(0);

  useEffect(() => {
    if (!enabled || !deploymentId) {
      return;
    }

    let cancelled = false;

    const poll = async () => {
      const token = localStorage.getItem('auth_token');
      if (!token) return;

      const params = new URLSearchParams();
      params.set('limit', '200');
      if (cursorRef.current) {
        params.set('since', cursorRef.current);
      }
      const url = `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/runtime-requests?${params}`;

      setLoading(true);
      try {
        const resp = await fetch(url, {
          headers: { Authorization: `Bearer ${token}` },
        });
        if (!resp.ok) {
          throw new Error(`HTTP ${resp.status}`);
        }
        const data: RuntimeRequestRow[] = await resp.json();
        if (cancelled) return;

        if (Array.isArray(data) && data.length > 0) {
          // Server returns DESC by occurred_at. Advance cursor to the
          // newest row we just received.
          cursorRef.current = data[0].timestamp;
          setRows((prev) => {
            const merged = [...data, ...prev];
            return merged.length > maxEntries ? merged.slice(0, maxEntries) : merged;
          });
        }
        setError(null);
      } catch (err) {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : 'Failed to fetch runtime requests';
        setError(msg);
        logger.warn('Runtime requests poll failed', err);
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
  }, [enabled, deploymentId, intervalMs, maxEntries, refetchTick]);

  return {
    rows,
    loading,
    error,
    refetch: () => setRefetchTick((t) => t + 1),
    clear: () => {
      cursorRef.current = null;
      setRows([]);
    },
  };
}
