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
  private stubs: ResponseStub[] = [];
  // Track stub IDs returned from Admin API for later updates/deletes
  private stubIds: Map<string, string> = new Map(); // key: "METHOD:path", value: mock ID

  constructor(private config: MockServerConfig = {}) {
    this.port = config.port || 0;
    this.host = config.host || '127.0.0.1';
    this.adminPort = 0; // Will be set during start
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
    }

    if (this.config.openApiSpec) {
      args.push('--spec', this.config.openApiSpec);
    }

    if (this.port) {
      args.push('--http-port', this.port.toString());
    } else {
      // Use port 0 to let OS assign a random port
      args.push('--http-port', '0');
    }

    // Enable admin API for dynamic stub management
    args.push('--admin', '--admin-port', '0');

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

    // Collect stderr for error messages
    if (this.process.stderr) {
      this.process.stderr.on('data', (data: Buffer) => {
        const errorMsg = data.toString();
        // Log errors but don't fail immediately - wait for health check
        console.warn('MockForge stderr:', errorMsg);
      });
    }

    // Wait for server to start
    // Check if process failed to start
    if (processStartError) {
      throw MockServerError.cliNotFound(processStartError);
    }
    await this.waitForServer();
  }

  /**
   * Parse port numbers from MockForge CLI output
   * Looks for patterns like:
   * - "📡 HTTP server listening on http://localhost:3000"
   * - "🎛️ Admin UI listening on http://127.0.0.1:9080"
   */
  private parsePortsFromOutput(output: string): void {
    // Parse HTTP server port
    // Pattern: "📡 HTTP server listening on http://localhost:PORT" or "📡 HTTP server on port PORT"
    const httpPortMatch = output.match(/HTTP server (?:listening on http:\/\/[^:]+:|on port )(\d+)/);
    if (httpPortMatch && this.port === 0) {
      const detectedPort = parseInt(httpPortMatch[1], 10);
      if (detectedPort > 0) {
        this.port = detectedPort;
      }
    }

    // Parse Admin UI port
    // Pattern: "🎛️ Admin UI listening on http://HOST:PORT" or "🎛️ Admin UI on port PORT"
    const adminPortMatch = output.match(/Admin UI (?:listening on http:\/\/[^:]+:|on port )(\d+)/);
    if (adminPortMatch && this.adminPort === 0) {
      const detectedAdminPort = parseInt(adminPortMatch[1], 10);
      if (detectedAdminPort > 0) {
        this.adminPort = detectedAdminPort;
      }
    }
  }

  /**
   * Wait for the server to be ready
   */
  private async waitForServer(): Promise<void> {
    const maxRetries = 60; // Increased to allow time for port detection
    const retryDelay = 200;

    // If port is 0, we need to wait for port detection from stdout
    // Give it a few attempts to parse the port
    let portDetected = this.port !== 0;
    let portDetectionAttempts = 0;
    const maxPortDetectionAttempts = 10;

    for (let i = 0; i < maxRetries; i++) {
      // If port is 0, wait for it to be detected from stdout
      if (!portDetected && portDetectionAttempts < maxPortDetectionAttempts) {
        portDetectionAttempts++;
        await sleep(retryDelay);
        portDetected = this.port !== 0;
        continue;
      }

      // If port is still 0 after detection attempts, throw standardized error
      if (this.port === 0) {
        throw MockServerError.portDetectionFailed();
      }

      try {
        // Try to connect to health endpoint
        await axios.get(`http://${this.host}:${this.port}/health`, {
          timeout: 200,
        });
        return;
      } catch (error) {
        // If it's a connection error, continue retrying
        // If it's a timeout, the server might still be starting
        await sleep(retryDelay);
      }
    }

    throw MockServerError.healthCheckTimeout(
      maxRetries * retryDelay,
      this.port
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
    const stub: ResponseStub = {
      method: method.toUpperCase(),
      path,
      status: options.status || 200,
      headers: options.headers || {},
      body,
      latencyMs: options.latencyMs,
    };

    this.stubs.push(stub);

    // If admin API is available, use it to add the stub dynamically
    if (this.adminPort) {
      try {
        // Convert ResponseStub to MockConfig format expected by Admin API
        const mockConfig = {
          id: '', // Empty ID - server will generate one
          name: `${method.toUpperCase()} ${path}`, // Generate a name from method and path
          method: stub.method,
          path: stub.path,
          response: {
            body: stub.body,
            headers: stub.headers && Object.keys(stub.headers).length > 0 ? stub.headers : undefined,
          },
          enabled: true,
          latency_ms: stub.latencyMs || undefined,
          status_code: stub.status !== 200 ? stub.status : undefined,
        };

        const response = await axios.post(`http://${this.host}:${this.adminPort}/__mockforge/api/mocks`, mockConfig);
        // Store the mock ID for later updates/deletes
        const stubKey = `${stub.method}:${stub.path}`;
        if (response.data && response.data.id) {
          this.stubIds.set(stubKey, response.data.id);
        }
      } catch (error) {
        console.warn('Failed to add stub via admin API:', error);
      }
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
    const stubKey = `${method.toUpperCase()}:${path}`;
    const mockId = this.stubIds.get(stubKey);

    if (!mockId && this.adminPort) {
      // Try to find the mock by querying the API
      try {
        const response = await axios.get(`http://${this.host}:${this.adminPort}/__mockforge/api/mocks`);
        const mocks = response.data.mocks || [];
        const mock = mocks.find((m: any) =>
          m.method.toUpperCase() === method.toUpperCase() && m.path === path
        );
        if (mock) {
          this.stubIds.set(stubKey, mock.id);
          // Continue with update below
        } else {
          // Mock not found, create it instead
          return this.stubResponse(method, path, body, options);
        }
      } catch (error) {
        console.warn('Failed to find stub for update:', error);
        return;
      }
    }

    if (this.adminPort && mockId) {
      try {
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
            headers: stub.headers && Object.keys(stub.headers).length > 0 ? stub.headers : undefined,
          },
          enabled: true,
          latency_ms: stub.latencyMs || undefined,
          status_code: stub.status !== 200 ? stub.status : undefined,
        };

        await axios.put(`http://${this.host}:${this.adminPort}/__mockforge/api/mocks/${mockId}`, mockConfig);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        console.warn('Failed to update stub via admin API:', errorMessage);
      }
    }
  }

  /**
   * Remove a specific stub
   */
  async removeStub(method: string, path: string): Promise<void> {
    const stubKey = `${method.toUpperCase()}:${path}`;
    const mockId = this.stubIds.get(stubKey);

    // Remove from local cache
    this.stubs = this.stubs.filter(s =>
      !(s.method.toUpperCase() === method.toUpperCase() && s.path === path)
    );
    this.stubIds.delete(stubKey);

    if (this.adminPort && mockId) {
      try {
        await axios.delete(`http://${this.host}:${this.adminPort}/__mockforge/api/mocks/${mockId}`);
      } catch (error) {
        // If delete fails, try to find and delete by querying
        try {
          const response = await axios.get(`http://${this.host}:${this.adminPort}/__mockforge/api/mocks`);
          const mocks = response.data.mocks || [];
          const mock = mocks.find((m: any) =>
            m.method.toUpperCase() === method.toUpperCase() && m.path === path
          );
          if (mock) {
            await axios.delete(`http://${this.host}:${this.adminPort}/__mockforge/api/mocks/${mock.id}`);
          }
        } catch (findError) {
          const errorMessage = findError instanceof Error ? findError.message : String(findError);
          console.warn('Failed to remove stub via admin API:', errorMessage);
        }
      }
    }
  }

  /**
   * Clear all stubs
   */
  async clearStubs(): Promise<void> {
    this.stubs = [];
    this.stubIds.clear();

    if (this.adminPort) {
      try {
        // Get all mocks and delete them one by one
        const response = await axios.get(`http://${this.host}:${this.adminPort}/__mockforge/api/mocks`);
        const mocks = response.data.mocks || [];

        // Delete each mock
        await Promise.all(
          mocks.map((mock: { id: string }) =>
            axios.delete(`http://${this.host}:${this.adminPort}/__mockforge/api/mocks/${mock.id}`)
              .catch(() => {}) // Ignore individual delete errors
          )
        );
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        console.warn('Failed to clear stubs via admin API:', errorMessage);
      }
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

      // Wait for process to exit
      await new Promise<void>((resolve) => {
        if (this.process) {
          this.process.on('exit', () => resolve());
          setTimeout(() => resolve(), 1000); // Fallback timeout
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
    } catch (error) {
      return 0;
    }
  }
}
