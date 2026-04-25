import { spawn, ChildProcess } from 'child_process';
import { promisify } from 'util';
import axios from 'axios';
import {
  MockServerConfig,
  ResponseStub,
  StubOptions,
  VerificationRequest,
  VerificationCount,
  VerificationResult,
} from './types';
import { MockServerError, MockServerErrorCode } from './errors';

const sleep = promisify(setTimeout);

/**
 * MockServer - Embedded mock server for testing
 */
export class MockServer {
  private process: ChildProcess | null = null;
  private port: number;
  private host: string;
  private adminPort: number;
  private enableAdminUi: boolean;
  private wsPort: number;
  private grpcPort: number;
  private metricsPort: number;
  private noConfig: boolean;
  private startupTimeoutMs: number;
  private stubs: ResponseStub[] = [];
  // Track stub IDs returned from Admin API for later updates/deletes
  private stubIds: Map<string, string> = new Map(); // key: "METHOD:path", value: mock ID
  private stderrBuffer = '';

  constructor(private config: MockServerConfig = {}) {
    this.port = config.port ?? 0;
    this.host = config.host ?? '127.0.0.1';
    // Admin UI is opt-in for the SDK: the management API that stubs go
    // through lives on the HTTP port, not the admin port. Users who want to
    // browse the admin UI during tests can pass `adminPort` explicitly.
    this.enableAdminUi = config.adminPort !== undefined;
    this.adminPort = config.adminPort ?? 0;
    this.wsPort = config.wsPort ?? 0;
    this.grpcPort = config.grpcPort ?? 0;
    this.metricsPort = config.metricsPort ?? 0;
    this.noConfig = config.noConfig ?? true;
    this.startupTimeoutMs = config.startupTimeoutMs ?? 12_000;
  }

  /**
   * Start the mock server
   */
  static async start(config: MockServerConfig = {}): Promise<MockServer> {
    const server = new MockServer(config);
    await server.startServer();
    return server;
  }

  /**
   * Start the server process
   */
  private async startServer(): Promise<void> {
    const args = ['serve'];

    if (this.config.configFile) {
      args.push('--config', this.config.configFile);
    } else if (this.noConfig) {
      args.push('--no-config');
    }

    if (this.config.openApiSpec) {
      args.push('--spec', this.config.openApiSpec);
    }

    // Always pass every protocol port so we never inherit a fixed default
    // (3001/50051/…) that could collide with another MockForge on the machine.
    // Port 0 = ephemeral/disabled depending on the protocol; see MockServerConfig.
    args.push('--http-port', String(this.port));
    args.push('--ws-port', String(this.wsPort));
    args.push('--grpc-port', String(this.grpcPort));
    args.push('--metrics-port', String(this.metricsPort));

    // The management API (used for dynamic stubbing) is served on the HTTP
    // port itself at `/__mockforge/api/*`, so we don't need the admin UI for
    // the SDK to work. We only spawn it if the caller explicitly opted in
    // with `adminPort`.
    if (this.enableAdminUi) {
      args.push('--admin', '--admin-port', String(this.adminPort));
    }

    try {
      this.process = spawn('mockforge', args, {
        stdio: ['ignore', 'pipe', 'pipe'],
      });
    } catch (error) {
      throw MockServerError.cliNotFound(error instanceof Error ? error : undefined);
    }

    // Track if process failed to start
    let processStartError: Error | null = null;
    this.process.on('error', (error: Error) => {
      processStartError = error;
    });

    // Collect stdout to parse port information
    let stdoutBuffer = '';
    if (this.process.stdout) {
      this.process.stdout.on('data', (data: Buffer) => {
        stdoutBuffer += data.toString();
        this.parsePortsFromOutput(stdoutBuffer);
      });
    }

    // Collect stderr so we can surface it if the process dies early
    if (this.process.stderr) {
      this.process.stderr.on('data', (data: Buffer) => {
        this.stderrBuffer += data.toString();
      });
    }

    let earlyExit: { code: number | null; signal: NodeJS.Signals | null } | null = null;
    this.process.on('exit', (code, signal) => {
      earlyExit = { code, signal };
    });

    if (processStartError) {
      throw MockServerError.cliNotFound(processStartError);
    }
    await this.waitForServer(() => earlyExit);
  }

