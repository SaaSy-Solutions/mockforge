# MockForge SDK - Known Limitations

This document outlines known limitations in the current SDK implementation and provides workarounds where applicable.

## 1. Port Discovery (Port 0 / Random Ports)

### Issue
When specifying port `0` to use a random available port, the SDKs cannot currently detect which port was actually assigned by the operating system.

### Affected SDKs
- ✅ **Rust SDK**: Not affected - uses in-process server
- ❌ **Node.js SDK**: Affected
- ❌ **Python SDK**: Affected
- ❌ **Go SDK**: Affected

### Impact
- Cannot use `port: 0` for automatic port assignment
- Tests must use explicit ports or risk port conflicts

### Workaround
```typescript
// Node.js - Use explicit port
const server = await MockServer.start({ port: 3000 });
```

```python
# Python - Use explicit port
with MockServer(port=3000) as server:
    ...
```

```go
// Go - Use explicit port
server := mockforge.NewMockServer(mockforge.MockServerConfig{Port: 3000})
```

### Solution Required
Parse MockForge CLI stdout to detect the actual bound port:
```
MockForge server listening on 127.0.0.1:54321  # Parse this
```

**Tracking Issue**: #TBD

---

## 2. Dynamic Stub Updates

### Issue
Stubs cannot be added or modified after the server has started.

### Affected SDKs
- ❌ **Rust SDK**: Router is built once at startup
- ⚠️  **Node.js SDK**: Admin API not integrated
- ⚠️  **Python SDK**: Admin API not integrated
- ⚠️  **Go SDK**: Admin API not integrated

### Impact
- All stubs must be defined before calling `start()`
- Cannot modify responses during test execution
- Cannot clear stubs without restarting

### Workaround
**Define all stubs before starting:**

```rust
// Rust - Add stubs before start
let mut server = MockServer::new()
    .port(3000)
    .start()
    .await?;

// This works - server hasn't started yet
server.stub_response("GET", "/api/users", json!({...})).await?;

// ❌ Cannot add more stubs after this
server.start().await?;
```

**Or restart the server:**

```python
# Python - Restart to change stubs
server.stop()
server = MockServer(port=3000)
server.stub_response('GET', '/new-endpoint', {...})
server.start()
```

### Solution Required
1. **Rust SDK**: Implement hot-reload mechanism or admin API client
2. **Other SDKs**: Integrate with MockForge admin API for runtime stub management

**Tracking Issue**: #TBD

---

## 3. Admin API Integration

### Issue
MockForge has an admin API for dynamic configuration, but the SDKs don't integrate with it yet.

### Affected SDKs
- ⚠️  **All SDKs**: Admin API exists but not used

### Impact
- Cannot dynamically add/remove stubs
- Cannot query server state
- Cannot inspect request history
- Cannot modify configuration at runtime

### Current Code
```typescript
// Node.js - Admin port is never discovered
this.adminPort = 0; // Always stays 0!

if (this.adminPort) {  // Never true
    await axios.post(`http://${this.host}:${this.adminPort}/api/stubs`, stub);
}
```

### Solution Required
1. Parse admin port from CLI output: `Admin API listening on port 54322`
2. Implement admin API client methods:
   - `addStub(stub)` - Add stub at runtime
   - `removeStub(id)` - Remove stub
   - `listStubs()` - Query current stubs
   - `getRequests()` - Inspect request history

**Tracking Issue**: #TBD

---

## 4. Integration Tests

### Issue
Integration tests require MockForge CLI to be installed and are currently skipped.

### Affected SDKs
- ✅ **Rust SDK**: Has integration tests (requires building)
- ⚠️  **Node.js SDK**: Tests created but marked `.skip()`
- ⚠️  **Python SDK**: Tests created but marked `@pytest.mark.skip`
- ⚠️  **Go SDK**: Tests created but require `-tags=integration`

### Impact
- Cannot verify end-to-end functionality automatically
- CI/CD pipeline cannot test SDKs without CLI setup
- Manual testing required

### Current Test Status
```typescript
// Node.js - Tests are skipped
describe.skip('Integration tests (require MockForge CLI)', () => {
    it('should start and stop server', async () => {
        // Test code...
    });
});
```

```python
# Python - Tests are skipped
@pytest.mark.skip(reason="Requires MockForge CLI to be installed")
class TestMockServerIntegration:
    def test_start_and_stop(self):
        ...
