# VS Code Extension Architecture Review

## Executive Summary

The MockForge VS Code extension provides a solid foundation for managing mocks from within VS Code. However, there are several architectural improvements, bug fixes, and missing features that should be addressed to enhance reliability, maintainability, and user experience.

## Critical Issues

### 1. **WebSocket Event Type Mismatch** 🔴

**Location**: `src/mocksTreeProvider.ts:11`

**Problem**: The extension checks for event types using snake_case (`mock_created`, `mock_updated`, `mock_deleted`), but the server sends events using the enum tag format (`MockCreated`, `MockUpdated`, `MockDeleted`).

**Current Code**:
```typescript
if (['mock_created', 'mock_updated', 'mock_deleted'].includes(event.type)) {
    this.refresh();
}
```

**Server Event Format** (from `management_ws.rs`):
```rust
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MockEvent {
    MockCreated { ... },
    MockUpdated { ... },
    MockDeleted { ... },
    StatsUpdated { ... },
    Connected { ... },
}
```

**Fix Required**: Update event type checking to match server format. The server uses `snake_case` for the enum tag, so events will have `type: "mock_created"` not `type: "MockCreated"`. However, the event structure includes nested data. Need to verify actual message format.

### 2. **Import API Mismatch** 🔴

**Location**: `src/mockforgeClient.ts:125-129`

**Problem**: The server's `/import` endpoint expects a JSON body with `Vec<MockConfig>`, but the extension sends a plain string with `Content-Type: text/plain`.

**Server Expectation** (from `management.rs:358-365`):
```rust
async fn import_mocks(
    State(state): State<ManagementState>,
    Json(mocks): Json<Vec<MockConfig>>,
) -> impl IntoResponse
```

**Current Extension Code**:
```typescript
await this.http.post(`/import?format=${format}&merge=${merge}`, data, {
    headers: { 'Content-Type': 'text/plain' }
});
```

**Fix Required**: Parse the file content into `MockConfig[]` and send as JSON with proper `Content-Type: application/json`.

### 3. **Missing Server Control Commands** 🟡

**Location**: `package.json:80-90`

**Problem**: Commands `mockforge.startServer`, `mockforge.stopServer`, and `mockforge.restartServer` are defined but not implemented in `extension.ts`.

**Impact**: These commands appear in the command palette but do nothing when invoked.

**Fix Required**: Implement these commands or remove them from `package.json`. Note: The server doesn't expose start/stop REST endpoints, so these commands may need to:
- Trigger external CLI commands
- Use VS Code tasks
- Or be removed if not applicable

### 4. **No WebSocket Reconnection Logic** 🟡

**Location**: `src/mockforgeClient.ts:45-73`

**Problem**: Once the WebSocket disconnects, it never reconnects. The README acknowledges this as a known issue.

**Impact**: Users must restart the extension or reload the window to reconnect after network issues.

**Fix Required**: Implement exponential backoff reconnection logic with max retry limits and user-configurable settings.

## Architectural Improvements

### 5. **Missing Type Safety for WebSocket Events**

**Current**: Events are typed as `any`:
```typescript
private listeners: ((event: any) => void)[] = [];
```

**Recommendation**: Define proper TypeScript interfaces matching the server's `MockEvent` enum:

```typescript
interface MockEvent {
    type: 'mock_created' | 'mock_updated' | 'mock_deleted' | 'stats_updated' | 'connected';
    mock?: MockConfig;
    id?: string;
    stats?: ServerStats;
    message?: string;
    timestamp: string;
}
```

### 6. **No Connection State Management**

**Problem**: The extension doesn't track or expose connection state to users.

**Recommendation**:
- Add connection state tracking (connected/disconnected/connecting)
- Show connection status in the Server Control view
- Update UI indicators based on state
- Handle connection failures gracefully

### 7. **Error Handling & Retry Logic**

**Current Issues**:
- No retry logic for failed HTTP requests
- WebSocket errors are only logged, not surfaced to users
- Network timeouts don't provide user feedback

**Recommendation**:
- Implement retry logic with exponential backoff for HTTP requests
- Add configuration for retry attempts and timeout values
- Show user-friendly error messages with actionable steps
- Implement request queuing for offline scenarios

### 8. **Type Mismatch: MockConfig Interface**

**Current**: Extension's `MockConfig` interface:
```typescript
interface MockConfig {
    response: {
        body: any;
        headers?: Record<string, string>;
    };
}
```

**Server**: Uses `MockResponse` struct:
```rust
pub struct MockResponse {
    pub body: serde_json::Value,
    pub headers: Option<HashMap<String, String>>,
}
```