  /**
   * Parse port numbers from MockForge CLI output.
   *
   * The CLI prints two kinds of lines per protocol:
   *   1. `📡 HTTP server on port 0`            (early banner, uses the requested port)
   *   2. `📡 HTTP server listening on http://localhost:39647`  (post-bind, actual port)
   *
   * We deliberately only match (2), via `listening on https?://HOST:PORT`, so
   * ephemeral-port runs (`port: 0`) pick up the real OS-assigned port instead
   * of latching onto `0` from the banner.
   *
   * We also scan for ALL matches (global flag) and take the last — if the CLI
   * ever emits the message twice, the more recent one wins.
   */
  private parsePortsFromOutput(output: string): void {
    const lastMatch = (re: RegExp): number | null => {
      const all = [...output.matchAll(re)];
      if (all.length === 0) return null;
      const port = parseInt(all[all.length - 1][1], 10);
      return port > 0 ? port : null;
    };

    const httpPort = lastMatch(/HTTP server listening on https?:\/\/[^:/]+:(\d+)/g);
    if (httpPort !== null) this.port = httpPort;

    const adminPort = lastMatch(/Admin UI listening on https?:\/\/[^:/]+:(\d+)/g);
    if (adminPort !== null) this.adminPort = adminPort;
  }

  /**
   * Wait for the server to be ready
   */
  private async waitForServer(
    getEarlyExit: () => { code: number | null; signal: NodeJS.Signals | null } | null
  ): Promise<void> {
    const retryDelay = 200;
    const deadline = Date.now() + this.startupTimeoutMs;

    while (Date.now() < deadline) {
      const exited = getEarlyExit();
      if (exited) {
        throw MockServerError.serverStartFailed(
          `mockforge exited with code=${exited.code ?? 'null'} signal=${
            exited.signal ?? 'null'
          } before the HTTP server came up.\nstderr:\n${this.stderrBuffer.trim()}`
        );
      }

      // Wait for the HTTP port (required for both health and stub registration)
      // and, if the caller requested the admin UI, its port too. The main
      // blocker used to be admin port detection — the management API is on
      // the HTTP port, so the admin UI is a pure bonus for humans browsing
      // the server during a test run.
      const needAdmin = this.enableAdminUi && this.adminPort === 0;
      if (this.port === 0 || needAdmin) {
        await sleep(retryDelay);
        continue;
      }

      try {
        await axios.get(`http://${this.host}:${this.port}/health`, {
          timeout: 200,
        });
        return;
      } catch {
        await sleep(retryDelay);
      }
    }

    if (this.port === 0 || (this.enableAdminUi && this.adminPort === 0)) {
      throw MockServerError.portDetectionFailed(
        new Error(
          `Could not read bound ports from CLI stdout (http=${this.port}, admin=${this.adminPort}). ` +
            `This usually means the CLI exited early. stderr:\n${this.stderrBuffer.trim()}`
        )
      );
    }

    throw MockServerError.healthCheckTimeout(this.startupTimeoutMs, this.port);
  }

  /**
   * Require that the HTTP management API is available; throws otherwise so
   * tests fail fast instead of silently no-op'ing the stub registration.
   */
  private requireManagement(operation: string): void {
    if (this.port) return;
    throw MockServerError.adminApiError(
      operation,
      'HTTP port is unknown. The CLI did not report the HTTP port before startup completed.'
    );
  }

  /**
   * Stub a response
   */
  async stubResponse(
    method: string,
    path: string,
    body: any,
    options: StubOptions = {}
  ): Promise<void> {
    this.requireManagement('stubResponse');

    const stub: ResponseStub = {
      method: method.toUpperCase(),
      path,
      status: options.status || 200,
      headers: options.headers || {},
      body,
      latencyMs: options.latencyMs,
    };

    this.stubs.push(stub);

    const mockConfig = {
      id: '',
      name: `${method.toUpperCase()} ${path}`,
      method: stub.method,
      path: stub.path,
      response: {
        body: stub.body,
        headers:
          stub.headers && Object.keys(stub.headers).length > 0 ? stub.headers : undefined,
      },
      enabled: true,
      latency_ms: stub.latencyMs || undefined,
      status_code: stub.status !== 200 ? stub.status : undefined,
    };

    try {
      const response = await axios.post(
        `http://${this.host}:${this.port}/__mockforge/api/mocks`,
        mockConfig
      );
      const stubKey = `${stub.method}:${stub.path}`;
      if (response.data && response.data.id) {
        this.stubIds.set(stubKey, response.data.id);
      }
    } catch (error) {
      throw MockServerError.adminApiError(
        'stubResponse',
        error instanceof Error ? error.message : String(error),
        error instanceof Error ? error : undefined
      );
    }
  }

