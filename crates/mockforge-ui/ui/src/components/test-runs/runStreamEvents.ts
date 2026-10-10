export interface RunStreamEvent { type: string; text: string; received_at: string }
export const RUN_EVENT_TYPES = ['log', 'step_start', 'step_pass', 'step_fail', 'metric', 'fault_injected', 'fault_recovered', 'node_visited', 'training_epoch', 'experiment_start', 'experiment_result', 'diff_finding', 'request_replayed', 'component_dumped', 'component_restored', 'ping', 'done', 'stream_error'];
type DecodedEvent = { kind: 'ignore' } | { kind: 'error'; message: string } | { kind: 'event'; event: RunStreamEvent; summary: unknown };

export function runStreamLineLimit(maxLines: number) {
  return Number.isFinite(maxLines) ? Math.min(500, Math.max(1, Math.floor(maxLines))) : 500;
}
export function decodeRunStreamEvent(event: MessageEvent): DecodedEvent {
  if (event.type === 'ping') return { kind: 'ignore' };
  if (event.data.length > 65536) return { kind: 'error', message: 'Live stream stopped because an event was too large.' };
  try {
    const data: unknown = JSON.parse(event.data);
    return { kind: 'event', summary: data, event: { type: event.type || 'message', text: JSON.stringify(data).slice(0, 2000), received_at: new Date().toISOString() } };
  } catch {
    if (event.type === 'done' || event.type === 'stream_error') return { kind: 'error', message: 'The live stream ended with an invalid response.' };
    return { kind: 'ignore' };
  }
}
