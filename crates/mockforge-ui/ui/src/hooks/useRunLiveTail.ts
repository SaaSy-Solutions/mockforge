import { useEffect, useRef, useState } from 'react';
import { cloudTestRunsApi } from '../services/api/cloudTestRuns';
import { createRunEventBuffer } from '../components/test-runs/runEventBuffer';
import { decodeRunStreamEvent, runStreamLineLimit, RUN_EVENT_TYPES, type RunStreamEvent } from '../components/test-runs/runStreamEvents';

export interface RunLiveTailOptions {
  runId: string;
  inflight?: boolean;
  onDone?: (summary: unknown) => void;
  maxLines?: number;
}
/** Own one stream, its bounded buffer, and cleanup independently of presentation. */
export function useRunLiveTail({ runId, inflight = true, onDone, maxLines = 500 }: RunLiveTailOptions) {
  const [events, setEvents] = useState<RunStreamEvent[]>([]);
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const onDoneRef = useRef(onDone);
  onDoneRef.current = onDone;
  const limit = runStreamLineLimit(maxLines);
  useEffect(() => { setEvents([]); setError(null); }, [runId, attempt]);
  useEffect(() => {
    setStreaming(false);
    setEvents(previous => previous.slice(-limit));
    if (!inflight || !runId) { setError(null); return; }
    const source = cloudTestRunsApi.streamRunEvents(runId);
    let closed = false;
    const buffer = createRunEventBuffer(limit, batch => setEvents(previous => [...previous, ...batch].slice(-limit)));
    function close() { closed = true; source.close(); setStreaming(false); buffer.flush(); }
    source.onopen = () => { if (!closed) setStreaming(true); };
    const receive = (event: MessageEvent) => {
      if (closed) return;
      const decoded = decodeRunStreamEvent(event);
      if (decoded.kind === 'ignore') return;
      if (decoded.kind === 'error') { close(); setError(decoded.message); return; }
      buffer.push(decoded.event);
      switch (decoded.event.type) {
        case 'done': close(); onDoneRef.current?.(decoded.summary); break;
        case 'stream_error': close(); setError('The live stream failed. Retry to reconnect.'); break;
      }
    };
    for (const type of RUN_EVENT_TYPES) source.addEventListener(type, receive);
    source.addEventListener('message', receive);
    source.onerror = () => { if (!closed) { close(); setError('Live connection interrupted. Retry to reconnect.'); } };
    return () => { closed = true; buffer.cancel(); source.close(); };
  }, [runId, inflight, limit, attempt]);
  return { events, streaming, error, retry: () => setAttempt(previous => previous + 1) };
}