  /**
   * Update an existing stub
   */
  async updateStub(
    method: string,
    path: string,
    body: any,
    options: StubOptions = {}
  ): Promise<void> {
    this.requireManagement('updateStub');

    const stubKey = `${method.toUpperCase()}:${path}`;
    let mockId = this.stubIds.get(stubKey);

    if (!mockId) {
      // Mock wasn't created through this instance; try to locate it by
      // listing the Admin API's current mocks and matching on method+path.
      try {
        const response = await axios.get(
          `http://${this.host}:${this.port}/__mockforge/api/mocks`
        );
        const mocks = response.data.mocks || [];
        const mock = mocks.find(
          (m: any) => m.method.toUpperCase() === method.toUpperCase() && m.path === path
        );
        if (!mock) {
          return this.stubResponse(method, path, body, options);
        }
        mockId = mock.id;
        this.stubIds.set(stubKey, mock.id);
      } catch (error) {
        throw MockServerError.adminApiError(
          'updateStub.lookup',
          error instanceof Error ? error.message : String(error),
          error instanceof Error ? error : undefined
        );
      }
    }

    const stub: ResponseStub = {
      method: method.toUpperCase(),
      path,
      status: options.status || 200,
      headers: options.headers || {},
      body,
      latencyMs: options.latencyMs,
    };

    const mockConfig = {
      id: mockId,
      name: `${method.toUpperCase()} ${path}`,
      method: stub.method,
      path: stub.path,
      response: {
        body: stub.body,
        headers:
          stub.headers && Object.keys(stub.headers).length > 0 ? stub.headers : undefined,
      },
      enabled: true,
      latency_ms: stub.latencyMs || undefined,
      status_code: stub.status !== 200 ? stub.status : undefined,
    };

    try {
      await axios.put(
        `http://${this.host}:${this.port}/__mockforge/api/mocks/${mockId}`,
        mockConfig
      );
    } catch (error) {
      throw MockServerError.adminApiError(
        'updateStub',
        error instanceof Error ? error.message : String(error),
        error instanceof Error ? error : undefined
      );
    }
  }

  /**
   * Remove a specific stub
   */
  async removeStub(method: string, path: string): Promise<void> {
    this.requireManagement('removeStub');

    const stubKey = `${method.toUpperCase()}:${path}`;
    const mockId = this.stubIds.get(stubKey);

    this.stubs = this.stubs.filter(
      (s) => !(s.method.toUpperCase() === method.toUpperCase() && s.path === path)
    );
    this.stubIds.delete(stubKey);

    const doDelete = async (id: string) => {
      await axios.delete(
        `http://${this.host}:${this.port}/__mockforge/api/mocks/${id}`
      );
    };

    try {
      if (mockId) {
        await doDelete(mockId);
        return;
      }
      // Mock wasn't created by this instance; locate by method+path.
      const response = await axios.get(
        `http://${this.host}:${this.port}/__mockforge/api/mocks`
      );
      const mocks = response.data.mocks || [];
      const mock = mocks.find(
        (m: any) => m.method.toUpperCase() === method.toUpperCase() && m.path === path
      );
      if (mock) {
        await doDelete(mock.id);
      }
    } catch (error) {
      throw MockServerError.adminApiError(
        'removeStub',
        error instanceof Error ? error.message : String(error),
        error instanceof Error ? error : undefined
      );
    }
  }

  /**
   * Clear all stubs
   */
  async clearStubs(): Promise<void> {
    this.requireManagement('clearStubs');

    this.stubs = [];
    this.stubIds.clear();

    try {
      const response = await axios.get(
        `http://${this.host}:${this.port}/__mockforge/api/mocks`
      );
      const mocks = response.data.mocks || [];

      await Promise.all(
        mocks.map((mock: { id: string }) =>
          axios
            .delete(`http://${this.host}:${this.port}/__mockforge/api/mocks/${mock.id}`)
            .catch(() => {})
        )
      );
    } catch (error) {
      throw MockServerError.adminApiError(
        'clearStubs',
        error instanceof Error ? error.message : String(error),
        error instanceof Error ? error : undefined
      );
    }
  }

