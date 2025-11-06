/**
 * Unit tests for MockForgeClient
 */

import { MockForgeClient } from '../core/MockForgeClient';

describe('MockForgeClient', () => {
    const baseUrl = 'http://localhost:3000';
    let client: MockForgeClient;

    beforeEach(() => {
        client = new MockForgeClient(baseUrl);
        (global.fetch as jest.Mock).mockClear();
    });

    describe('healthCheck', () => {
        it('should return true when health endpoint responds with 200', async () => {
            (global.fetch as jest.Mock).mockResolvedValueOnce({
                ok: true,
                status: 200,
            });

            const result = await client.healthCheck();
            expect(result).toBe(true);
            expect(global.fetch).toHaveBeenCalledWith(
                `${baseUrl}/health`,
                expect.objectContaining({
                    method: 'GET',
                })
            );
        });

        it('should return true when mocks endpoint responds with 200', async () => {
            (global.fetch as jest.Mock)
                .mockResolvedValueOnce({ ok: false, status: 404 }) // Health fails
                .mockResolvedValueOnce({ ok: true, status: 200 }); // Mocks succeeds

            const result = await client.healthCheck();
            expect(result).toBe(true);
        });

        it('should return false when all endpoints fail', async () => {
            (global.fetch as jest.Mock)
                .mockRejectedValueOnce(new Error('Network error'))
                .mockRejectedValueOnce(new Error('Network error'));

            const result = await client.healthCheck();
            expect(result).toBe(false);
        });
    });

    describe('createMock', () => {
        it('should create a mock successfully', async () => {
            const mock = {
                name: 'Test Mock',
                method: 'GET',
                path: '/api/test',
                response: { body: { message: 'test' } },
            };

            (global.fetch as jest.Mock).mockResolvedValueOnce({
                ok: true,
                status: 200,
                json: async () => ({ ...mock, id: 'mock-id' }),
            });

            const result = await client.createMock(mock);
            expect(result.id).toBe('mock-id');
            expect(global.fetch).toHaveBeenCalledWith(
                `${baseUrl}/mocks`,
                expect.objectContaining({
                    method: 'POST',
                    headers: expect.objectContaining({
                        'Content-Type': 'application/json',
                    }),
                    body: JSON.stringify(mock),
                })
            );
        });

        it('should throw error when creation fails', async () => {
            const mock = {
                name: 'Test Mock',
                method: 'GET',
                path: '/api/test',
                response: { body: {} },
            };

            (global.fetch as jest.Mock).mockResolvedValueOnce({
                ok: false,
                status: 400,
                statusText: 'Bad Request',
                text: async () => 'Error message',
            });

            await expect(client.createMock(mock)).rejects.toThrow();
        });
    });

    describe('listMocks', () => {
        it('should list all mocks', async () => {
            const mocks = [
                { id: '1', name: 'Mock 1', method: 'GET', path: '/api/1', response: { body: {} } },
                { id: '2', name: 'Mock 2', method: 'POST', path: '/api/2', response: { body: {} } },
            ];

            (global.fetch as jest.Mock).mockResolvedValueOnce({
                ok: true,
                status: 200,
                json: async () => ({ mocks }),
            });

            const result = await client.listMocks();
            expect(result).toEqual(mocks);
        });
    });

    describe('deleteMock', () => {
        it('should delete a mock successfully', async () => {
            (global.fetch as jest.Mock).mockResolvedValueOnce({
                ok: true,
                status: 204,
            });

            await client.deleteMock('mock-id');
            expect(global.fetch).toHaveBeenCalledWith(
                `${baseUrl}/mocks/mock-id`,
                expect.objectContaining({
                    method: 'DELETE',
                })
            );
        });
    });
});

