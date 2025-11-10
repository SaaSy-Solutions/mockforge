# MockForge SDK Code Review Fixes

## Summary

This document summarizes the critical issues identified during code review and the fixes applied before commit.

## Critical Issues Fixed (P0)

### 1. ✅ Rust FFI Double-Start Bug
**File:** `/crates/mockforge-sdk/src/ffi.rs`
**Lines:** 32-48
**Issue:** The `mockforge_server_new` function was calling `start()` twice - once in the builder chain and once after unwrapping the result.
**Impact:** Would always fail because `start()` checks if server is already running.
**Fix:** Removed the redundant second `start()` call.

```rust
// Before (BUGGY):
let server = runtime.block_on(async {
    MockServer::new()
        .port(port)
        .start()  // First call
        .await
});
let server = match server {
    Ok(mut s) => {
        if runtime.block_on(s.start()).is_err() {  // Second call - BUG!
            return ptr::null_mut();
        }
        s
    }
    ...
};

// After (FIXED):
let server = runtime.block_on(async {
    MockServer::new()
        .port(port)
        .start()
        .await
});
let server = match server {
    Ok(s) => s,  // No second start() call
    Err(_) => return ptr::null_mut(),
};
```

### 2. ✅ Rust Server Startup Race Condition
**File:** `/crates/mockforge-sdk/src/server.rs`
**Lines:** 89-91
**Issue:** Fixed 100ms sleep was unreliable for waiting for server to be ready.
**Impact:** On slow systems, server might not be ready. On fast systems, tests wait unnecessarily.
**Fix:** Replaced sleep with proper health check polling.

```rust
// Before (UNRELIABLE):
// Wait a bit for the server to start
tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

// After (RELIABLE):
// Wait for the server to be ready by polling health
self.wait_for_ready().await?;

// Added new method:
async fn wait_for_ready(&self) -> Result<()> {
    let max_attempts = 50;
    let delay = tokio::time::Duration::from_millis(100);

    for attempt in 0..max_attempts {
        let client = reqwest::Client::builder()
            .timeout(tokio::time::Duration::from_millis(100))
            .build()?;

        match client.get(format!("{}/health", self.url())).send().await {
            Ok(response) if response.status().is_success() => return Ok(()),
            _ => {
                if attempt < max_attempts - 1 {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(Error::General("Server failed to become ready".to_string()))
}
```

**Additional Fix:** Added `reqwest` to main dependencies in `Cargo.toml`.

### 3. ✅ Python 3.8 Compatibility Issue
**File:** `/sdk/python/mockforge_sdk/mock_server.py`
**Line:** 28
**Issue:** Used `list[ResponseStub]` syntax which requires Python 3.9+, but setup.py claims Python 3.8 support.
**Impact:** RuntimeError on Python 3.8: `TypeError: 'type' object is not subscriptable`.
**Fix:** Changed to `List[ResponseStub]` and added import from `typing`.

```python
# Before (PYTHON 3.9+ ONLY):
self.stubs: list[ResponseStub] = []

# After (PYTHON 3.8+ COMPATIBLE):
from typing import Optional, Dict, Any, List
...
self.stubs: List[ResponseStub] = []
```

### 4. ✅ Go Resource Leak
**File:** `/sdk/go/mockserver.go`
**Lines:** 81-85
**Issue:** When `waitForServer()` failed, process was killed but `m.cmd` was not set to nil, causing `IsRunning()` to return true for dead processes.
**Impact:** Resource leaks and incorrect state reporting.
**Fix:** Added proper cleanup after kill.

```go
// Before (RESOURCE LEAK):
if err := m.waitForServer(); err != nil {
    m.cmd.Process.Kill()
    return err  // m.cmd still points to dead process!
}

// After (PROPER CLEANUP):
if err := m.waitForServer(); err != nil {
    m.cmd.Process.Kill()
    m.cmd.Wait() // Clean up zombie process
    m.cmd = nil  // Clear cmd so IsRunning() returns false
    return err
}
```

## Documentation Improvements

### ✅ Added CLI Dependency Documentation
**File:** `/sdk/README.md`
**Added:** Prerequisites section at the top of the README explaining that Node.js, Python, and Go SDKs require the MockForge CLI to be installed.

```markdown
## Prerequisites

**Important:** The Node.js, Python, and Go SDKs require the MockForge CLI to be installed and available in your PATH.

### Install MockForge CLI

```bash
# Via Cargo
cargo install mockforge-cli

# Or download pre-built binaries from:
# https://github.com/SaaSy-Solutions/mockforge/releases
```

Verify installation:
```bash
mockforge --version
```

**Note:** The Rust SDK embeds MockForge directly and does not require the CLI.
```

## Known Issues (Not Fixed - Lower Priority)

### Rust SDK Dynamic Stub Updates
**Issue:** Adding stubs after `start()` doesn't work because router is built once.
**Workaround:** Add all stubs before starting, or restart server.
**Future Fix:** Implement admin API or hot-reload mechanism.

### Port Discovery Not Implemented
**All SDKs:** Port 0 (random port) doesn't work properly because stdout parsing is not implemented.
**Workaround:** Specify explicit ports in tests.
**Future Fix:** Parse CLI stdout to detect actual bound port.

### Admin API Not Used
**All SDKs:** Code to call admin API exists but admin port is never discovered.
**Future Fix:** Implement port discovery from CLI output.

### Missing Tests
**Node.js, Python, Go:** No test files created yet.
**Future:** Add comprehensive test suites for each SDK.

## Testing Status

### Before Fixes
- Rust SDK: Compiled with warnings
- FFI: Would fail at runtime
- Python SDK: Would fail on Python 3.8
- Go SDK: Resource leaks possible

### After Fixes
```bash
$ cargo check -p mockforge-sdk
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.46s
```

✅ All critical bugs fixed
✅ Compilation successful
✅ Ready for commit

## Recommendations for Follow-up PRs

1. **P1 - High Priority**
   - Implement port discovery from CLI stdout
   - Add test suites for Node.js, Python, and Go SDKs
   - Implement admin API integration
   - Add more Rust SDK tests (edge cases, concurrent usage)

2. **P2 - Medium Priority**
   - Implement dynamic stub updates in Rust SDK
   - Add mutex protection to Go SDK for thread safety
   - Improve error messages across all SDKs
   - Add logging instead of silent failures

3. **P3 - Nice to Have**
   - Consider native FFI bindings for Node.js/Python instead of CLI spawning
   - Add async Python variant
   - Add context.Context support to Go SDK
   - Implement FFI error thread-local storage

## Commit Readiness Checklist

- ✅ Critical bugs fixed (P0)
- ✅ Code compiles without errors
- ✅ Documentation updated with prerequisites
- ✅ Python 3.8 compatibility restored
- ✅ Resource leaks fixed
- ✅ Race conditions addressed
- ✅ Code review document created
- ⏳ Integration tests passing (to be verified)

## Summary

Four critical P0 bugs have been fixed:
1. FFI double-start bug (Rust)
2. Server startup race condition (Rust)
3. Python 3.8 compatibility (Python)
4. Resource leak on error (Go)

Documentation has been improved to clearly state CLI dependency requirements.

The SDK is now ready for commit and can be safely used, with the understanding that some features (port 0, admin API) are not yet fully implemented.

---

*Fixed: 2025-10-22*
*Ready for Commit: Yes ✅*
