import { vi, describe, it, beforeEach, afterEach, expect, type Mock } from 'vitest';
/**
 * Unit tests for ForgeConnect
 */

import { ForgeConnect } from '../core/ForgeConnect';
import { MockForgeClient } from '../core/MockForgeClient';

vi.mock('../core/MockForgeClient');

describe('ForgeConnect', () => {
    let forgeConnect: ForgeConnect;

    beforeEach(() => {
        global.fetch = vi.fn();
        forgeConnect = new ForgeConnect({
            serverUrl: 'http://localhost:3000',
            mockMode: 'auto',
        });
    });

    afterEach(() => {
        forgeConnect.stop();
    });

    describe('initialize', () => {
        it('should connect to MockForge successfully', async () => {
            const mockClient = {
                listMocks: vi.fn().mockResolvedValue([]),
                listEnvironments: vi.fn().mockResolvedValue([]),
                getActiveEnvironment: vi.fn().mockResolvedValue(null),
                healthCheck: vi.fn().mockResolvedValue(true),
                getBaseUrl: vi.fn().mockReturnValue('http://localhost:3000'),
            };

            (MockForgeClient as Mock).mockImplementation(function () { return mockClient; });

            const result = await forgeConnect.initialize();
            expect(result).toBe(true);
            expect(forgeConnect.getConnectionStatus()).toMatchObject({connected: true, url: 'http://localhost:3000'});
        });

        it('should return false when connection fails', async () => {
            const mockClient = {
                listMocks: vi.fn().mockResolvedValue([]),
                listEnvironments: vi.fn().mockResolvedValue([]),
                getActiveEnvironment: vi.fn().mockResolvedValue(null),
                healthCheck: vi.fn().mockResolvedValue(false),
            };

            (MockForgeClient as Mock).mockImplementation(function () { return mockClient; });

            const result = await forgeConnect.initialize();
            expect(result).toBe(false);
        });
    });

    describe('createMockFromRequest', () => {
        it('should create a mock from a captured request', async () => {
            const mockClient = {
                listMocks: vi.fn().mockResolvedValue([]),
                listEnvironments: vi.fn().mockResolvedValue([]),
                getActiveEnvironment: vi.fn().mockResolvedValue(null),
                healthCheck: vi.fn().mockResolvedValue(true),
                createMock: vi.fn().mockResolvedValue({
                    id: 'mock-id',
                    name: 'GET /api/test',
                    method: 'GET',
                    path: '/api/test',
                    response: { body: {} },
                }),
            };

            (MockForgeClient as Mock).mockImplementation(function () { return mockClient; });
            await forgeConnect.initialize();

            const request = {
                method: 'GET',
                url: 'http://localhost:3000/api/test',
                path: '/api/test',
                timestamp: Date.now(),
            };

            const result = await forgeConnect.createMockFromRequest(request);
            expect(result).toBeDefined();
            expect(result?.id).toBe('mock-id');
        });
    });

    describe('getConnectionStatus', () => {
        it('should return connection status', () => {
            const status = forgeConnect.getConnectionStatus();
            expect(status).toHaveProperty('connected');
            expect(status.connected).toBe(false);
            expect(status.url).toBeUndefined();
        });
    });
});
