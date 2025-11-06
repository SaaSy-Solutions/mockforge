/**
 * Response Generator
 *
 * Generates mock responses from captured requests or templates
 */

import { CapturedRequest, MockResponse } from '../types';

/**
 * Generate a default mock response when no response is available
 */
export function generateDefaultResponse(request: CapturedRequest): any {
    const method = request.method.toUpperCase();

    // Generate response based on method
    switch (method) {
        case 'GET':
            return {
                id: '{{uuid}}',
                data: {},
                message: 'Mock response',
                timestamp: new Date().toISOString(),
            };

        case 'POST':
            return {
                id: '{{uuid}}',
                ...(request.body || {}),
                created_at: new Date().toISOString(),
                message: 'Resource created',
            };

        case 'PUT':
        case 'PATCH':
            return {
                id: '{{uuid}}',
                ...(request.body || {}),
                updated_at: new Date().toISOString(),
                message: 'Resource updated',
            };

        case 'DELETE':
            return {
                message: 'Resource deleted',
                timestamp: new Date().toISOString(),
            };

        default:
            return {
                message: 'Mock response',
                method: request.method,
                path: request.path,
                timestamp: new Date().toISOString(),
            };
    }
}

/**
 * Generate a mock response from a captured request
 */
export function generateMockResponse(request: CapturedRequest): MockResponse {
    // Use actual response if available
    let body: any;

    if (request.responseBody !== undefined) {
        body = request.responseBody;
    } else if (request.error) {
        // For network errors, generate a default response
        body = generateDefaultResponse(request);
    } else {
        // Generate default response
        body = generateDefaultResponse(request);
    }

    // Determine status code
    let statusCode = 200;
    if (request.statusCode) {
        statusCode = request.statusCode;
    } else if (request.error) {
        // Network errors typically return 502 (Bad Gateway)
        statusCode = 502;
    } else if (request.method === 'POST') {
        statusCode = 201;
    }

    // Build response headers
    const headers: Record<string, string> = {
        'Content-Type': 'application/json',
    };

    // Copy relevant response headers if available
    if (request.responseHeaders) {
        const relevantHeaders = ['content-type', 'content-encoding', 'cache-control'];
        relevantHeaders.forEach(header => {
            const value = request.responseHeaders![header] || request.responseHeaders![header.toLowerCase()];
            if (value) {
                headers[header] = value;
            }
        });
    }

    return {
        body,
        headers: Object.keys(headers).length > 0 ? headers : undefined,
    };
}

/**
 * Generate a mock name from a request
 */
export function generateMockName(request: CapturedRequest): string {
    const method = request.method.toUpperCase();
    const path = request.path || '/';

    // Clean up path for name
    const pathParts = path.split('/').filter(Boolean);
    const lastPart = pathParts[pathParts.length - 1] || 'root';

    return `${method} ${path}`;
}
