/**
 * Request Analyzer
 *
 * Analyzes captured requests to extract information for mock creation
 */

import { CapturedRequest } from '../types';

/**
 * Extract path from URL (without query string)
 */
export function extractPath(url: string): string {
    try {
        const urlObj = new URL(url);
        return urlObj.pathname;
    } catch {
        // If URL parsing fails, try to extract path manually
        const queryIndex = url.indexOf('?');
        if (queryIndex !== -1) {
            return url.substring(0, queryIndex);
        }
        return url;
    }
}

/**
 * Extract query parameters from URL
 */
export function extractQueryParams(url: string): Record<string, string> {
    try {
        const urlObj = new URL(url);
        const params: Record<string, string> = {};
        urlObj.searchParams.forEach((value, key) => {
            params[key] = value;
        });
        return params;
    } catch {
        return {};
    }
}

/**
 * Parse request headers from Headers object or array
 */
export function parseHeaders(headers: Headers | Record<string, string> | string[][]): Record<string, string> {
    const result: Record<string, string> = {};

    if (headers instanceof Headers) {
        headers.forEach((value, key) => {
            result[key] = value;
        });
    } else if (Array.isArray(headers)) {
        headers.forEach(([key, value]) => {
            result[key] = value;
        });
    } else {
        Object.assign(result, headers);
    }

    return result;
}

/**
 * Parse request body based on content type
 */
export async function parseBody(body: any, contentType?: string): Promise<any> {
    if (!body) {
        return undefined;
    }

    // If already parsed, return as-is
    if (typeof body === 'object' && !(body instanceof FormData) && !(body instanceof Blob)) {
        return body;
    }

    // If it's a string, try to parse based on content type
    if (typeof body === 'string') {
        const ct = contentType?.toLowerCase() || '';

        if (ct.includes('application/json')) {
            try {
                return JSON.parse(body);
            } catch {
                return body;
            }
        }

        if (ct.includes('application/x-www-form-urlencoded')) {
            const params: Record<string, string> = {};
            body.split('&').forEach(param => {
                const [key, value] = param.split('=');
                if (key) {
                    params[decodeURIComponent(key)] = decodeURIComponent(value || '');
                }
            });
            return params;
        }

        return body;
    }

    // For other types (FormData, Blob, etc.), return as-is
    return body;
}

/**
 * Analyze a captured request and extract all relevant information
 */
export async function analyzeRequest(
    method: string,
    url: string,
    headers: Headers | Record<string, string> | string[][] = {},
    body?: any,
    response?: Response
): Promise<CapturedRequest> {
    const parsedHeaders = parseHeaders(headers);
    const contentType = parsedHeaders['content-type'] || parsedHeaders['Content-Type'];
    const parsedBody = await parseBody(body, contentType);

    const path = extractPath(url);
    const queryParams = extractQueryParams(url);

    let responseBody: any;
    let responseHeaders: Record<string, string> = {};
    let statusCode: number | undefined;

    if (response) {
        statusCode = response.status;
        responseHeaders = parseHeaders(response.headers);

        // Try to parse response body
        const responseContentType = responseHeaders['content-type'] || responseHeaders['Content-Type'] || '';
        if (responseContentType.includes('application/json')) {
            try {
                const text = await response.clone().text();
                responseBody = JSON.parse(text);
            } catch {
                // If parsing fails, don't include body
            }
        }
    }

    return {
        method: method.toUpperCase(),
        url,
        path,
        queryParams: Object.keys(queryParams).length > 0 ? queryParams : undefined,
        headers: Object.keys(parsedHeaders).length > 0 ? parsedHeaders : undefined,
        body: parsedBody,
        statusCode,
        responseBody,
        responseHeaders: Object.keys(responseHeaders).length > 0 ? responseHeaders : undefined,
        timestamp: Date.now(),
    };
}

/**
 * Check if a request should trigger mock creation
 * Enhanced to detect unhandled requests (404/500/network errors)
 */
