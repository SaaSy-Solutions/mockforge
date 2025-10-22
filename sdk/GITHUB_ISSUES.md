# GitHub Issues to Create

Copy each issue below into GitHub's "New Issue" form.

---

## Issue 1: SDK Port Discovery

**Title:** SDK: Implement automatic port discovery for random port assignment

**Labels:** `enhancement`, `sdk`, `high-priority`

**Milestone:** v0.2.0

**Description:**

Currently, SDKs cannot detect which port MockForge CLI actually bound to when using port 0 (random port). This forces developers to use explicit ports, which can cause conflicts in parallel test execution.

**Problem:**
- Port 0 (random port) doesn't work
- Tests must use explicit ports
- Risk of port conflicts in CI/CD

**Affected SDKs:**
- ✅ Rust SDK - Not affected (native implementation)
- ❌ Node.js SDK
- ❌ Python SDK
- ❌ Go SDK

**Solution:**
Parse MockForge CLI stdout to detect bound ports:
```
[INFO] MockForge HTTP server listening on 127.0.0.1:54321
[INFO] MockForge Admin API listening on 127.0.0.1:54322
```

**Acceptance Criteria:**
- [ ] Parse HTTP port from CLI output
- [ ] Parse admin API port from CLI output
- [ ] Update `port` property with actual value
- [ ] Update `adminPort` property with actual value
- [ ] Handle parsing failures gracefully
- [ ] Add tests for port discovery
- [ ] Update documentation

**References:**
- `sdk/KNOWN_LIMITATIONS.md` - Section 1
- `sdk/FOLLOW_UP_ISSUES.md` - Issue #1

---

## Issue 2: SDK Admin API Integration

**Title:** SDK: Enable runtime stub manipulation via Admin API

**Labels:** `enhancement`, `sdk`, `high-priority`

**Milestone:** v0.3.0

**Description:**

MockForge has an admin API for runtime configuration, but SDKs don't use it yet. This prevents adding/removing stubs after the server starts.

**Problem:**
- Cannot add stubs after server starts
- Cannot remove stubs dynamically
- No request history inspection
- Limited runtime control

**Depends On:** #[port-discovery-issue-number]

