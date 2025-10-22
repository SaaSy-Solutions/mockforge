import { spawn, ChildProcess } from 'child_process';
import { promisify } from 'util';
import axios from 'axios';
import { MockServerConfig, ResponseStub, StubOptions } from './types';

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
    }

    // Enable admin API for dynamic stub management
    args.push('--admin', '--admin-port', '0');

    this.process = spawn('mockforge', args, {
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    // Wait for server to start
    await this.waitForServer();
  }

  /**
   * Wait for the server to be ready
   */
  private async waitForServer(): Promise<void> {
    const maxRetries = 30;
    const retryDelay = 100;

    for (let i = 0; i < maxRetries; i++) {
      try {
        if (this.process && this.process.stdout) {
          // Check if we can detect the port from stdout
          // This is a simplified version; actual implementation would parse logs
        }

        // Try to connect to health endpoint
        await axios.get(`http://${this.host}:${this.port}/health`, {
          timeout: 100,
        });
        return;
      } catch (error) {
        await sleep(retryDelay);
      }
    }

    throw new Error('Failed to start MockForge server');
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
        await axios.post(`http://${this.host}:${this.adminPort}/api/stubs`, stub);
      } catch (error) {
        console.warn('Failed to add stub via admin API:', error);
      }
    }
  }

  /**
   * Clear all stubs
   */
  async clearStubs(): Promise<void> {
    this.stubs = [];

    if (this.adminPort) {
      try {
        await axios.delete(`http://${this.host}:${this.adminPort}/api/stubs`);
      } catch (error) {
        console.warn('Failed to clear stubs via admin API:', error);
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
}
