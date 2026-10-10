import type { RunStreamEvent } from './runStreamEvents';

/** Bound queued work as well as rendered history; deliver at most every 100ms. */
export function createRunEventBuffer(limit: number, deliver: (batch: RunStreamEvent[]) => void) {
  let pending: RunStreamEvent[] = [];
  let timer: ReturnType<typeof setTimeout> | undefined;
  function flush() {
    clearTimeout(timer);
    timer = undefined;
    if (pending.length === 0) return;
    const batch = pending;
    pending = [];
    deliver(batch);
  }
  return {
    flush,
    push(event: RunStreamEvent) {
      pending.push(event);
      if (pending.length > limit) pending.splice(0, pending.length - limit);
      if (timer === undefined) timer = setTimeout(flush, 100);
    },
    cancel() { clearTimeout(timer); timer = undefined; pending = []; },
  };
}