```

```go
// Go - Tests require integration tag
//go:build integration
func TestMockServerStart(t *testing.T) {
    t.Skip("Requires MockForge CLI to be installed")
    ...
}
```

### Solution Required
1. Set up CI/CD to install MockForge CLI
2. Enable integration tests in CI
3. Add more comprehensive test coverage
4. Consider mocking CLI for unit tests

**Tracking Issue**: #TBD

---

## 5. CLI Dependency

### Issue
Node.js, Python, and Go SDKs require MockForge CLI to be installed and in PATH.

### Affected SDKs
- ✅ **Rust SDK**: Not affected - native library
- ❌ **Node.js SDK**: Requires CLI
- ❌ **Python SDK**: Requires CLI
- ❌ **Go SDK**: Requires CLI

### Impact
- Extra installation step for users
- Potential version mismatch between SDK and CLI
- Cannot work in environments without CLI access

### Workaround
**Documented in README.md:**
```bash
# Install MockForge CLI
cargo install mockforge-cli

# Verify installation
mockforge --version
```

### Long-term Solution
Consider native FFI bindings to Rust library instead of spawning CLI process:
- Faster startup
- No CLI dependency
- Better error handling
- Type-safe interface

**Tracking Issue**: #TBD

---

## 6. Error Visibility

### Issue
When MockForge CLI fails to start or encounters errors, the error messages are not always visible to the SDK user.

### Affected SDKs
- ⚠️  **All SDKs**: Limited error propagation

### Impact
- Difficult to debug startup failures
- Silent failures in some cases
- Poor developer experience

### Example
```python
# Python - Health check fails silently
try:
    response = requests.get(f"http://{self.host}:{self.port}/health", timeout=0.1)
    if response.status_code == 200:
        return
except requests.exceptions.RequestException:
    await sleep(retryDelay)  # Just retry, no logging
```

### Solution Required
1. Capture stderr from CLI process
2. Log errors to console or return in exceptions
3. Provide verbose/debug modes
4. Better error messages for common failures

**Tracking Issue**: #TBD

---

## 7. Concurrent Server Instances

### Issue
No explicit handling of multiple MockServer instances in the same process.

### Affected SDKs
- ⚠️  **All SDKs**: Possible but not tested

### Impact
- Unknown behavior with multiple servers
- Potential port conflicts
- Resource management unclear

### Workaround
Use different ports for each server:

```python
server1 = MockServer(port=3000)
server2 = MockServer(port=3001)
server3 = MockServer(port=3002)
```

### Solution Required
1. Test concurrent usage
2. Document thread-safety guarantees
3. Add synchronization if needed (Go SDK)
4. Verify resource cleanup

**Tracking Issue**: #TBD

---

## Summary Table

| Limitation | Rust | Node.js | Python | Go | Severity | Workaround Available |
|------------|------|---------|--------|-----|----------|---------------------|
| Port Discovery | ✅ N/A | ❌ Yes | ❌ Yes | ❌ Yes | Medium | ✅ Use explicit ports |
| Dynamic Stubs | ❌ Yes | ⚠️ Partial | ⚠️ Partial | ⚠️ Partial | High | ✅ Define before start |
| Admin API | ⚠️ Not used | ⚠️ Not used | ⚠️ Not used | ⚠️ Not used | Medium | ❌ No workaround |
| Integration Tests | ✅ Has tests | ⚠️ Skipped | ⚠️ Skipped | ⚠️ Skipped | Low | ✅ Manual testing |
| CLI Dependency | ✅ N/A | ❌ Required | ❌ Required | ❌ Required | Medium | ✅ Install CLI |
| Error Visibility | ⚠️ Partial | ⚠️ Limited | ⚠️ Limited | ⚠️ Limited | Low | ❌ No workaround |
| Concurrent Use | ⚠️ Unknown | ⚠️ Unknown | ⚠️ Unknown | ⚠️ Unknown | Low | ✅ Use different ports |

---

## Recommendations

### For Production Use
1. **Always use explicit ports** - Don't rely on port 0
2. **Define all stubs before start** - No dynamic updates yet
3. **Install MockForge CLI** - Required for non-Rust SDKs
4. **Handle errors gracefully** - Check for CLI installation
5. **Use Rust SDK for best experience** - Most mature, no CLI needed

### For Contributors
1. **Port discovery is highest priority** - Enables port 0
2. **Admin API integration is second** - Enables dynamic stubs
3. **Integration tests third** - Improves reliability
4. **Error visibility improvements** - Better DX

---

*Last Updated: 2025-10-22*
*SDKVersion: 0.1.0*
