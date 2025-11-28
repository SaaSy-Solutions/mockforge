/**
 * Integration tests for ForgeConnect
 *
 * These tests require a running MockForge server on localhost:3000
 * Run with: MOCKFORGE_SERVER_URL=http://localhost:3000 npm test -- --testPathPattern=integration
 */

import { ForgeConnect } from '../../core/ForgeConnect';

const MOCKFORGE_URL = process.env.MOCKFORGE_SERVER_URL || 'http://localhost:3000';

describe('ForgeConnect Integration', () => {
    let forgeConnect: ForgeConnect;

    beforeAll(async () => {
        // Check if MockForge is running
        try {
            const response = await fetch(`${MOCKFORGE_URL}/health`);
            if (!response.ok) {
                throw new Error('MockForge not running');
            }
        } catch (error) {
            console.warn('MockForge server not available, skipping integration tests');
            return;
        }
    });

    beforeEach(() => {
        forgeConnect = new ForgeConnect({
            serverUrl: MOCKFORGE_URL,
            mockMode: 'auto',
            enableLogging: false,
        });
    });

    afterEach(() => {
        forgeConnect.stop();
    });

    it('should connect to MockForge server', async () => {
        const connected = await forgeConnect.initialize();
        expect(connected).toBe(true);

        const status = forgeConnect.getConnectionStatus();
        expect(status.connected).toBe(true);
        expect(status.url).toBe(MOCKFORGE_URL);
    });

    it('should create a mock', async () => {
        await forgeConnect.initialize();

        const mock = await forgeConnect.createMock({
            name: 'Test Mock',
            method: 'GET',
            path: '/api/test',
            response: {
                body: { message: 'test' },
            },
        });

        expect(mock.id).toBeDefined();
        expect(mock.name).toBe('Test Mock');
    });

    it('should list mocks', async () => {
        await forgeConnect.initialize();

        // Create a mock first
        await forgeConnect.createMock({
            name: 'List Test Mock',
            method: 'GET',
            path: '/api/list-test',
            response: { body: {} },
        });

        const mocks = await forgeConnect.listMocks();
        expect(Array.isArray(mocks)).toBe(true);

        // Should contain our mock
        const ourMock = mocks.find(m => m.path === '/api/list-test');
        expect(ourMock).toBeDefined();
    });

    it('should delete a mock', async () => {
        await forgeConnect.initialize();

        // Create a mock
        const mock = await forgeConnect.createMock({
            name: 'Delete Test Mock',
            method: 'GET',
            path: '/api/delete-test',
            response: { body: {} },
        });

        expect(mock.id).toBeDefined();

        // Delete it
        await forgeConnect.deleteMock(mock.id!);

        // Verify it's gone
        const mocks = await forgeConnect.listMocks();
        const deletedMock = mocks.find(m => m.id === mock.id);
        expect(deletedMock).toBeUndefined();
    });
});
