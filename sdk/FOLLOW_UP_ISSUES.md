# MockForge SDK - Follow-up Issues

These GitHub issues should be created after the initial SDK commit to track remaining work.

---

## Issue #1: Implement Port Discovery from CLI Output

**Title**: SDK: Implement automatic port discovery for random port assignment

**Priority**: High
**Complexity**: Medium
**Affects**: Node.js SDK, Python SDK, Go SDK

### Description
Currently, SDKs cannot detect which port MockForge CLI actually bound to when using port 0 (random port). This forces developers to use explicit ports, which can cause conflicts in parallel test execution.

### Requirements
1. Parse MockForge CLI stdout to detect bound ports
2. Update `this.port` and `this.adminPort` with actual values
3. Handle parsing failures gracefully
4. Support both HTTP and admin API port discovery

### Example CLI Output to Parse
```
[INFO] MockForge HTTP server listening on 127.0.0.1:54321
[INFO] MockForge Admin API listening on 127.0.0.1:54322
```

### Implementation Hints
**Node.js:**
```typescript
private parsePortFromOutput(data: string): void {
    const httpMatch = data.match(/HTTP server listening on .*:(\d+)/);
    if (httpMatch) {
        this.port = parseInt(httpMatch[1]);
    }
    const adminMatch = data.match(/Admin API listening on .*:(\d+)/);
    if (adminMatch) {
        this.adminPort = parseInt(adminMatch[1]);
    }
}
```

### Test Cases
- [ ] Port 0 assigns random port correctly
- [ ] Explicit port is used when specified
- [ ] Admin port is discovered
- [ ] Parsing failures don't crash
- [ ] Multiple servers can coexist

### Files to Modify
- `sdk/nodejs/src/mockServer.ts`
- `sdk/python/mockforge_sdk/mock_server.py`
- `sdk/go/mockserver.go`

---

## Issue #2: Integrate Admin API for Dynamic Stub Management

**Title**: SDK: Enable runtime stub manipulation via Admin API

**Priority**: High
**Complexity**: High
**Affects**: All SDKs

### Description
MockForge has an admin API for runtime configuration, but SDKs don't use it yet. This prevents adding/removing stubs after the server starts.

