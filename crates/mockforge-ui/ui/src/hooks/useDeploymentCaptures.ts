/**
 * Pulls the in-deployment recorder capture list through the registry
 * server's cloud proxy (#234). The deployed mockforge-cli stores captures
 * in a local SQLite; we proxy `/api/recorder/requests` to make them
 * readable from the cloud admin UI without leaking the deployment URL to
 * the browser.
 *
 * Polling (default 5s) is fine here — captures are user-driven, so volume
 * is bounded by what the user is hitting the deployment with. This isn't
 * a high-throughput stream; we don't need SSE.
 */

import { useEffect, useState } from 'react';
import { logger } from '@/utils/logger';

/// Mirrors `mockforge_recorder::models::RecordedRequest`. Headers and
/// query_params are JSON-encoded strings on the wire — kept as `string`
/// here and parsed lazily when the detail view opens.
export interface DeploymentCapture {
  id: string;
  protocol: string;
  timestamp: string;
  method: string;
  path: string;
  query_params: string | null;
  headers: string;
  body: string | null;
  body_encoding: string;
  client_ip: string | null;
  trace_id: string | null;
  span_id: string | null;
  duration_ms: number | null;
  status_code: number | null;
  tags: string | null;
}

export interface DeploymentCaptureResponse {
  status_code: number;
  headers: string;
  body: string | null;
  body_encoding: string;
}

export interface UseDeploymentCapturesOptions {
  enabled?: boolean;
  intervalMs?: number;
  limit?: number;
}

export interface UseDeploymentCapturesReturn {
  captures: DeploymentCapture[];
  loading: boolean;
  error: string | null;
  refetch: () => void;
}

export function useDeploymentCaptures(
  deploymentId: string | undefined,
  options: UseDeploymentCapturesOptions = {},
): UseDeploymentCapturesReturn {
  const { enabled = true, intervalMs = 5000, limit = 100 } = options;

  const [captures, setCaptures] = useState<DeploymentCapture[]>([]);
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

      const url = `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/captures?limit=${limit}`;

      setLoading(true);
      try {
        const resp = await fetch(url, {
          headers: { Authorization: `Bearer ${token}` },
        });
        if (!resp.ok) {
          // The recorder API returns 503 when the recorder isn't enabled
          // on the deployment. Surface that as a friendly empty + hint.
          if (resp.status === 503 || resp.status === 404) {
            if (!cancelled) {
              setCaptures([]);
              setError(null);
            }
            return;
          }
          throw new Error(`HTTP ${resp.status}`);
        }
        const data = await resp.json();
        if (cancelled) return;
        // Recorder API may return either a bare array or { requests: [...] }
        // depending on version. Handle both.
        const list: DeploymentCapture[] = Array.isArray(data)
          ? data
          : Array.isArray(data?.requests)
            ? data.requests
            : [];
        setCaptures(list);
        setError(null);
      } catch (err) {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : 'Failed to fetch captures';
        setError(msg);
        logger.warn('Deployment captures poll failed', err);
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
    captures,
    loading,
    error,
    refetch: () => setRefetchTick((t) => t + 1),
  };
}

/// Fetch the response body for a single capture on demand. Used by the
/// capture detail drawer — separate from the list endpoint so the list
/// doesn't drag potentially-large response bodies along.
export async function fetchCaptureResponse(
  deploymentId: string,
  captureId: string,
): Promise<DeploymentCaptureResponse | null> {
  const token = localStorage.getItem('auth_token');
  if (!token) return null;
  const url = `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/captures/${encodeURIComponent(captureId)}/response`;
  const resp = await fetch(url, {
    headers: { Authorization: `Bearer ${token}` },
  });
  if (!resp.ok) return null;
  return resp.json();
}
