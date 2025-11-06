/**
 * Injected Script
 *
 * Runs in page context to intercept requests
 */

(function() {
    'use strict';

    // Only run if not already injected
    if (window.__forgeconnect_injected) {
        return;
    }
    window.__forgeconnect_injected = true;

    // Intercept fetch
    const originalFetch = window.fetch;
    window.fetch = async function(...args) {
        const url = typeof args[0] === 'string' ? args[0] : args[0] instanceof URL ? args[0].toString() : args[0].url;
        const method = args[1]?.method || 'GET';
        const headers = args[1]?.headers || {};
        const body = args[1]?.body;

        let response;
        let error;

        try {
            response = await originalFetch.apply(this, args);
        } catch (err) {
            error = {
                type: 'network',
                message: err.message || 'Network error',
            };
            // Create error response
            response = new Response(null, {
                status: 502,
                statusText: 'Bad Gateway',
            });
        }

        // Capture request
        const captured = {
            method: method.toUpperCase(),
            url: url,
            path: new URL(url, window.location.origin).pathname,
            headers: headers instanceof Headers ? Object.fromEntries(headers) : headers,
            body: body,
            statusCode: response.status,
            timestamp: Date.now(),
        };

        if (error) {
            captured.error = error;
        }

        // Try to parse response body
        if (response && !error) {
            try {
                const clone = response.clone();
                const contentType = clone.headers.get('content-type') || '';
                if (contentType.includes('application/json')) {
                    captured.responseBody = await clone.json();
                }
                captured.responseHeaders = Object.fromEntries(clone.headers);
            } catch {
                // Ignore parsing errors
            }
        }

        // Send to content script
        window.postMessage({
            type: 'FORGECONNECT_REQUEST',
            payload: captured,
        }, '*');

        if (error) {
            throw new Error(error.message);
        }

        return response;
    };

    // Intercept XMLHttpRequest
    const originalXHROpen = XMLHttpRequest.prototype.open;
    const originalXHRSend = XMLHttpRequest.prototype.send;
    const xhrInstances = new WeakMap();

    XMLHttpRequest.prototype.open = function(method, url, ...rest) {
        xhrInstances.set(this, { method, url: url.toString() });
        return originalXHROpen.apply(this, [method, url, ...rest]);
    };

    XMLHttpRequest.prototype.send = function(body) {
        const instance = xhrInstances.get(this);
        if (instance) {
            instance.body = body;

            const originalOnReadyStateChange = this.onreadystatechange;
            this.onreadystatechange = function() {
                if (originalOnReadyStateChange) {
                    originalOnReadyStateChange.apply(this, arguments);
                }

                if (this.readyState === XMLHttpRequest.DONE) {
                    const captured = {
                        method: instance.method.toUpperCase(),
                        url: instance.url,
                        path: new URL(instance.url, window.location.origin).pathname,
                        body: instance.body,
                        statusCode: this.status,
                        timestamp: Date.now(),
                    };

                    if (this.status === 0 || this.status >= 400) {
                        captured.error = {
                            type: this.status === 0 ? 'network' : 'http',
                            message: this.statusText || `HTTP ${this.status}`,
                        };
                    }

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

                    window.postMessage({
                        type: 'FORGECONNECT_REQUEST',
                        payload: captured,
                    }, '*');
                }
            };
        }

        return originalXHRSend.apply(this, [body]);
    };
})();
