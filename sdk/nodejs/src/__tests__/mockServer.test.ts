import * as http from 'http';
import { AddressInfo } from 'net';
import { EventEmitter } from 'events';
import * as childProcess from 'child_process';
import { MockServer, resolveManagementToken } from '../mockServer';

jest.mock('child_process', () => {
  const actual = jest.requireActual('child_process');
  // Pass through to the real spawn (integration tests) unless overridden.
  return { ...actual, spawn: jest.fn((...args: unknown[]) => actual.spawn(...args)) };
});

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

  describe('management token', () => {
    const saved = process.env.MOCKFORGE_MANAGEMENT_TOKEN;
    afterEach(() => {
      if (saved === undefined) delete process.env.MOCKFORGE_MANAGEMENT_TOKEN;
      else process.env.MOCKFORGE_MANAGEMENT_TOKEN = saved;
    });

    it('prefers the explicit option, then env, then a random token', () => {
      process.env.MOCKFORGE_MANAGEMENT_TOKEN = 'e'.repeat(40);
      expect(resolveManagementToken('x'.repeat(40))).toBe('x'.repeat(40));
      expect(resolveManagementToken()).toBe('e'.repeat(40));
      delete process.env.MOCKFORGE_MANAGEMENT_TOKEN;
      const a = resolveManagementToken();
      const b = resolveManagementToken();
      expect(a).toMatch(/^[0-9a-f]{64}$/);
      expect(a).not.toBe(b);
    });

    it('passes the token to the spawned CLI via env', async () => {
      const spawnMock = childProcess.spawn as unknown as jest.Mock;
      const fake = Object.assign(new EventEmitter(), {
        stdout: new EventEmitter(),
        stderr: new EventEmitter(),
        killed: false,
        kill: jest.fn(),
      });
      spawnMock.mockReturnValueOnce(fake);
      const token = 't'.repeat(40);
      const pending = MockServer.start({ managementToken: token, startupTimeoutMs: 1000 });
      setImmediate(() => fake.emit('exit', 1, null));
      await expect(pending).rejects.toBeDefined();
      expect(spawnMock).toHaveBeenCalledTimes(1);
      const opts = spawnMock.mock.calls[0][2];
      expect(opts.env.MOCKFORGE_MANAGEMENT_TOKEN).toBe(token);
    });

    it('sends the header on mock creation and verification requests', async () => {
      const seen: Array<{ url?: string; token?: string | string[] }> = [];
      const srv = http.createServer((req, res) => {
        seen.push({ url: req.url, token: req.headers['x-mockforge-management-token'] });
        req.resume();
        req.on('end', () => {
          res.setHeader('content-type', 'application/json');
          res.end(JSON.stringify({ id: 'm1', matched: true, count: 0, matches: [] }));
        });
      });
      await new Promise<void>((r) => srv.listen(0, '127.0.0.1', () => r()));
      try {
        const port = (srv.address() as AddressInfo).port;
        const token = 'k'.repeat(48);
        const server = new MockServer({ port, managementToken: token });
        await server.stubResponse('GET', '/x', { ok: true });
        await server.verify({ path: '/x' }, { type: 'never' });
        expect(seen.map((s) => s.url)).toEqual([
          '/__mockforge/api/mocks',
          '/api/verification/verify',
        ]);
        expect(seen.every((s) => s.token === token)).toBe(true);
      } finally {
        await new Promise<void>((r) => srv.close(() => r()));
      }
    });
  });

  describe('isRunning', () => {
    it('should return false before start', () => {
      const server = new MockServer();
      expect(server.isRunning()).toBe(false);
    });
  });

  // Integration tests spawn the real `mockforge` CLI, so they only run when
  // the caller explicitly opts in — NOT automatically under CI, because the
  // publish workflow runs on GitHub-hosted runners that don't have the CLI
  // installed. The main CI job that exercises integration tests installs
  // mockforge first and sets MOCKFORGE_INTEGRATION=1.
  const runIntegration = process.env.MOCKFORGE_INTEGRATION === '1';
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
