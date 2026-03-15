import type {
  ConformanceRun,
  ConformanceRunRequest,
  ConformanceRunSummary,
  ConformanceProgress,
} from '../types/conformance';
import { authenticatedFetch } from '../utils/apiClient';

const BASE_URL = '/api/conformance';

/** Start a new conformance test run */
export async function startConformanceRun(
  config: ConformanceRunRequest
): Promise<{ id: string }> {
  const res = await authenticatedFetch(`${BASE_URL}/run`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(config),
  });
  if (!res.ok) {
    throw new Error(`Failed to start conformance run: ${res.status} ${res.statusText}`);
  }
  return res.json();
}

/** Get a conformance run by ID */
export async function getConformanceRun(id: string): Promise<ConformanceRun> {
  const res = await authenticatedFetch(`${BASE_URL}/run/${id}`);
  if (!res.ok) {
    throw new Error(`Failed to get conformance run: ${res.status}`);
  }
  return res.json();
}

/** List all conformance runs */
export async function listConformanceRuns(): Promise<ConformanceRunSummary[]> {
  const res = await authenticatedFetch(`${BASE_URL}/runs`);
  if (!res.ok) {
    throw new Error(`Failed to list conformance runs: ${res.status}`);
  }
  return res.json();
}

/** Delete a conformance run */
export async function deleteConformanceRun(id: string): Promise<void> {
  const res = await authenticatedFetch(`${BASE_URL}/run/${id}`, { method: 'DELETE' });
  if (!res.ok) {
    throw new Error(`Failed to delete conformance run: ${res.status}`);
  }
}

/** Stream conformance progress via SSE
 *
 * Note: EventSource doesn't support custom headers, so we use fetch-based
 * polling as a fallback. For SSE with auth, we append the token as a query param.
 */
export function streamConformanceProgress(
  id: string,
  onEvent: (event: ConformanceProgress) => void,
  onError?: (error: Event) => void
): EventSource | null {
  const isCloud = !!import.meta.env.VITE_API_BASE_URL;
  if (isCloud) return null;

  // EventSource doesn't support Authorization headers natively.
  // The SSE endpoint is best-effort — we also poll via getConformanceRun.
  const eventSource = new EventSource(`${BASE_URL}/run/${id}/stream`);

  eventSource.addEventListener('conformance_progress', (e: MessageEvent) => {
    try {
      const data = JSON.parse(e.data) as ConformanceProgress;
      onEvent(data);
    } catch {
      // ignore parse errors
    }
  });

  if (onError) {
    eventSource.onerror = onError;
  }

  return eventSource;
}