export function shouldCreateMock(
    request: CapturedRequest,
    autoMockStatusCodes: number[] = [404, 500, 502, 503, 504],
    autoMockNetworkErrors: boolean = true
): boolean {
    // Check for network errors (timeout, CORS, connection refused, etc.)
    if (request.error && autoMockNetworkErrors) {
        return true;
    }

    // Check for HTTP error status codes
    if (request.statusCode) {
        // 4xx errors (client errors) - typically unhandled endpoints
        if (request.statusCode >= 400 && request.statusCode < 500) {
            if (autoMockStatusCodes.includes(request.statusCode)) {
                return true;
            }
            // Also auto-mock 404s by default (endpoint not found)
            if (request.statusCode === 404) {
                return true;
            }
        }

        // 5xx errors (server errors) - typically backend issues
        if (request.statusCode >= 500 && request.statusCode < 600) {
            if (autoMockStatusCodes.includes(request.statusCode)) {
                return true;
            }
        }
    }

    // Check for requests that failed but don't have explicit error info
    // This catches cases where the request was made but no response was received
    if (!request.statusCode && !request.responseBody && !request.error) {
        // Likely an unhandled request that didn't complete
        return false; // Don't auto-mock incomplete requests
    }

    return false;
}

/**
 * Infer schema from response body structure
 * Creates an OpenAPI-like schema from actual API response
 */
export interface InferredSchema {
    type: string;
    properties?: Record<string, InferredSchema>;
    items?: InferredSchema;
    required?: string[];
    example?: any;
}

export function inferSchemaFromResponse(responseBody: any): InferredSchema | null {
    if (responseBody === null || responseBody === undefined) {
        return { type: 'null' };
    }

    // Handle arrays
    if (Array.isArray(responseBody)) {
        if (responseBody.length === 0) {
            return {
                type: 'array',
                items: { type: 'object' },
            };
        }

        // Infer schema from first item
        const itemSchema = inferSchemaFromValue(responseBody[0]);
        return {
            type: 'array',
            items: itemSchema,
        };
    }

    // Handle objects
    if (typeof responseBody === 'object') {
        return inferSchemaFromValue(responseBody);
    }

    // Handle primitives
    return inferSchemaFromValue(responseBody);
}

/**
 * Infer schema from a single value
 */
function inferSchemaFromValue(value: any): InferredSchema {
    if (value === null || value === undefined) {
        return { type: 'null' };
    }

    if (Array.isArray(value)) {
        if (value.length === 0) {
            return { type: 'array', items: { type: 'object' } };
        }
        return {
            type: 'array',
            items: inferSchemaFromValue(value[0]),
        };
    }

    if (typeof value === 'object') {
        const properties: Record<string, InferredSchema> = {};
        const required: string[] = [];

        for (const [key, val] of Object.entries(value)) {
            const propSchema = inferSchemaFromValue(val);
            properties[key] = propSchema;

            // Consider non-null values as required
            if (val !== null && val !== undefined) {
                required.push(key);
            }
        }

        return {
            type: 'object',
            properties,
            required: required.length > 0 ? required : undefined,
            example: value,
        };
    }

    // Primitive types
    if (typeof value === 'string') {
        // Try to detect format
        if (/^\d{4}-\d{2}-\d{2}/.test(value)) {
            return { type: 'string', format: 'date-time', example: value };
        }
        if (/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)) {
            return { type: 'string', format: 'uuid', example: value };
        }
        if (/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) {
            return { type: 'string', format: 'email', example: value };
        }
        return { type: 'string', example: value };
    }

    if (typeof value === 'number') {
        return Number.isInteger(value)
            ? { type: 'integer', example: value }
            : { type: 'number', format: 'float', example: value };
    }

    if (typeof value === 'boolean') {
        return { type: 'boolean', example: value };
    }

    return { type: 'string', example: String(value) };
}
