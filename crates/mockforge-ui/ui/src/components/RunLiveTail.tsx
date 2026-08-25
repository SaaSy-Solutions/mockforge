import React, { useEffect, useRef, useState } from 'react';
import { cloudTestRunsApi } from '../services/api/cloudTestRuns';

/**
 * Inline live-tail for a queued/running test-run (#1021).
 *
 * Streams `/api/v1/test-runs/{id}/events` via the shared SSE endpoint
 * (works for every run kind: contract tests, chaos campaigns, flows,
 * clone training). Closes itself when the terminal `done` event arrives;
 * reconnect handling is delegated to the browser's EventSource +
 * Last-Event-ID replay on the server.
 */

interface StreamEvent {
    type: string;
    data: unknown;
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
    const sourceRef = useRef<EventSource | null>(null);

    useEffect(() => {
        if (!inflight || !runId) return;

        const es = cloudTestRunsApi.streamRunEvents(runId);
        sourceRef.current = es;
        setStreaming(true);

        const onMessage = (ev: MessageEvent) => {
            try {
                const data = JSON.parse(ev.data);
                setEvents((prev) => [
                    ...prev.slice(-(maxLines - 1)),
                    { type: ev.type || 'message', data, received_at: new Date().toISOString() },
                ]);
                if (ev.type === 'done') {
                    setStreaming(false);
                    es.close();
                    onDone?.(data);
                }
            } catch {
                /* ignore non-JSON ping payloads */
            }
        };

        for (const t of KNOWN_EVENT_TYPES) {
            es.addEventListener(t, onMessage);
        }
        es.addEventListener('message', onMessage);
        es.onerror = () => setStreaming(false);

        return () => {
            es.close();
            sourceRef.current = null;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [runId, inflight]);

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
                                <span className="text-cyan-400">{e.type}</span> {JSON.stringify(e.data)}
                            </div>
                        ))
                )}
            </div>
        </div>
    );
};

export default RunLiveTail;
