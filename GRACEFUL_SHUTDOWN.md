# Graceful Shutdown Implementation

## Overview
Implemented comprehensive graceful shutdown and error propagation for the MockForge CLI server orchestration.

## Problem Statement
Previously, when multiple server services (HTTP, WebSocket, gRPC, UI, Metrics) were spawned:
- Errors were only logged with `eprintln!` but not propagated
- Failed tasks would exit silently while others continued running
- The CLI always returned success (`Ok(())`) even when servers failed
- No mechanism to cancel remaining tasks when one failed
- Orphaned background tasks could continue running after the main process exited

## Solution Implemented

### 1. Added CancellationToken for Coordinated Shutdown
```rust
use tokio_util::sync::CancellationToken;
let shutdown_token = CancellationToken::new();
```

Each server task receives a clone of the token and listens for cancellation signals.

### 2. Proper Error Propagation
Each spawned task now:
- Returns `Result<(), String>` from the async block
- Wraps server errors with context (e.g., "HTTP server error: {}")
- Handles both graceful shutdown (via cancellation) and errors

### 3. Enhanced tokio::select! Logic
The main select! now:
- Properly unwraps `JoinHandle<Result<(), String>>` results
- Distinguishes between:
  - Graceful shutdown: `Ok(Ok(()))`
  - Server error: `Ok(Err(e))`
  - Task panic: `Err(e)`
- Captures the first error/exit to return

### 4. Coordinated Shutdown Sequence
When any task exits or fails:
1. The select! completes with the result
2. Logs the appropriate message (graceful or error)
3. Calls `shutdown_token.cancel()` to signal all other tasks
4. Waits 100ms for graceful shutdown
5. Returns error if any server failed, Ok otherwise

### 5. Improved Error Messages
- Bind failures are detected early in metrics server
- All errors include context about which server failed
- Panic handling with descriptive messages
- Non-zero exit code on any failure

## Code Changes

### File: `crates/mockforge-cli/Cargo.toml`
Added dependency:
```toml
tokio-util = "0.7"
```

Also added missing dependencies that were used but not declared:
- `mockforge-recorder`
- `reqwest`

### File: `crates/mockforge-cli/src/main.rs`

**Lines 816-1040**: Complete rewrite of server orchestration logic in `handle_serve`:

1. **Each server spawn** (HTTP, WS, gRPC, Admin, Metrics):
   - Wrapped in `tokio::select!` to listen for cancellation
   - Returns `Result<(), String>` instead of just logging errors
   - Handles shutdown signal gracefully

2. **Main select! loop** (lines 922-1026):
   - Properly handles all result combinations
   - Stores first error/exit in `result: Option<String>`
   - Returns None for graceful Ctrl+C

3. **Shutdown sequence** (lines 1028-1040):
   - Cancels all tasks via token
   - Waits for cleanup
   - Returns error if any server failed

## Benefits

1. **Reliability**: No orphaned processes after CLI exits
2. **Visibility**: Clear error messages show which server failed and why
3. **Proper Exit Codes**: Non-zero exit on failure enables scripting/monitoring
4. **Resource Cleanup**: All tasks properly shut down together
5. **User Experience**: Clear indication of what went wrong

## Testing Recommendations

Test the following scenarios:
1. All servers start successfully → graceful Ctrl+C shutdown
2. Port already in use → immediate error + cleanup
3. One server crashes mid-run → all others shut down
4. Rapid startup failures → proper error propagation

## Example Error Output

**Before** (port conflict):
```
❌ HTTP server error: Address already in use
[Other servers keep running...]
[Returns Ok(()) / exit code 0]
```

**After** (port conflict):
```
❌ HTTP server error: Address already in use
👋 Shutting down remaining servers...
[All servers stop]
[Returns Err / exit code 1]
```

## Related Issues
- Addresses the graceful shutdown concern from code review
- Improves error handling as noted in release readiness checklist
- Prevents resource leaks and orphaned processes
