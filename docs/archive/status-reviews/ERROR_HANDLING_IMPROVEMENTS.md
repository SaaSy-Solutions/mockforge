# Error Handling Improvements - Implementation Plan

**Date**: 2025-01-27
**Status**: In Progress
**Priority**: High

## Overview

This document tracks the systematic improvement of error handling across the MockForge codebase, replacing `unwrap()` and `expect()` calls in production code paths with proper error handling.

---

## Progress Summary

### Completed ✅
- CLI error handling infrastructure (`CliError` type with suggestions)
- Helper functions (`parse_address`, `require_config`)
- Critical path fixes (from previous work)

### In Progress 🔄
- Systematic audit of remaining unwrap() calls
- Regex compilation error handling in TypeScript stripper
- Request handling path improvements

### Planned 📋
- Comprehensive error handling documentation
- Integration test for graceful error handling

---

## 1. Configuration Loading Improvements

### File: `crates/mockforge-core/src/config.rs`

**Issue**: Regex compilation uses `unwrap()` in TypeScript type stripper

**Current Code**:
```rust
let interface_re = Regex::new(r"(?ms)interface\s+\w+\s*\{[^}]*\}\s*").unwrap();
```

**Fix**: These regex patterns are static and should compile successfully, but we should handle potential compilation errors gracefully:

```rust
// Compile regex patterns with error handling
// Note: These patterns are statically known and should never fail,
// but we handle errors to prevent panics in edge cases
let interface_re = Regex::new(r"(?ms)interface\s+\w+\s*\{[^}]*\}\s*")
    .map_err(|e| Error::generic(format!("Failed to compile regex pattern: {}", e)))?;
```

**Status**: 🔄 Ready to fix

**Impact**: Low (utility function, not critical path)

---

## 2. Server Startup Code

### File: `crates/mockforge-cli/src/main.rs`

**Status**: ✅ **Already Improved**
- Address parsing uses helper functions
- Configuration loading has proper error handling
- Most critical paths addressed

**Remaining**: Review any remaining address parsing or config loading

---

## 3. Request Handling Paths

### Files to Review:
- `crates/mockforge-http/src/handlers.rs` (if exists)
- `crates/mockforge-grpc/src/handlers.rs` (if exists)
- `crates/mockforge-ws/src/handlers.rs`

**Status**: 📋 To be audited

---

## 4. Error Handling Best Practices

### Guidelines for Future Code

1. **Use Result Types**: Always return `Result<T, E>` for fallible operations
2. **Provide Context**: Include context in error messages
3. **Use Helper Functions**: Leverage `parse_address()` and `require_config()`
4. **Log Before Returning**: Log errors for debugging
5. **User-Friendly Messages**: Provide actionable suggestions

### Example Pattern:

```rust
// ❌ BAD: Panics on invalid input
let addr = format!("{}:{}", host, port).parse().unwrap();

// ✅ GOOD: Proper error handling
let addr = progress::parse_address(
    &format!("{}:{}", host, port),
    "server address"
)?;

// ✅ ALSO GOOD: Using existing error types
let addr = format!("{}:{}", host, port)
    .parse()
    .map_err(|e| Error::Config(format!(
        "Invalid server address '{}:{}': {}", host, port, e
    )))?;
```

---

## Implementation Checklist

- [x] Create CliError type with suggestions ✅
- [x] Create helper functions (parse_address, require_config) ✅
- [ ] Replace regex unwrap() calls in config.rs
- [ ] Audit request handling paths
- [ ] Add error logging before returns
- [ ] Document error handling patterns in CONTRIBUTING.md
- [ ] Add integration test for graceful error handling

---

## Metrics

### Before (from code review):
- Total `unwrap()` calls: 3,681
- Production code unwraps: ~100-200 needing review

### Target:
- Zero unwrap() calls in critical production paths
- All user-facing errors have helpful messages
- Comprehensive error logging

---

## Next Steps

1. ✅ Fix regex unwrap() calls in TypeScript stripper
2. 📋 Audit HTTP/gRPC/WebSocket handlers
3. 📋 Add error handling documentation
4. 📋 Create integration test for error scenarios

---

**Last Updated**: 2025-01-27
