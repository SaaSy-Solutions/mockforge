import React from 'react';
import { useRunLiveTail, type RunLiveTailOptions } from '../hooks/useRunLiveTail';

/** Inline bounded live events for queued/running runs, with explicit retry. */
export type RunLiveTailProps = RunLiveTailOptions;
export const RunLiveTail: React.FC<RunLiveTailProps> = props => {
    const { events, streaming, error, retry } = useRunLiveTail(props);
    const inflight = props.inflight ?? true;

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
                    <button type="button" className="underline" onClick={retry}>
                        Retry live stream
                    </button>
                </div>
            )}
            <div className="bg-black/90 text-green-300 dark:text-green-300 rounded p-3 font-mono text-xs max-h-72 overflow-y-auto">
                {events.length === 0 ? (
                    <div className="text-gray-500 italic">Waiting for events…</div>
                ) : (
                    events.map((e, i) => (
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