### Requirements
1. Discover admin API port (depends on Issue #1)
2. Implement admin API client methods
3. Enable dynamic stub operations
4. Add request history inspection
5. Support configuration updates

### API Methods to Implement
```typescript
// Add these methods to MockServer
async addStubDynamic(stub: ResponseStub): Promise<void>
async removeStub(stubId: string): Promise<void>
async listStubs(): Promise<ResponseStub[]>
async clearStubs(): Promise<void>
async getRequestHistory(): Promise<Request[]>
async getMetrics(): Promise<Metrics>
```

### Admin API Endpoints
- `POST /api/stubs` - Add stub
- `GET /api/stubs` - List stubs
- `DELETE /api/stubs/:id` - Remove stub
- `DELETE /api/stubs` - Clear all
- `GET /api/requests` - Request history
- `GET /api/metrics` - Server metrics

### Test Cases
- [ ] Add stub after server starts
- [ ] Remove individual stub
- [ ] Clear all stubs
- [ ] Query request history
- [ ] Handle admin API errors
- [ ] Verify stub is actually active

### Files to Modify
- All SDK server implementation files
- Add new admin API client modules

---

## Issue #3: Enable Integration Tests in CI/CD

**Title**: SDK: Set up CI pipeline for integration testing

**Priority**: Medium
**Complexity**: Medium
**Affects**: CI/CD Pipeline

### Description
Integration tests are currently skipped because they require MockForge CLI. CI/CD should install the CLI and run full integration tests.

### Requirements
1. Install MockForge CLI in CI environment
2. Enable integration tests
3. Run tests for all SDKs
4. Report coverage
5. Fail build on test failures

### CI Configuration (GitHub Actions Example)
```yaml
name: SDK Integration Tests

on: [push, pull_request]

jobs:
  test-nodejs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install Rust
        uses: actions-rs/toolchain@v1
      - name: Install MockForge CLI
        run: cargo install --path crates/mockforge-cli
      - name: Run Node.js tests
        run: |
          cd sdk/nodejs
          npm install
          npm test

  test-python:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install MockForge CLI
        run: cargo install --path crates/mockforge-cli
      - name: Run Python tests
        run: |
          cd sdk/python
          pip install -e .
          pytest

  test-go:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install MockForge CLI
        run: cargo install --path crates/mockforge-cli
      - name: Run Go tests
        run: |
          cd sdk/go
          go test -v -tags=integration
```

### Test Cases
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Tests run in parallel
- [ ] Coverage reports generated
- [ ] Failures are clearly reported

---

## Issue #4: Implement Dynamic Router Reloading for Rust SDK

**Title**: Rust SDK: Enable adding stubs after server starts

**Priority**: Medium
**Complexity**: High
**Affects**: Rust SDK

### Description
The Rust SDK builds the Axum router once at startup. New stubs added after `start()` don't take effect.

### Possible Solutions

#### Option A: Hot-Reload with State
```rust
pub struct MockServer {
    stubs: Arc<RwLock<Vec<ResponseStub>>>,
    // ... other fields
}

// Router uses state to lookup stubs dynamically
async fn handle_request(
    State(stubs): State<Arc<RwLock<Vec<ResponseStub>>>>,
    req: Request<Body>
) -> Response {
    let stubs = stubs.read().await;
    // Match request against stubs...
}
```

#### Option B: Admin API Client
Use MockForge's own admin API like other SDKs.

#### Option C: Router Hot-Swap
Rebuild router and swap it atomically (complex).

### Recommended Approach
**Option B** (Admin API Client) - Consistent with other SDKs, leverages existing infrastructure.

### Test Cases
- [ ] Add stub after start works
- [ ] Remove stub works
- [ ] Clear stubs works
- [ ] Concurrent stub updates safe
- [ ] Performance acceptable

---

## Issue #5: Add Comprehensive Test Suites

**Title**: SDK: Expand test coverage for all SDKs

**Priority**: Medium
**Complexity**: Medium
**Affects**: All SDKs

### Description
Current test coverage is minimal. Need comprehensive tests for all functionality.

### Test Categories Needed

#### Unit Tests (No CLI Required)
- [ ] Constructor/initialization
- [ ] Configuration validation
- [ ] URL generation
- [ ] State management
- [ ] Error handling
- [ ] Builder pattern

#### Integration Tests (Require CLI)
- [ ] Server start/stop
- [ ] Stub creation
- [ ] HTTP requests
- [ ] Multiple stubs
- [ ] Error responses
- [ ] Latency simulation
- [ ] Header manipulation
- [ ] Template expansion

#### E2E Tests
- [ ] Real application scenarios
- [ ] Concurrent requests
- [ ] Multiple servers
- [ ] Error recovery
- [ ] Resource cleanup

### Coverage Goals
- Unit tests: 80%+
- Integration tests: All public APIs
- E2E tests: Common use cases

### Files to Create/Modify
- `sdk/nodejs/src/**/*.test.ts`
- `sdk/python/tests/test_*.py`
- `sdk/go/*_test.go`
- `crates/mockforge-sdk/tests/*.rs`

---

## Issue #6: Improve Error Visibility and Debugging

**Title**: SDK: Better error messages and debugging support

**Priority**: Low
**Complexity**: Low
**Affects**: All SDKs

### Description
Error messages are often silent or unclear. Need better error propagation and debugging tools.

### Requirements
1. Capture and log CLI stderr
2. Provide verbose/debug modes
3. Better error messages for common failures
4. Structured error types
5. Debugging helpers

### Improvements Needed

#### Error Logging
```typescript
// Node.js - Capture stderr
this.process.stderr?.on('data', (data) => {
    if (this.debug) {
        console.error('[MockForge Error]', data.toString());
    }
    this.lastError = data.toString();
});
```

#### Debug Mode
```python
# Python - Add debug flag
server = MockServer(port=3000, debug=True)
# Logs all CLI output, HTTP requests, stub matching, etc.
```

#### Common Error Messages
- "MockForge CLI not found in PATH" → "Install with: cargo install mockforge-cli"
- "Port 3000 already in use" → "Try a different port or use port: 0"
- "Health check timeout" → "Server failed to start. Check logs for details."

### Test Cases
- [ ] CLI not found shows helpful message
- [ ] Port conflict shows clear error
- [ ] Startup timeout shows debugging info
- [ ] Debug mode logs comprehensively

---

## Issue #7: Consider Native FFI Bindings

**Title**: SDK: Explore native Rust FFI instead of CLI spawning

**Priority**: Low
**Complexity**: Very High
**Affects**: Node.js SDK, Python SDK, Go SDK

### Description
Current SDKs spawn MockForge CLI as a subprocess. Native FFI bindings to the Rust library would be faster, more reliable, and remove CLI dependency.

### Benefits
- No CLI installation required
- Faster startup (no process spawn)
- Better error handling
- Type-safe interface
- Lower resource usage

### Challenges
- Complex FFI implementation
- Memory management across languages
- Async/threading complexities
- Platform-specific builds
- Distribution complexity

### Research Needed
- **Node.js**: Investigate N-API / neon bindings
- **Python**: Investigate PyO3
- **Go**: Investigate cgo

### Considerations
- May not be worth the complexity
- CLI approach is simpler to maintain
- FFI adds build-time dependencies
- Current approach works acceptably

### Decision Points
- [ ] Benchmark CLI vs FFI performance
- [ ] Assess maintenance burden
- [ ] Evaluate user experience impact
- [ ] Consider hybrid approach

---

## Priority Matrix

| Issue | Priority | Complexity | Impact | Effort | Recommended Order |
|-------|----------|------------|--------|--------|-------------------|
| #1 Port Discovery | High | Medium | High | Medium | 1st |
| #2 Admin API | High | High | High | High | 2nd |
| #3 CI/CD Tests | Medium | Medium | Medium | Medium | 3rd |
| #4 Rust Hot-Reload | Medium | High | Medium | High | 4th |
| #5 Test Coverage | Medium | Medium | High | High | 5th |
| #6 Error Visibility | Low | Low | Medium | Low | 6th |
| #7 FFI Bindings | Low | Very High | Low | Very High | Future |

---

## Milestone Planning

### v0.2.0 - Essential Fixes
- Issue #1: Port Discovery
- Issue #3: CI/CD Tests (partial)
- Issue #6: Error Visibility

### v0.3.0 - Dynamic Features
- Issue #2: Admin API Integration
- Issue #4: Rust Hot-Reload
- Issue #3: CI/CD Tests (complete)

### v0.4.0 - Quality & Coverage
- Issue #5: Comprehensive Tests
- Performance optimizations
- Documentation improvements

### v1.0.0 - Production Ready
- All known issues resolved
- Full test coverage
- Production deployments validated
- Issue #7: FFI Bindings (consideration)

---

*Generated: 2025-10-22*
*For: MockForge SDK v0.1.0*