**Issue**: The `body` type should be `serde_json::Value` (JSON value), not `any`. While TypeScript's `any` is flexible, using `unknown` or a proper JSON type would be safer.

### 9. **Missing Features**

**Request Logging**: No way to view request history or logs from the extension.

**Mock Response Preview**: Users can't preview mock responses without making actual requests.

**Mock Execution History**: No tracking of which mocks were triggered and when.

**Bulk Operations**: No way to enable/disable multiple mocks at once.

**Mock Validation**: No validation of mock configuration before creation.

### 10. **Configuration Management**

**Current**: Configuration is read once at activation.

**Issues**:
- Configuration changes require extension reload
- No validation of server URL format
- No connection timeout configuration

**Recommendation**:
- Listen for configuration changes and reconnect automatically
- Validate server URL format (must be valid HTTP/HTTPS URL)
- Add connection timeout configuration option
- Provide connection test command

### 11. **Code Organization**

**Current Structure**:
```
src/
  extension.ts          (main entry, all commands)
  mockforgeClient.ts    (API client)
  mocksTreeProvider.ts  (tree view)
  serverControlProvider.ts (server info view)
```

**Recommendations**:
- Split `extension.ts` into separate command handlers
- Create a `types/` directory for shared interfaces
- Add a `utils/` directory for helper functions
- Consider using a state management pattern for shared state

**Suggested Structure**:
```
src/
  commands/
    createMock.ts
    editMock.ts
    deleteMock.ts
    exportMocks.ts
    importMocks.ts
    ...
  types/
    mock.ts
    server.ts
    events.ts
  utils/
    validation.ts
    formatting.ts
  providers/
    mocksTreeProvider.ts
    serverControlProvider.ts
  services/
    mockforgeClient.ts
    connectionManager.ts
  extension.ts
```

### 12. **Testing**

**Current**: No test files found.

**Recommendation**:
- Add unit tests for API client
- Add integration tests for tree providers
- Mock WebSocket connections for testing
- Test error scenarios and edge cases

### 13. **Performance Considerations**

**Large Mock Lists**: The README mentions that large mock lists may slow down the tree view.

**Recommendation**:
- Implement virtual scrolling or pagination
- Add filtering/search functionality
- Lazy load mock details
- Debounce refresh operations

### 14. **Security Considerations**

**Current**: Server URL is user-configurable but not validated.

**Recommendations**:
- Validate server URL format
- Warn users about non-localhost connections
- Consider certificate validation for HTTPS
- Sanitize user input in mock creation

### 15. **User Experience**

**Missing UX Features**:
- No loading indicators during async operations
- No confirmation dialogs for destructive operations (except delete)
- No undo/redo support
- No keyboard shortcuts for common actions
- No status bar integration (connection status)

**Recommendations**:
- Add loading spinners to tree views
- Show progress for long operations
- Add keyboard shortcuts (e.g., `Ctrl+R` to refresh)
- Integrate connection status into status bar
- Add telemetry for feature usage (opt-in)

## Implementation Priority

### High Priority (Fix Immediately)
1. WebSocket event type mismatch (#1)
2. Import API mismatch (#2)
3. WebSocket reconnection (#4)

### Medium Priority (Next Sprint)
4. Missing server control commands (#3)
5. Connection state management (#6)
6. Error handling improvements (#7)
7. Type safety improvements (#5, #8)

### Low Priority (Backlog)
8. Code organization refactoring (#11)
9. Testing infrastructure (#12)
10. Missing features (#9)
11. UX improvements (#15)

## Code Quality Improvements

### 1. Add JSDoc Comments
All public methods and classes should have JSDoc comments explaining their purpose, parameters, and return values.

### 2. Consistent Error Messages
Standardize error message format and provide actionable guidance.

### 3. Logging Strategy
- Use VS Code's output channel for structured logging
- Add log levels (debug, info, warn, error)
- Make logs accessible via command

### 4. Configuration Validation
Add a configuration validation function that checks:
- Server URL format
- Connection timeout values
- Notification preferences

## Recommended Next Steps

1. **Immediate**: Fix critical bugs (#1, #2, #4)
2. **Short-term**: Implement missing features (#3, #6, #7)
3. **Medium-term**: Refactor code organization (#11) and add tests (#12)
4. **Long-term**: Add advanced features (#9) and UX improvements (#15)

## Additional Notes

- The extension uses `ws` package for WebSocket, which is Node.js-specific. Consider adding browser compatibility if needed.
- The `axios` HTTP client is a good choice, but consider adding request interceptors for logging and error handling.
- The tree view providers are well-structured but could benefit from better error state handling.
- Consider adding a welcome/onboarding experience for first-time users.

