import React, { useEffect, useRef, useState } from 'react';
import { cloudTestRunsApi } from '../services/api/cloudTestRuns';

/**
 * Inline live-tail for a queued/running test-run (#1021).
 *
 * Streams `/api/v1/test-runs/{id}/events` via the shared SSE endpoint
 * (works for every run kind: contract tests, chaos campaigns, flows,
 * clone training). Closes itself when the terminal `done` event arrives;
 * Failed streams close rather than reconnect indefinitely. Users can retry
 * explicitly; retained events and pending render work are bounded.
 */

interface StreamEvent {
    type: string;
    text: string;
    received_at: string;
}

const KNOWN_EVENT_TYPES = [
    'log',
    'step_start',
    'step_pass',
    'step_fail',
    'metric',
    'fault_injected',
    'fault_recovered',
    'node_visited',
    'training_epoch',
    'experiment_start',
    'experiment_result',
    'diff_finding',
    'request_replayed',
    'component_dumped',
    'component_restored',
    'ping',
    'done',
    'stream_error',
];

export interface RunLiveTailProps {
    /** The test_run id whose event stream should be tailed. */
    runId: string;
    /** Whether the run is still queued/running (non-inflight runs skip streaming). */
    inflight?: boolean;
    /** Called once when the terminal `done` event arrives. */
    onDone?: (summary: unknown) => void;
    /** Max rendered lines kept in state (default 500). */
    maxLines?: number;
}

export const RunLiveTail: React.FC<RunLiveTailProps> = ({
    runId,
    inflight = true,
    onDone,
    maxLines = 500,
}) => {
    const [events, setEvents] = useState<StreamEvent[]>([]);
    const [streaming, setStreaming] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [retry, setRetry] = useState(0);
    const onDoneRef = useRef(onDone);
    onDoneRef.current = onDone;
    const lineLimit = Number.isFinite(maxLines) ? Math.min(500, Math.max(1, Math.floor(maxLines))) : 500;

    useEffect(() => {
        setEvents([]);
        setError(null);
        setStreaming(false);
        if (!inflight || !runId) return;

        const es = cloudTestRunsApi.streamRunEvents(runId);
        let closed = false;
        let pending: StreamEvent[] = [];
        let timer: ReturnType<typeof setTimeout> | undefined;
        const flush = () => {
            timer = undefined;
            if (pending.length === 0) return;
            const batch = pending;
            pending = [];
            setEvents((prev) => [...prev, ...batch].slice(-lineLimit));
        };
        const close = () => {
            closed = true;
            es.close();
            setStreaming(false);
            clearTimeout(timer);
            flush();
        };
        es.onopen = () => { if (!closed) setStreaming(true); };

        const onMessage = (ev: MessageEvent) => {
            if (closed || ev.type === 'ping') return;
            if (ev.data.length > 65536) {
                close();
                setError('Live stream stopped because an event was too large.');
                return;
            }
            try {
                const data = JSON.parse(ev.data);
                pending.push({ type: ev.type || 'message', text: JSON.stringify(data).slice(0, 2000), received_at: new Date().toISOString() });
                if (pending.length > lineLimit) pending.splice(0, pending.length - lineLimit);
                if (timer === undefined) timer = setTimeout(flush, 100);
                if (ev.type === 'done') {
                    close();
                    onDoneRef.current?.(data);
                } else if (ev.type === 'stream_error') {
                    close();
                    setError('The live stream failed. Retry to reconnect.');
                }
            } catch {
                if (ev.type === 'done' || ev.type === 'stream_error') {
                    close();
                    setError('The live stream ended with an invalid response.');
                }
            }
        };

        for (const t of KNOWN_EVENT_TYPES) {
            es.addEventListener(t, onMessage);
        }
        es.addEventListener('message', onMessage);
        es.onerror = () => {
            if (closed) return;
            close();
            setError('Live connection interrupted. Retry to reconnect.');
        };

        return () => {
            closed = true;
            clearTimeout(timer);
            pending = [];
            es.close();
        };
    }, [runId, inflight, lineLimit, retry]);

    if (!inflight && events.length === 0) return null;

    return (
        <div>
            <div className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Live events
                {streaming && (
                    <span className="text-blue-600 dark:text-blue-400 inline-flex items-center gap-1 text-xs">
                        <span className="relative flex h-2 w-2">
                            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75" />
                            <span className="relative inline-flex rounded-full h-2 w-2 bg-blue-500" />
                        </span>
                        live
                    </span>
                )}
            </div>
            {error && (
                <div role="alert" className="mb-2 flex items-center gap-2 text-sm text-red-700 dark:text-red-400">
                    <span>{error}</span>
                    <button type="button" className="underline" onClick={() => setRetry((value) => value + 1)}>
                        Retry live stream
                    </button>
                </div>
            )}
            <div className="bg-black/90 text-green-300 dark:text-green-300 rounded p-3 font-mono text-xs max-h-72 overflow-y-auto">
                {events.length === 0 ? (
                    <div className="text-gray-500 italic">Waiting for events…</div>
                ) : (
                    events
                        .filter((e) => e.type !== 'ping')
                        .map((e, i) => (
                            <div key={i} className="whitespace-pre-wrap break-all">
                                <span className="text-gray-500">
                                    [{new Date(e.received_at).toLocaleTimeString()}]
                                </span>{' '}
                                <span className="text-cyan-400">{e.type}</span> {e.text}
                            </div>
                        ))
                )}
            </div>
        </div>
    );
};

export default RunLiveTail;
