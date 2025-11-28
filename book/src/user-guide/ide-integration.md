# IDE Integration

MockForge provides first-class IDE integration to enhance your development workflow. This guide covers the VS Code extension and its features.

## VS Code Extension

The MockForge VS Code extension brings the power of MockForge directly into your editor, providing real-time mock management, validation, and preview capabilities.

### Installation

Install the extension from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=saasy-solutions.mockforge-vscode):

1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "MockForge"
4. Click Install

Or install via command line:

```bash
code --install-extension saasy-solutions.mockforge-vscode
```

### Configuration

Configure the extension in VS Code settings:

```json
{
  "mockforge.serverUrl": "http://localhost:3000",
  "mockforge.autoConnect": true,
  "mockforge.showNotifications": true,
  "mockforge.inlinePreview.enabled": true
}
```

**Settings:**
- `mockforge.serverUrl`: MockForge server URL (default: `http://localhost:3000`)
- `mockforge.autoConnect`: Automatically connect on startup (default: `true`)
- `mockforge.showNotifications`: Show notifications for mock changes (default: `true`)
- `mockforge.inlinePreview.enabled`: Enable inline preview of mock responses (default: `true`)

## Features

### Peek Mock Response

Hover over API endpoint references in your code to see mock responses inline.

**Supported Patterns:**
- `fetch('/api/users')`
- `axios.get('/api/products')`
- `http.get('/api/orders')`
- Endpoint paths in `mockforge.yaml` files

**Usage:**
1. Hover over any API endpoint reference in your code
2. See the mock response (headers and body) in a tooltip
3. Click "Open in Playground" to test the endpoint interactively

**Example:**

```typescript
// Hover over this endpoint to see the mock response
const response = await fetch('/api/users');
```

The hover tooltip will show:
- Response headers
- Response body (formatted JSON)
- Link to open in MockForge Playground

### Config Validation

Get real-time validation for your `mockforge.yaml` files with inline error reporting.

**Features:**
- Inline error reporting with accurate line/column positions
- Validates against JSON Schema generated from MockForge config types
- Supports multiple config types:
  - Main config (`mockforge.yaml`)
  - Reality config (`reality/*.yaml`)
  - Persona config (`personas/*.yaml`)
  - Blueprint config (`blueprint.yaml`)
- Auto-detects schema type based on file name and location
- Helpful error messages for:
  - Missing required fields
  - Type mismatches
  - Invalid enum values
  - Format errors (email, uri, etc.)
  - Pattern mismatches

**Example:**

```yaml
# mockforge.yaml
reality:
  level: invalid_level  # ❌ Error: Invalid reality level
```

The extension will show an inline error:
```
Invalid reality level: invalid_level. Valid values: static, light, moderate, high, chaos
```

### Mocks Explorer

Visual tree view of all your mocks with real-time WebSocket updates.

**Features:**
- Browse all mocks in a tree view
- Color-coded by HTTP method (GET, POST, PUT, DELETE, etc.)
- Real-time updates when mocks change
- Context menu actions:
  - Edit Mock
  - Delete Mock
  - Toggle Mock (enable/disable)
  - View Details

**Usage:**
1. Open the MockForge sidebar (click the MockForge icon in the activity bar)
2. Browse your mocks in the "Mocks Explorer" view
3. Click on a mock to see details
4. Right-click for context menu actions

### Server Control

Monitor your MockForge server status and statistics.

**Features:**
- Connection status indicator
- Server version and port
- Uptime and request count
- Active mocks count
- Quick actions to start/stop/restart server

**Usage:**
1. Open the "Server Control" panel in the MockForge sidebar
2. View server statistics
3. Use quick actions to manage the server

### Playground Integration

Quick access to MockForge Playground from hover tooltips.

**Usage:**
1. Hover over an API endpoint reference
2. Click "Open in Playground" in the tooltip
3. The MockForge Admin UI playground opens in your browser
4. The endpoint method and path are pre-filled

## Workflow Integration

### Development Workflow

1. **Start MockForge server**: `mockforge start`
2. **Open VS Code**: The extension auto-connects
3. **Edit config files**: Get real-time validation
4. **Hover over endpoints**: See mock responses inline
5. **Test in Playground**: Click "Open in Playground" from hover tooltips

### Config Validation Workflow

1. **Edit `mockforge.yaml`**: Start typing your configuration
2. **See inline errors**: Validation happens in real-time
3. **Fix errors**: Follow the error messages to correct issues
4. **Generate schemas** (optional): Run `mockforge schema generate` to create local schemas

### Mock Management Workflow

1. **View mocks**: Open Mocks Explorer
2. **Create mock**: Click "+" icon or use command palette
3. **Edit mock**: Right-click → Edit Mock
4. **Test mock**: Hover over endpoint reference to see response
5. **Open in Playground**: Click link in hover tooltip

## Troubleshooting

### Extension Not Connecting

1. Check that MockForge server is running
2. Verify `mockforge.serverUrl` setting is correct
3. Check VS Code output panel for connection errors
4. Try restarting the extension: Command Palette → "MockForge: Restart Extension"

### Validation Not Working

1. Ensure you have a `mockforge.yaml` file open
2. Check that schemas are available:
   - Look for `schemas/` directory in workspace
   - Or run `mockforge schema generate` to create schemas
3. Check VS Code output panel for validation errors

### Hover Preview Not Showing

1. Ensure `mockforge.inlinePreview.enabled` is `true`
2. Check that MockForge server is connected
3. Verify the endpoint path matches a configured mock
4. Try hovering over different endpoint patterns

## Related Documentation

- [VS Code Extension README](../../../vscode-extension/README.md)
- [Configuration Files](../configuration/files.md)
- [Getting Started](../getting-started/getting-started.md)

