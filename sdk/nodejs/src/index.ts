/**
 * MockForge SDK for Node.js
 *
 * Embed MockForge mock servers directly in your tests
 *
 * @example
 * ```typescript
 * import { MockServer } from '@mockforge/sdk';
 *
 * describe('API Tests', () => {
 *   let server: MockServer;
 *
 *   beforeEach(async () => {
 *     server = await MockServer.start({ port: 3000 });
 *   });
 *
 *   afterEach(async () => {
 *     await server.stop();
 *   });
 *
 *   it('should mock user API', async () => {
 *     await server.stubResponse('GET', '/api/users/123', {
 *       id: 123,
 *       name: 'John Doe'
 *     });
 *
 *     const response = await fetch('http://localhost:3000/api/users/123');
 *     const data = await response.json();
 *     expect(data.id).toBe(123);
 *   });
 * });
 * ```
 */

export { MockServer } from './mockServer';
export { StubBuilder } from './stubBuilder';
export { MockServerError, MockServerErrorCode } from './errors';
export type { MockServerConfig, ResponseStub, StubOptions, VerificationRequest, VerificationCount, VerificationResult } from './types';
export * from './types';
