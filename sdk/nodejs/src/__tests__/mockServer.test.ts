import { MockServer } from '../mockServer';

describe('MockServer', () => {
  describe('constructor', () => {
    it('should create a server with default config', () => {
      const server = new MockServer();
      expect(server).toBeDefined();
      expect(server.getPort()).toBe(0);
    });

    it('should create a server with custom port', () => {
      const server = new MockServer({ port: 3000 });
      expect(server.getPort()).toBe(3000);
    });

    it('should create a server with custom host', () => {
      const server = new MockServer({ host: '0.0.0.0' });
      expect(server).toBeDefined();
    });
  });

  describe('url', () => {
    it('should return correct URL', () => {
      const server = new MockServer({ port: 3000, host: '127.0.0.1' });
      expect(server.url()).toBe('http://127.0.0.1:3000');
    });
  });

  describe('isRunning', () => {
    it('should return false before start', () => {
      const server = new MockServer();
      expect(server.isRunning()).toBe(false);
    });
  });

  // Note: Integration tests that actually start the server are skipped
  // because they require the MockForge CLI to be installed
  describe.skip('Integration tests (require MockForge CLI)', () => {
    let server: MockServer;

    afterEach(async () => {
      if (server && server.isRunning()) {
        await server.stop();
      }
    });

    it('should start and stop server', async () => {
      server = await MockServer.start({ port: 3456 });
      expect(server.isRunning()).toBe(true);
      await server.stop();
      expect(server.isRunning()).toBe(false);
    });

    it('should stub a response', async () => {
      server = await MockServer.start({ port: 3457 });

      await server.stubResponse('GET', '/test', { message: 'hello' });

      // Would test actual HTTP request here
      await server.stop();
    });
  });
});
