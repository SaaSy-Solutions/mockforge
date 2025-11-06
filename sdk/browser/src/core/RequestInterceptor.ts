/**
 * Request Interceptor
 * 
 * Intercepts fetch and XMLHttpRequest calls to capture requests/responses
 */

import { CapturedRequest } from '../types';
import { analyzeRequest, shouldCreateMock } from '../utils/requestAnalyzer';

/**
 * Callback type for captured requests
 */
export type RequestCaptureCallback = (request: CapturedRequest) => void | Promise<void>;

/**
 * Request interceptor that captures fetch and XHR requests
 */
export class RequestInterceptor {
    private originalFetch: typeof fetch;
    private originalXHROpen: typeof XMLHttpRequest.prototype.open;
    private originalXHRSend: typeof XMLHttpRequest.prototype.send;
    private captureCallback?: RequestCaptureCallback;
    private enabled: boolean = false;
    private autoMockStatusCodes: number[] = [404, 500, 502, 503, 504];
    private autoMockNetworkErrors: boolean = true;

    constructor() {
        this.originalFetch = window.fetch.bind(window);
        this.originalXHROpen = XMLHttpRequest.prototype.open;
        this.originalXHRSend = XMLHttpRequest.prototype.send;
    }

    /**
     * Start intercepting requests
     */
    start(callback: RequestCaptureCallback): void {
        if (this.enabled) {
            return;
        }

        this.captureCallback = callback;
        this.enabled = true;

        // Intercept fetch
        this.interceptFetch();

        // Intercept XMLHttpRequest
        this.interceptXHR();
    }

    /**
     * Stop intercepting requests
     */
    stop(): void {
        if (!this.enabled) {
            return;
        }

        this.enabled = false;
        this.captureCallback = undefined;

        // Restore original fetch
        window.fetch = this.originalFetch;

        // Restore original XHR
        XMLHttpRequest.prototype.open = this.originalXHROpen;
        XMLHttpRequest.prototype.send = this.originalXHRSend;
    }

    /**
     * Configure auto-mock behavior
     */
    configureAutoMock(options: {
        statusCodes?: number[];
        networkErrors?: boolean;
    }): void {
        if (options.statusCodes) {
            this.autoMockStatusCodes = options.statusCodes;
        }
        if (options.networkErrors !== undefined) {
            this.autoMockNetworkErrors = options.networkErrors;
        }
    }

    /**
     * Intercept fetch API
     */
    private interceptFetch(): void {
        const self = this;
        
        window.fetch = async function(
            input: RequestInfo | URL,
            init?: RequestInit
        ): Promise<Response> {
            const url = typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
            const method = init?.method || (input instanceof Request ? input.method : 'GET');
            const headers = init?.headers || (input instanceof Request ? input.headers : new Headers());
            const body = init?.body || (input instanceof Request ? input.body : undefined);

            let response: Response;
            let error: CapturedRequest['error'] | undefined;

            try {
                response = await self.originalFetch(input, init);
            } catch (err) {
                // Network error occurred
                error = {
                    type: err instanceof TypeError && err.message.includes('Failed to fetch') ? 'network' : 'network',
                    message: err instanceof Error ? err.message : 'Unknown network error',
                };
                
                // Create a mock response for the error
                response = new Response(null, {
                    status: 502,
                    statusText: 'Bad Gateway',
                    headers: { 'Content-Type': 'application/json' },
                });
            }

            // Capture the request
            if (self.captureCallback) {
                try {
                    const captured = await analyzeRequest(
                        method,
                        url,
                        headers,
                        body,
                        error ? undefined : response
                    );

                    if (error) {
                        captured.error = error;
                    }

                    // Check if we should trigger callback
                    const shouldTrigger = shouldCreateMock(
                        captured,
                        self.autoMockStatusCodes,
                        self.autoMockNetworkErrors
                    ) || !error; // Always capture successful requests for manual mock creation

                    if (shouldTrigger) {
                        await self.captureCallback(captured);
                    }
                } catch (err) {
                    // Silently fail capture to not break the application
                    console.warn('[ForgeConnect] Failed to capture request:', err);
                }
            }

            // If there was an error, throw it after capturing
            if (error) {
                throw new Error(error.message);
            }

            return response;
        };
    }