**Solution:**
Integrate with MockForge Admin API:
1. Discover admin API port (from Issue #1)
2. Implement admin API client methods
3. Enable dynamic stub operations

**API Methods to Implement:**
```typescript
async addStubDynamic(stub: ResponseStub): Promise<void>
async removeStub(stubId: string): Promise<void>
async listStubs(): Promise<ResponseStub[]>
async clearStubs(): Promise<void>
async getRequestHistory(): Promise<Request[]>
```

**Admin API Endpoints:**
- `POST /api/stubs` - Add stub
- `GET /api/stubs` - List stubs
- `DELETE /api/stubs/:id` - Remove stub
- `DELETE /api/stubs` - Clear all
- `GET /api/requests` - Request history

**Acceptance Criteria:**
- [ ] Discover admin API port
- [ ] Implement `addStubDynamic()`
- [ ] Implement `removeStub()`
- [ ] Implement `listStubs()`
- [ ] Implement `clearStubs()`
- [ ] Implement `getRequestHistory()`
- [ ] Add tests for all methods
- [ ] Update documentation

**References:**
- `sdk/KNOWN_LIMITATIONS.md` - Section 3
- `sdk/FOLLOW_UP_ISSUES.md` - Issue #2

---

## Issue 3: SDK CI/CD Integration Tests

**Title:** SDK: Set up CI pipeline for integration testing

**Labels:** `testing`, `sdk`, `ci-cd`, `medium-priority`

**Milestone:** v0.2.0

**Description:**

Integration tests are currently skipped because they require MockForge CLI. CI/CD should install the CLI and run full integration tests.

**Problem:**
- Integration tests are skipped
- No automated end-to-end testing
- Manual testing required
- Cannot verify SDK functionality in CI

**Solution:**
Set up CI/CD pipeline to:
1. Install MockForge CLI
2. Enable integration tests
3. Run tests for all SDKs
4. Report coverage

**Example GitHub Actions:**
```yaml
name: SDK Integration Tests

jobs:
  test-nodejs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install MockForge CLI
        run: cargo install --path crates/mockforge-cli
      - name: Run tests
        run: |
          cd sdk/nodejs
          npm install
          npm test
```

**Acceptance Criteria:**
- [ ] Add CI workflow for Node.js SDK
- [ ] Add CI workflow for Python SDK
- [ ] Add CI workflow for Go SDK
- [ ] Install MockForge CLI in CI
- [ ] Enable integration tests (remove `.skip()`)
- [ ] Add coverage reporting
- [ ] All tests pass in CI
- [ ] Update README with CI badges

**References:**
- `sdk/KNOWN_LIMITATIONS.md` - Section 4
- `sdk/FOLLOW_UP_ISSUES.md` - Issue #3

---

## Issue 4: Rust SDK Dynamic Stub Updates

**Title:** Rust SDK: Enable adding stubs after server starts

**Labels:** `enhancement`, `sdk`, `rust`, `medium-priority`

**Milestone:** v0.3.0

**Description:**

The Rust SDK builds the Axum router once at startup. New stubs added after `start()` don't take effect.

**Problem:**
- Stubs must be added before `start()`
- Cannot modify responses during tests
- Inflexible for dynamic testing scenarios

**Possible Solutions:**

**Option A:** Hot-reload with shared state
**Option B:** Admin API client (recommended - consistent with other SDKs)
**Option C:** Router hot-swap

**Recommended Approach:**
Use MockForge's admin API like other SDKs (depends on Issue #2).

**Acceptance Criteria:**
- [ ] Choose implementation approach
- [ ] Implement dynamic stub updates
- [ ] Add tests for runtime stub manipulation
- [ ] Verify thread safety
- [ ] Benchmark performance
- [ ] Update documentation

**References:**
- `sdk/KNOWN_LIMITATIONS.md` - Section 2
- `sdk/FOLLOW_UP_ISSUES.md` - Issue #4

---

## Issue 5: SDK Comprehensive Test Coverage

**Title:** SDK: Expand test coverage for all SDKs

**Labels:** `testing`, `sdk`, `medium-priority`

**Milestone:** v0.4.0

**Description:**

Current test coverage is minimal. Need comprehensive tests for all functionality.

**Current Status:**
- Rust: Integration tests only
- Node.js: Basic unit tests, integration skipped
- Python: Basic unit tests, integration skipped
- Go: Basic unit tests, integration tagged

**Test Categories Needed:**

**Unit Tests (No CLI):**
- Constructor/initialization
- Configuration validation
- URL generation
- State management
- Error handling
- Builder pattern

**Integration Tests (Require CLI):**
- Server start/stop
- Stub creation
- HTTP requests
- Multiple stubs
- Error responses
- Latency simulation
- Header manipulation
- Template expansion

**E2E Tests:**
- Real application scenarios
- Concurrent requests
- Multiple servers
- Error recovery

**Coverage Goals:**
- Unit tests: 80%+
- Integration tests: All public APIs
- E2E tests: Common use cases

**Acceptance Criteria:**
- [ ] Add unit tests for all SDKs
- [ ] Add integration tests for all SDKs
- [ ] Add E2E test examples
- [ ] Achieve 80%+ code coverage
- [ ] Add coverage reporting
- [ ] Document test patterns

**References:**
- `sdk/FOLLOW_UP_ISSUES.md` - Issue #5

---

## Issue 6: SDK Error Visibility

**Title:** SDK: Better error messages and debugging support

**Labels:** `enhancement`, `sdk`, `dx`, `low-priority`

**Milestone:** v0.2.0

**Description:**

Error messages are often silent or unclear. Need better error propagation and debugging tools.

**Problem:**
- CLI errors not visible
- Silent failures
- Difficult to debug startup issues
- Poor developer experience

**Solution:**
1. Capture CLI stderr
2. Add verbose/debug modes
3. Better error messages
4. Structured error types

**Improvements:**

**Error Logging:**
```typescript
// Capture stderr
this.process.stderr?.on('data', (data) => {
    if (this.debug) {
        console.error('[MockForge Error]', data.toString());
    }
});
```

**Debug Mode:**
```python
server = MockServer(port=3000, debug=True)
# Logs all CLI output, HTTP requests, stub matching
```

**Common Error Messages:**
- "MockForge CLI not found" → "Install with: cargo install mockforge-cli"
- "Port in use" → "Try different port or use port: 0"
- "Health check timeout" → "Server failed to start. Check logs."

**Acceptance Criteria:**
- [ ] Capture CLI stderr in all SDKs
- [ ] Add debug/verbose mode
- [ ] Improve error messages
- [ ] Add troubleshooting guide
- [ ] Test error scenarios

**References:**
- `sdk/KNOWN_LIMITATIONS.md` - Section 6
- `sdk/FOLLOW_UP_ISSUES.md` - Issue #6

---

## Issue 7: Research Native FFI Bindings

**Title:** SDK: Explore native Rust FFI instead of CLI spawning

**Labels:** `research`, `sdk`, `future`, `low-priority`

**Milestone:** Future / v1.0.0

**Description:**

Current SDKs spawn MockForge CLI as a subprocess. Native FFI bindings to the Rust library would be faster and more reliable.

**Benefits:**
- No CLI installation required
- Faster startup (no process spawn)
- Better error handling
- Type-safe interface
- Lower resource usage

**Challenges:**
- Complex FFI implementation
- Memory management across languages
- Async/threading complexities
- Platform-specific builds
- Distribution complexity

**Research Tasks:**
- [ ] Benchmark CLI vs FFI performance
- [ ] Investigate N-API/neon for Node.js
- [ ] Investigate PyO3 for Python
- [ ] Investigate cgo for Go
- [ ] Assess maintenance burden
- [ ] Evaluate user experience impact
- [ ] Consider hybrid approach
- [ ] Create proof of concept

**Decision Criteria:**
- Performance improvement > 2x
- Maintenance burden acceptable
- Distribution complexity manageable
- Clear user experience benefit

**References:**
- `sdk/FOLLOW_UP_ISSUES.md` - Issue #7

---

## Summary

**v0.2.0 Milestone** (Essential):
- Issue #1: Port Discovery
- Issue #3: CI/CD Tests
- Issue #6: Error Visibility

**v0.3.0 Milestone** (Dynamic Features):
- Issue #2: Admin API
- Issue #4: Rust Hot-Reload

**v0.4.0 Milestone** (Quality):
- Issue #5: Test Coverage

**Future** (Research):
- Issue #7: FFI Bindings
