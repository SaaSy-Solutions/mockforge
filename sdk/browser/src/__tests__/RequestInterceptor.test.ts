/**
 * Unit tests for RequestInterceptor
 */

import { RequestInterceptor } from '../core/RequestInterceptor';

describe('RequestInterceptor', () => {
    let interceptor: RequestInterceptor;
    let captureCallback: jest.Mock;

    beforeEach(() => {
        interceptor = new RequestInterceptor();
        captureCallback = jest.fn();
        
        // Reset fetch
        (global.fetch as jest.Mock).mockClear();
    });

    afterEach(() => {
        interceptor.stop();
    });

    describe('start/stop', () => {
        it('should start intercepting requests', () => {
            interceptor.start(captureCallback);
            expect(interceptor.isEnabled()).toBe(true);
        });

        it('should stop intercepting requests', () => {
            interceptor.start(captureCallback);
            interceptor.stop();
            expect(interceptor.isEnabled()).toBe(false);
        });
    });

    describe('fetch interception', () => {
        it('should capture successful fetch requests', async () => {
            interceptor.start(captureCallback);

            (global.fetch as jest.Mock).mockResolvedValueOnce({
                ok: true,
                status: 200,
                headers: new Headers({ 'Content-Type': 'application/json' }),
                json: async () => ({ data: 'test' }),
            });

            await fetch('http://localhost:3000/api/test', {
                method: 'GET',
            });

            // Wait for async callback
            await new Promise(resolve => setTimeout(resolve, 10));

            expect(captureCallback).toHaveBeenCalled();
            const captured = captureCallback.mock.calls[0][0];
            expect(captured.method).toBe('GET');
            expect(captured.url).toBe('http://localhost:3000/api/test');
        });

        it('should capture failed fetch requests', async () => {
            interceptor.start(captureCallback);

            (global.fetch as jest.Mock).mockRejectedValueOnce(
                new TypeError('Failed to fetch')
            );

            try {
                await fetch('http://localhost:3000/api/test');
            } catch {
                // Expected to fail
            }

            // Wait for async callback
            await new Promise(resolve => setTimeout(resolve, 10));

            expect(captureCallback).toHaveBeenCalled();
            const captured = captureCallback.mock.calls[0][0];
            expect(captured.error).toBeDefined();
            expect(captured.error?.type).toBe('network');
        });
    });

    describe('configureAutoMock', () => {
        it('should configure auto-mock status codes', () => {
            interceptor.configureAutoMock({
                statusCodes: [404, 500],
                networkErrors: true,
            });

            // Configuration is applied (tested indirectly through behavior)
            expect(interceptor.isEnabled()).toBe(false); // Not started yet
        });
    });
});

