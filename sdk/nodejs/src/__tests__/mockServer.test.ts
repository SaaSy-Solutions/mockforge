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

  // Gate: require MOCKFORGE_CLI_PATH to be set or the `mockforge` binary to be
  // on PATH. CI sets this; local devs can opt in with `MOCKFORGE_INTEGRATION=1`.
  const runIntegration =
    process.env.MOCKFORGE_INTEGRATION === '1' || !!process.env.CI;
  const d = runIntegration ? describe : describe.skip;

  d('Integration tests (require MockForge CLI)', () => {
    let server: MockServer;

    afterEach(async () => {
      if (server && server.isRunning()) {
        await server.stop();
      }
    });

    it('starts on a random port and stops cleanly', async () => {
      server = await MockServer.start({ port: 0, startupTimeoutMs: 30_000 });
      expect(server.isRunning()).toBe(true);
      expect(server.getPort()).toBeGreaterThan(0);
      await server.stop();
      expect(server.isRunning()).toBe(false);
    });

    it('serves a registered stub', async () => {
      server = await MockServer.start({ port: 0, startupTimeoutMs: 30_000 });
      await server.stubResponse('GET', '/api/users/123', { id: 123 });

      const res = await fetch(`${server.url()}/api/users/123`);
      expect(res.status).toBe(200);
      expect(await res.json()).toEqual({ id: 123 });
    });

    it('honours custom status codes and response headers', async () => {
      server = await MockServer.start({ port: 0, startupTimeoutMs: 30_000 });
      await server.stubResponse(
        'POST',
        '/api/widgets',
        { ok: true },
        { status: 201, headers: { 'X-Source': 'sdk-test' } }
      );

      const res = await fetch(`${server.url()}/api/widgets`, { method: 'POST' });
      expect(res.status).toBe(201);
      expect(res.headers.get('x-source')).toBe('sdk-test');
    });

    it('clearStubs removes all registered stubs', async () => {
      server = await MockServer.start({ port: 0, startupTimeoutMs: 30_000 });
      await server.stubResponse('GET', '/gone', { here: true });
      expect((await fetch(`${server.url()}/gone`)).status).toBe(200);

      await server.clearStubs();
      expect((await fetch(`${server.url()}/gone`)).status).toBe(404);
    });
  });
});
