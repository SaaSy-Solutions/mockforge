# Coverage Target Analysis

## Summary
Current coverage: **48.04%** (target: 80%)

## Top Priority Files with Uncovered Lines

### 1. priority_handler.rs - Critical Uncovered Lines

#### Lines 237-241: Custom Fixture Serialization Error Path
```rust
// Line 236-241: Error handling when custom fixture response can't be serialized
serde_json::to_string(&custom_fixture.response).map_err(|e| {
    Error::generic(format!(
        "Failed to serialize custom fixture response: {}",
        e
    ))
})?
```
**Test Needed**: Create a custom fixture with a response that fails JSON serialization (e.g., circular reference, invalid type)

#### Lines 266-282: Replay Handler Path
```rust
// Lines 266-282: Replay handler with recorded fixture
if let Some(recorded_request) = self.record_replay.replay_handler().load_fixture(&fingerprint).await?
```
**Test Needed**: Actually record a request and then replay it to hit this path

#### Lines 301, 313, 318, 321-329: Behavioral Scenario Replay Edge Cases
- Line 301: Timing delay application
- Lines 313, 318, 321-329: Session ID extraction from cookies (not just headers)
**Test Needed**: 
- Test with session ID in cookies instead of headers
- Test with timing delay
- Test error handling in try_replay

#### Line 337: Route Chaos Latency Injection Error
```rust
// Line 336-338: Error handling when latency injection fails
if let Err(e) = route_chaos.inject_latency(method, uri).await {
    tracing::warn!("Failed to inject per-route latency: {}", e);
}
```
**Test Needed**: Create a route chaos injector that returns an error from inject_latency

#### Lines 400-700: Proxy/Continuum Blending (MAJOR GAP)
This entire section has many uncovered lines:
- **Lines 402-406**: Migration mode checking
- **Lines 409-410**: Mock mode migration (skip proxy)
- **Lines 416-521**: Continuum blending with both proxy and mock
- **Lines 523-530**: Continuum fallback when mock generation fails
- **Lines 531-538**: Continuum fallback when mock generation errors
- **Lines 539-570**: Continuum fallback when proxy fails but mock succeeds
- **Lines 572-588**: Continuum when both fail
- **Lines 593-662**: Normal proxy handling
- **Lines 608-636**: Shadow mode comparison
- **Lines 664-675**: Proxy error handling with migration modes

**Test Needed**: 
- Test proxy handler with continuum engine enabled
- Test shadow mode
- Test migration modes (Mock, Real)
- Test continuum blending scenarios
- Test proxy error paths

### 2. openapi/response.rs - Uncovered Lines

#### Lines 250, 252-253, 255: Response generation edge cases
**Test Needed**: Test response generation with edge case schemas

#### Lines 264-265, 282-283: Example extraction
**Test Needed**: Test with examples that have different structures

#### Lines 295, 297, 303-304, 306, 309: Scenario selection
**Test Needed**: Test different scenario selection modes

#### Lines 327, 329, 333-344, 346, 349: Schema generation paths
**Test Needed**: Test schema generation with different types and references

### 3. workspace/request.rs - Uncovered Lines

Most uncovered lines are in:
- Request execution with caching
- Request execution with environment variable substitution
- Request matching with complex patterns
- Route registry updates with nested folders

### 4. openapi/validation.rs - Uncovered Lines

Most uncovered lines are in:
- Parameter validation with schema references
- Request body validation with complex schemas
- Response header validation
- Error message formatting

## Recommended Test Strategy

### Phase 1: High-Impact, Low-Effort
1. **Custom fixture serialization error** (priority_handler.rs:237-241)
   - Create fixture with unserializable response
   - Verify error is handled gracefully

2. **Replay handler with actual recording** (priority_handler.rs:266-282)
   - Record a request
   - Replay it to hit the replay path

3. **Route chaos latency error** (priority_handler.rs:337)
   - Create injector that returns error
   - Verify error is logged but doesn't crash

### Phase 2: Medium-Impact, Medium-Effort
4. **Behavioral scenario replay edge cases** (priority_handler.rs:301-329)
   - Test with cookies for session ID
   - Test with timing delays
   - Test error handling

5. **Proxy handler basic paths** (priority_handler.rs:593-675)
   - Test normal proxy handling
   - Test proxy error handling
   - Test migration modes

### Phase 3: High-Impact, High-Effort
6. **Continuum blending** (priority_handler.rs:416-588)
   - Test blending with both responses
   - Test fallback scenarios
   - Test error handling

7. **Shadow mode** (priority_handler.rs:608-636)
   - Test shadow mode comparison
   - Test when responses differ

## Estimated Coverage Impact

- Phase 1: +0.5-1.0% coverage
- Phase 2: +1.0-2.0% coverage  
- Phase 3: +2.0-3.0% coverage

Total potential: +3.5-6.0% coverage (bringing us to ~51-54%)

## Next Steps

1. Start with Phase 1 tests (quick wins)
2. Measure coverage impact after each phase
3. Continue iterating on highest-impact uncovered lines

