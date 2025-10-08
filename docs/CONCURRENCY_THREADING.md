# Concurrency and Threading Guide

## Overview

MockForge uses Tokio's async runtime for handling concurrent operations. This document outlines concurrency patterns, potential issues, and best practices for ensuring optimal performance.

## Runtime Configuration

### Default Thread Pool

By default, Tokio uses a multi-threaded runtime with the number of worker threads equal to the number of CPU cores. When running all servers (HTTP, gRPC, WebSocket, Admin UI), all tasks share this runtime.

```rust
// Default runtime (in main.rs)
#[tokio::main]
async fn main() {
    // Tokio automatically sizes thread pool to CPU cores
}
```

### Custom Thread Pool Configuration

For production deployments with specific requirements, you can configure the runtime:

```rust
// Custom runtime configuration
#[tokio::main(worker_threads = 8)]
async fn main() {
    // 8 worker threads regardless of CPU count
}

// Or programmatically:
let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(8)
    .thread_name("mockforge-worker")
    .enable_all()
    .build()
    .unwrap();
```

## Identified Concurrency Issues & Solutions

### 1. CPU-Intensive Operations (Argon2 Password Hashing)

**Issue**: Argon2 password hashing is intentionally slow and CPU-intensive. Running it on async worker threads blocks them from handling other tasks.

**Location**: `crates/mockforge-core/src/encryption/derivation.rs`

**Solution**: Use `tokio::task::spawn_blocking` to offload to blocking thread pool.

```rust
// ✅ CORRECT: Async version using spawn_blocking
pub async fn derive_master_key_async(&self, password: String) -> EncryptionResult<EncryptionKey> {
    let params = self.default_argon2_params.clone();
    tokio::task::spawn_blocking(move || {
        let manager = Self::new();
        manager.derive_key(
            password.as_bytes(),
            KeyDerivationMethod::Argon2 {
                memory_kib: params.memory_kib,
                iterations: params.iterations,
                parallelism: params.parallelism,
            },
            "master_key_salt",
            EncryptionAlgorithm::Aes256Gcm,
        )
    })
    .await
    .map_err(|e| EncryptionError::key_derivation_failed(format!("Task join error: {}", e)))?
}

// ❌ INCORRECT: Synchronous version blocks async runtime
pub fn derive_master_key(&self, password: &str) -> EncryptionResult<EncryptionKey> {
    // This blocks the async worker thread!
    self.derive_key(password.as_bytes(), ...)
}
```

**When to use**:
- Use `derive_master_key_async()` when calling from async contexts (HTTP handlers, gRPC services)
- Use `derive_master_key()` only in synchronous contexts (CLI commands, tests)

### 2. Blocking File I/O

**Issue**: Using `std::fs::*` operations directly in async contexts blocks worker threads.

**Locations**:
- `crates/mockforge-core/src/encryption/key_management.rs`
- `crates/mockforge-ui/src/handlers.rs`
- `crates/mockforge-core/src/encryption.rs`

**Solution**: Use `tokio::fs::*` or `spawn_blocking` with `std::fs`.

```rust
// ✅ CORRECT: Async file operations
pub async fn store_key_async(&mut self, key_id: &KeyId, encrypted_key: &[u8]) -> EncryptionResult<()> {
    let file_path = self.key_file_path(key_id);
    let key_id = key_id.clone();
    let encrypted_key = encrypted_key.to_vec();

    tokio::task::spawn_blocking(move || {
        std::fs::write(&file_path, encrypted_key)
            .map_err(|e| EncryptionError::generic(format!("Failed to store key {}: {}", key_id, e)))
    })
    .await
    .map_err(|e| EncryptionError::generic(format!("Task join error: {}", e)))?
}

// Or using tokio::fs
pub async fn store_key_tokio(&mut self, key_id: &KeyId, encrypted_key: &[u8]) -> EncryptionResult<()> {
    let file_path = self.key_file_path(key_id);
    tokio::fs::write(&file_path, encrypted_key)
        .await
        .map_err(|e| EncryptionError::generic(format!("Failed to store key {}: {}", key_id, e)))
}
```

**Completed work**:
- [x] Update `mockforge-ui/src/handlers.rs` to use async file operations
  - `delete_fixture_by_id()` - Uses `spawn_blocking` for file deletion
  - `cleanup_empty_directories()` - Uses `spawn_blocking` for directory operations
  - `download_fixture_by_id()` - Uses `spawn_blocking` for file reading
  - `rename_fixture_by_id()` - Uses `spawn_blocking` for file rename
  - `move_fixture_by_id()` - Uses `spawn_blocking` for file move and mkdir
  - `save_file_to_filesystem()` - Uses `spawn_blocking` for file write/read
  - `get_parent_process_id()` - Uses `spawn_blocking` for /proc filesystem reads (Linux)
  - Helper functions: `count_fixtures_in_directory_async()` added for future use

### 3. JavaScript Execution (rquickjs)

**Issue**: The JavaScript runtime (rquickjs) is synchronous and blocks.

**Location**: `crates/mockforge-core/src/request_scripting.rs`

**Solution**: Already correctly handled with `spawn_blocking`.

