/**
 * Subscribe to runtime logs streamed from the registry server's Fly logs proxy.
 *
 * The hosted mock detail modal previously displayed deployment lifecycle
 * events under a "Logs" tab. Those still exist (they live in the registry
 * server's `deployment_logs` table) but they're not what users mean by "logs."
 * This hook talks to the new SSE endpoint added in #224, which polls Fly's
 * logs API and forwards container stdout/stderr to the browser.
 */

import { useEffect, useRef, useState } from 'react';
import { logger } from '@/utils/logger';

export interface FlyRuntimeLogEntry {
  timestamp: string;
  level: string;
  message: string;
  instance?: string;
  region?: string;
}

export interface UseFlyRuntimeLogsOptions {
  /** Whether the stream should be active. Defaults to true when deploymentId is set. */
  enabled?: boolean;
  /** Maximum entries to keep buffered. Older entries are dropped FIFO. */
  maxEntries?: number;
}

export interface UseFlyRuntimeLogsReturn {
  entries: FlyRuntimeLogEntry[];
  /** SSE connection status. */
  connected: boolean;
  /** Last error reported by the server-side poll loop, if any. */
  error: string | null;
  /** True when the server signalled FLYIO_API_TOKEN is not configured. */
  notConfigured: boolean;
  /** Manually clear the buffer. */
  clear: () => void;
}

export function useFlyRuntimeLogs(
  deploymentId: string | undefined,
  options: UseFlyRuntimeLogsOptions = {},
): UseFlyRuntimeLogsReturn {
  const { enabled = true, maxEntries = 500 } = options;

  const [entries, setEntries] = useState<FlyRuntimeLogEntry[]>([]);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notConfigured, setNotConfigured] = useState(false);

  const sourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    if (!enabled || !deploymentId) {
      return;
    }

    // EventSource doesn't honour custom headers, so the auth_token rides as a
    // query string. The registry server already accepts both Authorization
    // header and ?token= for SSE endpoints (see middleware/auth.rs).
    const token = localStorage.getItem('auth_token') ?? '';
    const url =
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/runtime-logs/stream` +
      (token ? `?token=${encodeURIComponent(token)}` : '');

    const source = new EventSource(url);
    sourceRef.current = source;
    setError(null);
    setNotConfigured(false);

    source.onopen = () => {
      setConnected(true);
    };

    source.onerror = (ev) => {
      setConnected(false);
      logger.warn('Fly runtime logs SSE error', ev);
    };

    source.addEventListener('config', (ev) => {
      const messageEv = ev as MessageEvent;
      setNotConfigured(true);
      logger.info('Fly runtime logs not configured:', messageEv.data);
    });

    source.addEventListener('logs', (ev) => {
      const messageEv = ev as MessageEvent;
      try {
        const parsed = JSON.parse(messageEv.data) as FlyRuntimeLogEntry[];
        if (Array.isArray(parsed) && parsed.length > 0) {
          setEntries((prev) => {
            const next = [...parsed, ...prev];
            return next.length > maxEntries ? next.slice(0, maxEntries) : next;
          });
        }
      } catch (err) {
        logger.warn('Failed to parse Fly runtime logs payload', err);
      }
    });

    source.addEventListener('error', (ev) => {
      const messageEv = ev as MessageEvent;
      // Server-side error event with payload (different from network onerror).
      if (messageEv.data) {
        try {
          const payload = JSON.parse(messageEv.data) as { error?: string };
          if (payload.error) {
            setError(payload.error);
          }
        } catch {
          setError(String(messageEv.data));
        }
      }
    });

    return () => {
      source.close();
      sourceRef.current = null;
      setConnected(false);
    };
  }, [enabled, deploymentId, maxEntries]);

  return {
    entries,
    connected,
    error,
    notConfigured,
    clear: () => setEntries([]),
  };
}
