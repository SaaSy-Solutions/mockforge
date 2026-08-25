import { describe, expect, it, vi, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { RunLiveTail } from '../RunLiveTail';

// Minimal EventSource stub: captures listeners so tests can dispatch
// named events exactly like the browser would.
class MockEventSource {
    static instances: MockEventSource[] = [];
    url: string;
    readyState = 1;
    onerror: (() => void) | null = null;
    private listeners = new Map<string, ((ev: { data: string }) => void)[]>();

    constructor(url: string) {
        this.url = url;
        MockEventSource.instances.push(this);
    }

    addEventListener(type: string, cb: (ev: { data: string }) => void) {
        const list = this.listeners.get(type) ?? [];
        list.push(cb);
        this.listeners.set(type, list);
    }

    emit(type: string, data: unknown) {
        // Real EventSource sets `type` to the event name on named events.
        for (const cb of this.listeners.get(type) ?? []) {
            cb({ type, data: JSON.stringify(data) });
        }
        const message = this.listeners.get('message') ?? [];
        void message; // 'message' listeners only fire for unnamed events
    }

    close() {
        this.readyState = 3;
    }
}

describe('RunLiveTail', () => {
    afterEach(() => {
        MockEventSource.instances = [];
        vi.restoreAllMocks();
    });

    it('renders nothing for a non-inflight run with no events', () => {
        const { container } = render(<RunLiveTail runId="r1" inflight={false} />);
        expect(container).toBeEmptyDOMElement();
    });

    it('opens the stream for an in-flight run and shows the live badge', async () => {
        vi.stubGlobal('EventSource', MockEventSource);
        render(<RunLiveTail runId="run-123" inflight />);
        await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
        expect(MockEventSource.instances[0].url).toContain('/api/v1/test-runs/run-123/stream');
        expect(screen.getByText('live')).toBeInTheDocument();
    });

    it('renders emitted events and closes on done', async () => {
        vi.stubGlobal('EventSource', MockEventSource);
        const onDone = vi.fn();
        render(<RunLiveTail runId="run-abc" inflight onDone={onDone} />);
        const es = MockEventSource.instances.at(-1)!;

        es.emit('node_visited', { node_name: 'checkout', duration_ms: 12 });
        es.emit('done', { status: 'passed' });

        await waitFor(() => expect(onDone).toHaveBeenCalled());
        // Called once per mounted stream (StrictMode double-invokes effects).
        for (const [arg] of onDone.mock.calls) {
            expect(arg).toEqual({ status: 'passed' });
        }
        expect(screen.getByText(/node_visited/)).toBeInTheDocument();
        expect(screen.queryByText('Waiting for events…')).not.toBeInTheDocument();
    });

    it('filters ping keep-alives out of the rendered timeline', async () => {
        vi.stubGlobal('EventSource', MockEventSource);
        render(<RunLiveTail runId="run-ping" inflight />);
        const es = MockEventSource.instances.at(-1)!;
        es.emit('ping', {});
        await waitFor(() =>
            expect(screen.queryByText(/Waiting for events/)).toBeInTheDocument(),
        );
    });
});