  /**
   * Get the server URL
   */
  url(): string {
    return `http://${this.host}:${this.port}`;
  }

  /**
   * Get the server port
   */
  getPort(): number {
    return this.port;
  }

  /**
   * Get the Admin UI port (0 if not yet detected)
   */
  getAdminPort(): number {
    return this.adminPort;
  }

  /**
   * Check if the server is running
   */
  isRunning(): boolean {
    return this.process !== null && !this.process.killed;
  }

  /**
   * Stop the mock server
   */
  async stop(): Promise<void> {
    if (this.process) {
      this.process.kill('SIGTERM');

      await new Promise<void>((resolve) => {
        if (this.process) {
          this.process.on('exit', () => resolve());
          setTimeout(() => resolve(), 2000);
        } else {
          resolve();
        }
      });

      this.process = null;
    }
  }

  /**
   * Verify requests against a pattern and count assertion
   */
  async verify(
    pattern: VerificationRequest,
    expected: VerificationCount
  ): Promise<VerificationResult> {
    try {
      const response = await axios.post(
        `${this.url()}/api/verification/verify`,
        {
          pattern: {
            method: pattern.method,
            path: pattern.path,
            query_params: pattern.queryParams || {},
            headers: pattern.headers || {},
            body_pattern: pattern.bodyPattern,
          },
          expected,
        }
      );
      return response.data;
    } catch (error: any) {
      return {
        matched: false,
        count: 0,
        expected,
        matches: [],
        errorMessage: `Verification API request failed: ${error.message}`,
      };
    }
  }

  /**
   * Verify that a request was never made
   */
  async verifyNever(pattern: VerificationRequest): Promise<VerificationResult> {
    try {
      const response = await axios.post(
        `${this.url()}/api/verification/never`,
        {
          method: pattern.method,
          path: pattern.path,
          query_params: pattern.queryParams || {},
          headers: pattern.headers || {},
          body_pattern: pattern.bodyPattern,
        }
      );
      return response.data;
    } catch (error: any) {
      return {
        matched: false,
        count: 0,
        expected: { type: 'never' },
        matches: [],
        errorMessage: `Verification API request failed: ${error.message}`,
      };
    }
  }

  /**
   * Verify that a request was made at least N times
   */
  async verifyAtLeast(
    pattern: VerificationRequest,
    min: number
  ): Promise<VerificationResult> {
    try {
      const response = await axios.post(
        `${this.url()}/api/verification/at-least`,
        {
          pattern: {
            method: pattern.method,
            path: pattern.path,
            query_params: pattern.queryParams || {},
            headers: pattern.headers || {},
            body_pattern: pattern.bodyPattern,
          },
          min,
        }
      );
      return response.data;
    } catch (error: any) {
      return {
        matched: false,
        count: 0,
        expected: { type: 'at_least', value: min },
        matches: [],
        errorMessage: `Verification API request failed: ${error.message}`,
      };
    }
  }

  /**
   * Verify that requests occurred in a specific sequence
   */
  async verifySequence(
    patterns: VerificationRequest[]
  ): Promise<VerificationResult> {
    try {
      const response = await axios.post(
        `${this.url()}/api/verification/sequence`,
        {
          patterns: patterns.map((p) => ({
            method: p.method,
            path: p.path,
            query_params: p.queryParams || {},
            headers: p.headers || {},
            body_pattern: p.bodyPattern,
          })),
        }
      );
      return response.data;
    } catch (error: any) {
      return {
        matched: false,
        count: 0,
        expected: { type: 'exactly', value: patterns.length },
        matches: [],
        errorMessage: `Verification API request failed: ${error.message}`,
      };
    }
  }

  /**
   * Get count of matching requests
   */
  async countRequests(pattern: VerificationRequest): Promise<number> {
    try {
      const response = await axios.post(
        `${this.url()}/api/verification/count`,
        {
          pattern: {
            method: pattern.method,
            path: pattern.path,
            query_params: pattern.queryParams || {},
            headers: pattern.headers || {},
            body_pattern: pattern.bodyPattern,
          },
        }
      );
      return response.data.count || 0;
    } catch {
      return 0;
    }
  }
}

// Touch MockServerErrorCode so consumers using isolatedModules don't tree-shake the enum out
void MockServerErrorCode;