```rust
// ✅ CORRECT: JavaScript execution is already offloaded
pub async fn execute_script(&self, script: &str, ...) -> Result<ScriptResult> {
    tokio::task::spawn_blocking(move || {
        let runtime = Runtime::new().expect("Failed to create JavaScript runtime");
        // JS execution happens here, off the async runtime
        context.with(|ctx| {
            ctx.eval(script.as_str()).expect("Script execution failed")
        })
    })
    .await?
}
```

**HTTP calls within scripts**:
The script engine exposes `http.get()` and `http.post()` functions. These use `block_in_place` because:
1. The JS runtime is already in `spawn_blocking`
2. We need to block the current blocking thread, not spawn another
3. This is acceptable since the entire script execution is already isolated

```rust
// Script HTTP functions use block_in_place (acceptable in this context)
let http_get_func = Function::new(ctx.clone(), |url: String| -> String {
    tokio::task::block_in_place(|| {
        reqwest::blocking::get(&url)
            .and_then(|resp| resp.text())
            .unwrap_or_else(|_| "".to_string())
    })
})?;
```

### 4. Encryption/Decryption Operations

**Issue**: AES-GCM and ChaCha20-Poly1305 encryption/decryption are CPU-bound but relatively fast.

**Current Status**: These operations are fast enough (microseconds) that offloading to `spawn_blocking` adds more overhead than benefit.

**Recommendation**: Keep current implementation for small payloads (<1MB). For large files, consider using `spawn_blocking`:

```rust
// For large payloads
pub async fn encrypt_large(&self, plaintext: &str) -> Result<String> {
    if plaintext.len() > 1_000_000 { // 1MB threshold
        let key = self.key_data.clone();
        let plaintext = plaintext.to_string();

        tokio::task::spawn_blocking(move || {
            // Perform encryption on blocking thread
            Self::encrypt_impl(&key, &plaintext)
        }).await?
    } else {
        // Fast path for small data
        self.encrypt(plaintext, None)
    }
}
```

## Best Practices

### 1. Identify Blocking Operations

**CPU-bound operations** that should use `spawn_blocking`:
- Password hashing (Argon2, PBKDF2, bcrypt)
- Heavy cryptographic operations (signing, verification)
- Large data compression/decompression
- Complex regex on large inputs
- JSON parsing of very large documents

**I/O operations** that should use `tokio::fs` or `spawn_blocking`:
- File reads/writes
- Directory traversal
- File metadata operations

### 2. Choosing Between `spawn_blocking` and Async I/O

Use `tokio::fs::*` when:
- You need just basic file operations
- You want to avoid blocking thread overhead
- You're doing many small I/O operations

Use `spawn_blocking` with `std::fs` when:
- You need features not available in `tokio::fs`
- You're working with existing synchronous code
- You're doing complex file operations

### 3. Monitor Thread Pool Usage

Add metrics to track blocking thread pool usage:

```rust
use tokio::runtime::Handle;

// Get runtime metrics
let metrics = Handle::current().metrics();
let num_blocking_threads = metrics.num_blocking_threads();
let blocking_queue_depth = metrics.blocking_queue_depth();

tracing::info!(
    "Runtime stats - Blocking threads: {}, Queue depth: {}",
    num_blocking_threads,
    blocking_queue_depth
);
```

### 4. Configure Blocking Thread Pool

For workloads with many blocking operations, increase the blocking thread pool size:

```rust
let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(4)  // Async worker threads
    .max_blocking_threads(512)  // Blocking thread pool size (default: 512)
    .enable_all()
    .build()?;
```

## Performance Considerations

### Tokio Thread Pools

Tokio maintains two thread pools:

1. **Worker Thread Pool**: Handles async tasks
   - Size: Number of CPU cores (or configured)
   - Use for: Fast, non-blocking operations

2. **Blocking Thread Pool**: Handles blocking operations
   - Size: Dynamic, up to `max_blocking_threads` (default 512)
   - Use for: CPU-intensive or blocking I/O operations

### Avoiding Starvation

**Problem**: Too many blocking operations can starve async tasks.

**Solution**:
- Use `spawn_blocking` for operations >10-100μs
- Monitor blocking queue depth
- Consider increasing `max_blocking_threads` for CPU-heavy workloads

### Load Testing Recommendations

Test your deployment under realistic load:

```bash
# Concurrent connections test
wrk -t12 -c400 -d30s http://localhost:8080/api/endpoint

# Monitor runtime metrics
curl http://localhost:9090/metrics | grep tokio

# Check thread usage
top -H -p $(pgrep mockforge)
```

## Migration Checklist

For converting blocking code to async:

- [ ] Identify blocking operations (file I/O, CPU-intensive tasks)
- [ ] Replace `std::fs::*` with `tokio::fs::*` or `spawn_blocking`
- [ ] Wrap CPU-intensive operations in `spawn_blocking`
- [ ] Add `async` to function signatures
- [ ] Update callers to `.await` the operations
- [ ] Add proper error handling for `JoinError`
- [ ] Add metrics/logging for monitoring
- [ ] Load test to verify performance improvements

## Resources

- [Tokio Tutorial - Spawning](https://tokio.rs/tokio/tutorial/spawning)
- [Async Book - Blocking Operations](https://rust-lang.github.io/async-book/07_workarounds/03_blocking.html)
- [Tokio Docs - spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
