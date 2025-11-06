/**
 * Unit tests for request analyzer utilities
 */

import {
    extractPath,
    extractQueryParams,
    parseHeaders,
    shouldCreateMock,
} from '../../utils/requestAnalyzer';
import { CapturedRequest } from '../../types';

describe('requestAnalyzer', () => {
    describe('extractPath', () => {
        it('should extract path from URL', () => {
            expect(extractPath('http://localhost:3000/api/users?page=1')).toBe('/api/users');
        });

        it('should handle URLs without query string', () => {
            expect(extractPath('http://localhost:3000/api/users')).toBe('/api/users');
        });
    });

    describe('extractQueryParams', () => {
        it('should extract query parameters', () => {
            const params = extractQueryParams('http://localhost:3000/api/users?page=1&limit=10');
            expect(params.page).toBe('1');
            expect(params.limit).toBe('10');
        });

        it('should return empty object for URLs without query string', () => {
            const params = extractQueryParams('http://localhost:3000/api/users');
            expect(Object.keys(params)).toHaveLength(0);
        });
    });

    describe('parseHeaders', () => {
        it('should parse Headers object', () => {
            const headers = new Headers();
            headers.set('Content-Type', 'application/json');
            headers.set('Authorization', 'Bearer token');

            const parsed = parseHeaders(headers);
            expect(parsed['Content-Type']).toBe('application/json');
            expect(parsed['Authorization']).toBe('Bearer token');
        });

        it('should parse record object', () => {
            const headers = {
                'Content-Type': 'application/json',
                'Authorization': 'Bearer token',
            };

            const parsed = parseHeaders(headers);
            expect(parsed).toEqual(headers);
        });
    });

    describe('shouldCreateMock', () => {
        it('should return true for network errors', () => {
            const request: CapturedRequest = {
                method: 'GET',
                url: 'http://localhost:3000/api/test',
                path: '/api/test',
                error: { type: 'network', message: 'Failed to fetch' },
                timestamp: Date.now(),
            };

            expect(shouldCreateMock(request, [], true)).toBe(true);
        });

        it('should return true for configured status codes', () => {
            const request: CapturedRequest = {
                method: 'GET',
                url: 'http://localhost:3000/api/test',
                path: '/api/test',
                statusCode: 404,
                timestamp: Date.now(),
            };

            expect(shouldCreateMock(request, [404, 500], false)).toBe(true);
        });

        it('should return false for successful requests', () => {
            const request: CapturedRequest = {
                method: 'GET',
                url: 'http://localhost:3000/api/test',
                path: '/api/test',
                statusCode: 200,
                timestamp: Date.now(),
            };

            expect(shouldCreateMock(request, [404, 500], false)).toBe(false);
        });
    });
});
