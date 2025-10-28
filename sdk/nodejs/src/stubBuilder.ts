import { ResponseStub } from './types';

/**
 * Fluent builder for creating response stubs
 */
export class StubBuilder {
  private stub: Partial<ResponseStub>;

  constructor(method: string, path: string) {
    this.stub = {
      method: method.toUpperCase(),
      path,
      status: 200,
      headers: {},
    };
  }

  /**
   * Set the response status code
   */
  status(code: number): this {
    this.stub.status = code;
    return this;
  }

  /**
   * Set a response header
   */
  header(key: string, value: string): this {
    if (!this.stub.headers) {
      this.stub.headers = {};
    }
    this.stub.headers[key] = value;
    return this;
  }

  /**
   * Set multiple response headers
   */
  headers(headers: Record<string, string>): this {
    this.stub.headers = { ...this.stub.headers, ...headers };
    return this;
  }

  /**
   * Set the response body
   */
  body(body: any): this {
    this.stub.body = body;
    return this;
  }

  /**
   * Set response latency in milliseconds
   */
  latency(ms: number): this {
    this.stub.latencyMs = ms;
    return this;
  }

  /**
   * Build the response stub
   */
  build(): ResponseStub {
    if (!this.stub.body) {
      throw new Error('Response body is required');
    }
    return this.stub as ResponseStub;
  }
}
