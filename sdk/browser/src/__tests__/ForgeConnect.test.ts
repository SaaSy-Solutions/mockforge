/**
 * Unit tests for ForgeConnect
 */

import { ForgeConnect } from '../core/ForgeConnect';
import { MockForgeClient } from '../core/MockForgeClient';

jest.mock('../core/MockForgeClient');

describe('ForgeConnect', () => {
    let forgeConnect: ForgeConnect;

    beforeEach(() => {
        (global.fetch as jest.Mock).mockClear();
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
                healthCheck: jest.fn().mockResolvedValue(true),
                getBaseUrl: jest.fn().mockReturnValue('http://localhost:3000'),
            };

            (MockForgeClient as jest.Mock).mockImplementation(() => mockClient);

            const result = await forgeConnect.initialize();
            expect(result).toBe(true);
        });

        it('should return false when connection fails', async () => {
            const mockClient = {
                healthCheck: jest.fn().mockResolvedValue(false),
            };

            (MockForgeClient as jest.Mock).mockImplementation(() => mockClient);

            const result = await forgeConnect.initialize();
            expect(result).toBe(false);
        });
    });

    describe('createMockFromRequest', () => {
        it('should create a mock from a captured request', async () => {
            const mockClient = {
                healthCheck: jest.fn().mockResolvedValue(true),
                createMock: jest.fn().mockResolvedValue({
                    id: 'mock-id',
                    name: 'GET /api/test',
                    method: 'GET',
                    path: '/api/test',
                    response: { body: {} },
                }),
            };

            (MockForgeClient as jest.Mock).mockImplementation(() => mockClient);
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
            expect(status).toHaveProperty('url');
        });
    });
});

