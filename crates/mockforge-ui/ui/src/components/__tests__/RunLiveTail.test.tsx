import { describe, expect, it, vi, afterEach } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { RunLiveTail } from '../RunLiveTail';

// Minimal EventSource stub: captures listeners so tests can dispatch
// named events exactly like the browser would.
class MockEventSource {
    static instances: MockEventSource[] = [];
    url: string;
    readyState = 1;
    onerror: (() => void) | null = null;
    onopen: (() => void) | null = null;
    private listeners = new Map<string, ((ev: { type: string; data: string }) => void)[]>();

    constructor(url: string) {
        this.url = url;
        MockEventSource.instances.push(this);
    }

    addEventListener(type: string, cb: (ev: { type: string; data: string }) => void) {
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
        vi.unstubAllGlobals();
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
        act(() => MockEventSource.instances[0].onopen?.());
        expect(screen.getByText('live')).toBeInTheDocument();
    });

    it('renders emitted events and closes on done', async () => {
        vi.stubGlobal('EventSource', MockEventSource);
        const onDone = vi.fn();
        render(<RunLiveTail runId="run-abc" inflight onDone={onDone} />);
        await waitFor(() => expect(MockEventSource.instances.length).toBeGreaterThan(0));
        const es = MockEventSource.instances.at(-1)!;

        // The handler calls onDone synchronously, so waiting on onDone alone
        // can pass before React re-renders the new events; flush inside act
        // and wait for the rendered text itself.
        act(() => {
            es.emit('node_visited', { node_name: 'checkout', duration_ms: 12 });
            es.emit('done', { status: 'passed' });
        });

        await waitFor(() => expect(onDone).toHaveBeenCalled());
        // Called once per mounted stream (StrictMode double-invokes effects).
        for (const [arg] of onDone.mock.calls) {
            expect(arg).toEqual({ status: 'passed' });
        }
        expect(await screen.findByText(/node_visited/)).toBeInTheDocument();
        expect(screen.queryByText('Waiting for events…')).not.toBeInTheDocument();
    });

    it('filters ping keep-alives out of the rendered timeline', async () => {
        vi.stubGlobal('EventSource', MockEventSource);
        render(<RunLiveTail runId="run-ping" inflight />);
        const es = MockEventSource.instances.at(-1)!;
        act(() => es.emit('ping', {}));
        await waitFor(() =>
            expect(screen.queryByText(/Waiting for events/)).toBeInTheDocument(),
        );
    });

    it('closes on a transport error and reconnects only after an explicit retry', async () => {
        vi.stubGlobal('EventSource', MockEventSource);
        render(<RunLiveTail runId="error-run" />);
        const source = MockEventSource.instances[0];
        act(() => source.onerror?.());
        expect(source.readyState).toBe(3);
        expect(screen.getByRole('alert')).toHaveTextContent('interrupted');
        expect(MockEventSource.instances).toHaveLength(1);
        fireEvent.click(screen.getByRole('button', { name: 'Retry live stream' }));
        expect(MockEventSource.instances).toHaveLength(2);
    });

    it('closes on stream_error and ignores events after closing', async () => {
        vi.stubGlobal('EventSource', MockEventSource);
        render(<RunLiveTail runId="server-error" />);
        const source = MockEventSource.instances[0];
        act(() => {
            source.emit('stream_error', { error: 'failed' });
            source.emit('log', { message: 'late-event' });
        });
        expect(source.readyState).toBe(3);
        expect(screen.getByRole('alert')).toHaveTextContent('failed');
        expect(screen.queryByText(/late-event/)).not.toBeInTheDocument();
    });

    it('bounds bursts even with maxLines=1 and discards heartbeat traffic', async () => {
        vi.stubGlobal('EventSource', MockEventSource);
        render(<RunLiveTail runId="burst" maxLines={1} />);
        const source = MockEventSource.instances[0];
        act(() => {
            for (let i = 0; i < 1000; i++) {
                source.emit('log', { message: `line-${i}` });
                source.emit('ping', {});
            }
            source.emit('done', { status: 'passed' });
        });
        expect(screen.getByText(/passed/)).toBeInTheDocument();
        expect(screen.queryByText(/line-/)).not.toBeInTheDocument();
        expect(source.readyState).toBe(3);
    });

    it('cancels pending renders and closes the old stream when switching runs', async () => {
        vi.stubGlobal('EventSource', MockEventSource);
        const view = render(<RunLiveTail runId="old" />);
        const old = MockEventSource.instances[0];
        act(() => old.emit('log', { message: 'old-message' }));
        view.rerender(<RunLiveTail runId="new" />);
        expect(old.readyState).toBe(3);
        expect(screen.queryByText(/old-message/)).not.toBeInTheDocument();
        view.unmount();
        expect(MockEventSource.instances[1].readyState).toBe(3);
    });

    it('stops oversized events before parsing or retaining them', async () => {
        vi.stubGlobal('EventSource', MockEventSource);
        render(<RunLiveTail runId="oversized" />);
        const source = MockEventSource.instances[0];
        act(() => source.emit('log', { message: 'x'.repeat(65536) }));
        expect(source.readyState).toBe(3);
        expect(screen.getByRole('alert')).toHaveTextContent('too large');
    });
});
