/**
 * Response Generator
 *
 * Generates mock responses from captured requests or templates
 * Enhanced with schema inference for realistic mock data generation
 */

import { CapturedRequest, MockResponse } from '../types';
import { inferSchemaFromResponse, InferredSchema } from './requestAnalyzer';

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
 * Generate realistic mock data from schema
 */
function generateDataFromSchema(schema: InferredSchema): any {
    if (schema.type === 'array') {
        // Generate 1-3 items for arrays
        const count = Math.floor(Math.random() * 3) + 1;
        const items: any[] = [];
        for (let i = 0; i < count; i++) {
            items.push(generateDataFromSchema(schema.items || { type: 'object' }));
        }
        return items;
    }

    if (schema.type === 'object' && schema.properties) {
        const obj: any = {};
        for (const [key, propSchema] of Object.entries(schema.properties)) {
            // Use example if available, otherwise generate
            if (propSchema.example !== undefined) {
                obj[key] = generateValueFromExample(propSchema.example, propSchema);
            } else {
                obj[key] = generateDataFromSchema(propSchema);
            }
        }
        return obj;
    }

    // Use example if available
    if (schema.example !== undefined) {
        return generateValueFromExample(schema.example, schema);
    }

    // Generate based on type
    switch (schema.type) {
        case 'string':
            if (schema.format === 'uuid') return '{{uuid}}';
            if (schema.format === 'email') return '{{faker.email}}';
            if (schema.format === 'date-time') return '{{now}}';
            return '{{faker.word}}';
        case 'integer':
            return '{{randInt 1 1000}}';
        case 'number':
            return '{{randFloat 0 100}}';
        case 'boolean':
            return true;
        default:
            return null;
    }
}

/**
 * Generate value from example, applying faker templates where appropriate
 */
function generateValueFromExample(example: any, schema: InferredSchema): any {
    if (typeof example === 'string') {
        // Apply faker templates based on format or content
        if (schema.format === 'uuid') return '{{uuid}}';
        if (schema.format === 'email') return '{{faker.email}}';
        if (schema.format === 'date-time') return '{{now}}';
        // Keep string as-is but wrap in template if it looks like a placeholder
        return example;
    }

    if (typeof example === 'number') {
        // For numbers, keep similar range but randomize
        if (schema.type === 'integer') {
            const range = Math.max(1, Math.abs(example));
            return `{{randInt 1 ${range * 2}}}`;
        }
        return `{{randFloat 0 ${Math.max(1, Math.abs(example) * 2)}}}`;
    }

    return example;
}

/**
 * Generate a mock response from a captured request
 * Enhanced with schema inference for unhandled requests
 */
export function generateMockResponse(request: CapturedRequest): MockResponse {
    let body: any;
    let inferredSchema: InferredSchema | null = null;

    // If we have a response body, use it and infer schema
    if (request.responseBody !== undefined && request.responseBody !== null) {
        body = request.responseBody;
        // Infer schema from actual response for future generation
        inferredSchema = inferSchemaFromResponse(request.responseBody);
    } else if (request.error) {
        // For network errors, generate a default response based on request
        body = generateDefaultResponse(request);
    } else {
        // For unhandled requests (404, etc.), try to infer expected structure
        // from request body or generate sensible defaults
        if (request.body && typeof request.body === 'object') {
            // If we have a request body, infer what the response might look like
            inferredSchema = inferSchemaFromResponse(request.body);
            // Generate response based on inferred schema
            if (inferredSchema) {
                body = generateDataFromSchema(inferredSchema);
            } else {
                body = generateDefaultResponse(request);
            }
        } else {
            body = generateDefaultResponse(request);
        }
    }

    // Determine status code
    let statusCode = 200;
    if (request.statusCode) {
        // For error status codes, keep them but generate valid response body
        statusCode = request.statusCode;
    } else if (request.error) {
        // Network errors typically return 502 (Bad Gateway)
        statusCode = 502;
    } else if (request.method === 'POST') {
        statusCode = 201;
    } else if (request.method === 'PUT' || request.method === 'PATCH') {
        statusCode = 200;
    } else if (request.method === 'DELETE') {
        statusCode = 204;
    }

    // Build response headers
    const headers: Record<string, string> = {
        'Content-Type': 'application/json',
    };

    // Copy relevant response headers if available
    if (request.responseHeaders) {
        const relevantHeaders = ['content-type', 'content-encoding', 'cache-control', 'x-request-id'];
        relevantHeaders.forEach(header => {
            const value = request.responseHeaders![header] || request.responseHeaders![header.toLowerCase()];
            if (value) {
                headers[header] = value;
            }
        });
    }

    // Add generated headers
    if (statusCode === 201) {
        headers['Location'] = `${request.path}/{{uuid}}`;
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
