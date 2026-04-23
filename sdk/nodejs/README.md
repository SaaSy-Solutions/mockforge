# @mockforge-dev/sdk

Embed [MockForge](https://github.com/SaaSy-Solutions/mockforge) mock servers directly in your Node.js / TypeScript tests. Start on a random port, register stubs at runtime, and tear the server down when you're done.

> Requires the `mockforge` CLI (≥ 0.3.116) on your PATH. The SDK spawns it as a child process.

## Install

```bash
npm install --save-dev @mockforge-dev/sdk
```

Install the CLI once per machine:

```bash
# Rust toolchain present?
cargo install mockforge-cli

# Otherwise grab a pre-built release:
# https://github.com/SaaSy-Solutions/mockforge/releases
```

Verify with `mockforge --version`.

## Usage

```ts
import { MockServer } from '@mockforge-dev/sdk';

describe('User API', () => {
  let server: MockServer;

  beforeEach(async () => {
    server = await MockServer.start({ port: 0 }); // 0 = random available port
  });

  afterEach(async () => {
    await server.stop();
  });

  it('returns the stubbed user', async () => {
    await server.stubResponse('GET', '/api/users/123', {
      id: 123,
      name: 'John Doe',
    });

    const res = await fetch(`${server.url()}/api/users/123`);
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ id: 123, name: 'John Doe' });
  });
});
```

### Custom status and headers

```ts
await server.stubResponse(
  'POST',
  '/api/widgets',
  { ok: true },
  { status: 201, headers: { 'X-Source': 'sdk-test' }, latencyMs: 50 }
);
```

### Against an OpenAPI spec

```ts
const server = await MockServer.start({
  port: 0,
  openApiSpec: './openapi.yaml',
});
```

## Configuration

All fields on `MockServerConfig` are optional.

| Field              | Default       | Description                                                                                         |
| ------------------ | ------------- | --------------------------------------------------------------------------------------------------- |
| `port`             | `0`           | HTTP port. `0` = random available port.                                                             |
| `host`             | `127.0.0.1`   | Host to bind to.                                                                                    |
| `adminPort`        | _disabled_    | If set, also starts the admin UI on this port. Rarely needed for tests.                             |
| `wsPort`           | `0`           | WebSocket port. `0` = random.                                                                       |
| `grpcPort`         | `0`           | gRPC port. `0` disables gRPC entirely.                                                              |
| `metricsPort`      | `0`           | Prometheus metrics port. Metrics are off unless the CLI-level `--metrics` flag is set.              |
| `configFile`       | _unset_       | Path to a MockForge config file.                                                                    |
| `openApiSpec`      | _unset_       | Path to an OpenAPI spec file.                                                                       |
| `noConfig`         | `true`        | Skip auto-discovery of `mockforge.yaml` in the cwd / ancestors. Avoids accidental config inheritance. |
| `startupTimeoutMs` | `12_000`      | Timeout for the CLI to bind its ports and report ready. Bump on slow CI.                            |

## Instance methods

| Method                                       | Description                                           |
| -------------------------------------------- | ----------------------------------------------------- |
| `stubResponse(method, path, body, options?)` | Register a response stub at runtime.                  |
| `updateStub(method, path, body, options?)`   | Replace an existing stub (creates one if missing).    |
| `removeStub(method, path)`                   | Remove a single stub.                                 |
| `clearStubs()`                               | Remove every stub.                                    |
| `url()`                                      | Base URL (e.g. `http://127.0.0.1:38381`).             |
| `getPort()`                                  | Bound HTTP port.                                      |
| `getAdminPort()`                             | Bound admin UI port (`0` if not started).             |
| `isRunning()`                                | Whether the CLI subprocess is still alive.            |
| `stop()`                                     | Terminate the subprocess.                             |
| `verify(pattern, count)`                     | Assert request count matches pattern + expectation.   |
| `verifyNever(pattern)`                       | Assert no request matched the pattern.                |
| `verifyAtLeast(pattern, n)`                  | Assert at least `n` requests matched.                 |
| `verifySequence(patterns[])`                 | Assert requests occurred in the given order.          |
| `countRequests(pattern)`                     | Return the number of requests matching the pattern.   |

## How stubs work

Stubs go through the MockForge HTTP server's management API at `http://<host>:<httpPort>/__mockforge/api/mocks`. When a request comes in that doesn't match any OpenAPI route, MockForge checks its dynamic mock table and serves the first match by descending priority. This means:

- Stubs registered via `stubResponse` take effect immediately — no restart.
- Stubs are **per-server**; they live in memory and disappear with `stop()`.
- If you also load an OpenAPI spec, explicit routes defined by the spec win over dynamic stubs.

## Errors

All failures throw a `MockServerError` with a `code` from `MockServerErrorCode`:

```ts
import { MockServerError, MockServerErrorCode } from '@mockforge-dev/sdk';

try {
  server = await MockServer.start({ port: 0 });
} catch (e) {
  if (e instanceof MockServerError && e.code === MockServerErrorCode.CLI_NOT_FOUND) {
    console.error('Install mockforge: cargo install mockforge-cli');
  }
  throw e;
}
```

## Requirements

- Node.js ≥ 18 (for built-in `fetch`)
- `mockforge` CLI ≥ 0.3.116 on PATH

## License

MIT