    /**
     * Intercept XMLHttpRequest
     */
    private interceptXHR(): void {
        const self = this;
        const xhrInstances = new WeakMap<XMLHttpRequest, {
            method: string;
            url: string;
            headers: Record<string, string>;
            body?: any;
        }>();

        // Intercept open()
        XMLHttpRequest.prototype.open = function(
            method: string,
            url: string | URL,
            async?: boolean,
            username?: string | null,
            password?: string | null
        ): void {
            const urlString = typeof url === 'string' ? url : url.toString();
            
            xhrInstances.set(this, {
                method,
                url: urlString,
                headers: {},
            });

            return self.originalXHROpen.call(this, method, url, async, username, password);
        };

        // Intercept send()
        XMLHttpRequest.prototype.send = function(body?: Document | XMLHttpRequestBodyInit | null): void {
            const instance = xhrInstances.get(this);
            if (!instance) {
                return self.originalXHRSend.call(this, body);
            }

            // Capture request body
            instance.body = body as any;

            // Capture headers
            const headers: Record<string, string> = {};
            if (this.getAllResponseHeaders) {
                const headerString = this.getAllResponseHeaders();
                if (headerString) {
                    headerString.split('\r\n').forEach(line => {
                        const [key, value] = line.split(': ');
                        if (key && value) {
                            headers[key] = value;
                        }
                    });
                }
            }

            // Set up response capture
            const originalOnReadyStateChange = this.onreadystatechange;
            
            this.onreadystatechange = function(event: Event): void {
                // Call original handler first
                if (originalOnReadyStateChange) {
                    originalOnReadyStateChange.call(this, event);
                }

                // Capture when request completes
                if (this.readyState === XMLHttpRequest.DONE && self.captureCallback) {
                    try {
                        const captured: CapturedRequest = {
                            method: instance.method,
                            url: instance.url,
                            path: new URL(instance.url, window.location.origin).pathname,
                            headers: Object.keys(headers).length > 0 ? headers : undefined,
                            body: instance.body,
                            statusCode: this.status,
                            timestamp: Date.now(),
                        };

                        // Try to parse response
                        try {
                            const responseText = this.responseText;
                            if (responseText) {
                                const contentType = this.getResponseHeader('Content-Type') || '';
                                if (contentType.includes('application/json')) {
                                    captured.responseBody = JSON.parse(responseText);
                                } else {
                                    captured.responseBody = responseText;
                                }
                            }
                        } catch {
                            // Ignore parsing errors
                        }

                        // Capture response headers
                        const responseHeaders: Record<string, string> = {};
                        if (this.getAllResponseHeaders) {
                            const headerString = this.getAllResponseHeaders();
                            if (headerString) {
                                headerString.split('\r\n').forEach(line => {
                                    const [key, value] = line.split(': ');
                                    if (key && value) {
                                        responseHeaders[key] = value;
                                    }
                                });
                            }
                        }
                        if (Object.keys(responseHeaders).length > 0) {
                            captured.responseHeaders = responseHeaders;
                        }

                        // Check for errors
                        if (this.status === 0 || this.status >= 400) {
                            captured.error = {
                                type: this.status === 0 ? 'network' : 'http',
                                message: this.statusText || `HTTP ${this.status}`,
                            };
                        }

                        // Check if we should trigger callback
                        const shouldTrigger = shouldCreateMock(
                            captured,
                            self.autoMockStatusCodes,
                            self.autoMockNetworkErrors
                        ) || this.status < 400; // Always capture successful requests

                        if (shouldTrigger) {
                            self.captureCallback!(captured).catch(err => {
                                console.warn('[ForgeConnect] Error in capture callback:', err);
                            });
                        }
                    } catch (err) {
                        console.warn('[ForgeConnect] Failed to capture XHR request:', err);
                    }
                }
            };

            return self.originalXHRSend.call(this, body);
        };
    }

    /**
     * Check if interceptor is enabled
     */
    isEnabled(): boolean {
        return this.enabled;
    }
}

