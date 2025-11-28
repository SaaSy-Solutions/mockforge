# IDE Extension Guide

This guide walks you through installing, configuring, and using the MockForge VS Code extension to enhance your development workflow with inline mock previews, config validation, and seamless playground integration.

## Overview

The MockForge VS Code extension brings the power of MockForge directly into your editor, providing:

- **Peek Mock Response**: Hover over API endpoints to see mock responses inline
- **Config Validation**: Real-time validation of `mockforge.yaml` files with inline errors
- **Mocks Explorer**: Visual tree view of all mocks with real-time updates
- **Playground Integration**: Quick access to MockForge Playground from hover tooltips
- **Mock Management**: Create, edit, enable/disable, and delete mocks from VS Code

## Installation

### From VS Code Marketplace

1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "MockForge"
4. Click **Install**

Or install via command line:

```bash
code --install-extension saasy-solutions.mockforge-vscode
```

### From VSIX File

If you have a `.vsix` file:

```bash
code --install-extension mockforge-vscode-0.1.0.vsix
```

### Verify Installation

After installation, you should see:
- MockForge icon in the Activity Bar (left sidebar)
- MockForge commands available in Command Palette (Ctrl+Shift+P / Cmd+Shift+P)

## Configuration

### Basic Setup

Configure the extension in VS Code settings:

1. Open Settings (Ctrl+, / Cmd+,)
2. Search for "mockforge"
3. Configure the following:

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

### Workspace Settings

For project-specific configuration, add to `.vscode/settings.json`:

```json
{
  "mockforge.serverUrl": "http://localhost:3000",
  "mockforge.autoConnect": true
}
```

## Feature 1: Peek Mock Response

The peek response feature shows mock responses when you hover over API endpoint references in your code.

### How It Works

1. **Hover over an endpoint** in your code (JavaScript, TypeScript, or YAML)
2. **See the mock response** in a tooltip with headers and body
3. **Click "Open in Playground"** to test the endpoint interactively

### Supported Patterns

The extension detects endpoints in various patterns:

**JavaScript/TypeScript:**
```typescript
// fetch() calls
const response = await fetch('/api/users');

// axios calls
const data = await axios.get('/api/products');

// http client calls
const result = await http.get('/api/orders');
```

**YAML Config Files:**
```yaml
# mockforge.yaml
responses:
  - path: /api/users
    method: GET
```

### Example Usage

**Before (without extension):**
- You write: `fetch('/api/users')`
- You have no idea what the response looks like
- You need to run the app or check documentation

**After (with extension):**
- You write: `fetch('/api/users')`
- You hover over the endpoint
- You see:
  ```json
  {
    "users": [
      {
        "id": 1,
        "name": "John Doe",
        "email": "john@example.com"
      }
    ]
  }
  ```
- You click "Open in Playground" to test it

### Configuration

Enable/disable peek response:

```json
{
  "mockforge.inlinePreview.enabled": true
}
```

### Troubleshooting

**Preview not showing:**
1. Ensure MockForge server is running
2. Check that `mockforge.inlinePreview.enabled` is `true`
3. Verify the endpoint path matches a configured mock
4. Check VS Code output panel for errors

**Preview shows "No mock configured":**
- The endpoint doesn't have a mock configured
- Click "Open in Playground" to create one
- Or add a mock in `mockforge.yaml`

## Feature 2: Config Validation

Get real-time validation for your `mockforge.yaml` files with inline error reporting.

### How It Works

1. **Open `mockforge.yaml`** in VS Code
2. **See inline errors** for invalid configuration
3. **Get helpful messages** for missing fields, type mismatches, and invalid values
4. **Auto-detect schema type** based on file name and location

### Supported Config Types

The extension validates:

- **Main config**: `mockforge.yaml`, `mockforge.yml`, `mockforge.json`
- **Blueprint config**: `blueprint.yaml`
- **Reality config**: Files in `reality/` directory or `reality*.yaml`
- **Persona config**: Files in `personas/` directory

### Example Validation

**Invalid Configuration:**
```yaml
# mockforge.yaml
reality:
  level: invalid_level  # ❌ Error shown here
```

**Error Message:**
```
Invalid reality level: invalid_level. Valid values: static, light, moderate, high, chaos
```

**Valid Configuration:**
```yaml
# mockforge.yaml
reality:
  level: moderate  # ✅ No errors
```

### Schema Generation

The extension can auto-generate schemas:

1. **Run schema generation:**
   ```bash
   mockforge schema generate
   ```

2. **Schemas are saved** to `schemas/` directory

3. **Extension auto-detects** schemas in:
   - `schemas/` directory
   - `.mockforge/` directory
   - Project root

### Validation Features

- **Real-time validation**: Errors appear as you type
- **Accurate positions**: Line and column numbers for errors
- **Helpful messages**: Clear descriptions of what's wrong
- **Auto-detection**: Automatically detects schema type
- **Multiple schemas**: Supports all MockForge config types

### Troubleshooting

**Validation not working:**
1. Ensure you have a `mockforge.yaml` file open
2. Check that schemas are available:
   - Look for `schemas/` directory
   - Or run `mockforge schema generate`
3. Check VS Code output panel for validation errors

**Schema not found:**
- Run `mockforge schema generate` to create schemas
- Or manually place schemas in `schemas/` directory

## Feature 3: Mocks Explorer

Visual tree view of all your mocks with real-time WebSocket updates.

### Accessing Mocks Explorer

1. **Click MockForge icon** in Activity Bar (left sidebar)
2. **Open "Mocks Explorer"** view
3. **Browse all mocks** in a tree view

### Features

- **Color-coded by HTTP method**: GET (blue), POST (green), PUT (orange), DELETE (red)
- **Real-time updates**: Changes sync automatically via WebSocket
- **Context menu actions**:
  - Edit Mock
  - Delete Mock
  - Toggle Mock (enable/disable)
  - View Details

### Using Mocks Explorer

**View Mocks:**
1. Open Mocks Explorer
2. Click on a mock to see details
3. Expand folders to see grouped mocks

**Edit Mock:**
1. Right-click on a mock
2. Select "Edit Mock"
3. Modify JSON configuration
4. Save to update

**Toggle Mock:**
1. Right-click on a mock
2. Select "Toggle Mock"
3. Mock is enabled/disabled

**Delete Mock:**
1. Right-click on a mock
2. Select "Delete Mock"
3. Confirm deletion

## Feature 4: Playground Integration

Quick access to MockForge Playground from hover tooltips and commands.

### From Hover Tooltip

1. **Hover over an endpoint** in your code
2. **See the tooltip** with mock response
3. **Click "Open in Playground"** link
4. **Playground opens** in browser with endpoint pre-filled

### From Command Palette

1. **Open Command Palette** (Ctrl+Shift+P / Cmd+Shift+P)
2. **Type "MockForge: Open Playground"**
3. **Enter endpoint details** (method and path)
4. **Playground opens** in browser

### Playground URL Format

The extension constructs URLs like:
```
http://localhost:3000/admin/#/playground?method=GET&path=/api/users
```

For standalone admin UI:
```
http://localhost:9080/#/playground?method=GET&path=/api/users
```

## Feature 5: Server Control

Monitor your MockForge server status and statistics.

### Accessing Server Control

1. **Click MockForge icon** in Activity Bar
2. **Open "Server Control"** view
3. **View server statistics**

### Information Displayed

- **Connection status**: Connected / Disconnected
- **Server version**: MockForge version
- **Server port**: Port number
- **Uptime**: How long server has been running
- **Request count**: Total requests processed
- **Active mocks**: Number of active mocks

### Quick Actions

- **Start Server**: Start MockForge server
- **Stop Server**: Stop MockForge server
- **Restart Server**: Restart MockForge server

## Complete Workflow Example

Let's walk through a complete workflow using all features:

### 1. Install Extension

```bash
code --install-extension saasy-solutions.mockforge-vscode
```

### 2. Start MockForge Server

```bash
mockforge serve
```

### 3. Configure Extension

Add to `.vscode/settings.json`:

```json
{
  "mockforge.serverUrl": "http://localhost:3000",
  "mockforge.autoConnect": true,
  "mockforge.inlinePreview.enabled": true
}
```

### 4. Create Mock Config

Create `mockforge.yaml`:

```yaml
http:
  port: 3000

responses:
  - path: /api/users
    method: GET
    body:
      users:
        - id: 1
          name: "John Doe"
          email: "john@example.com"
        - id: 2
          name: "Jane Smith"
          email: "jane@example.com"
```

### 5. Use Peek Response

In your code:

```typescript
// Hover over this endpoint to see the mock response
const response = await fetch('/api/users');
```

**Hover tooltip shows:**
```json
{
  "users": [
    {
      "id": 1,
      "name": "John Doe",
      "email": "john@example.com"
    },
    {
      "id": 2,
      "name": "Jane Smith",
      "email": "jane@example.com"
    }
  ]
}
```

### 6. Open in Playground

Click "Open in Playground" in the hover tooltip to test the endpoint interactively.

### 7. Validate Config

Edit `mockforge.yaml` and see real-time validation:

```yaml
reality:
  level: moderate  # ✅ Valid
```

```yaml
reality:
  level: invalid  # ❌ Error: Invalid reality level
```

### 8. Manage Mocks

- Open Mocks Explorer
- View all mocks
- Edit, delete, or toggle mocks
- See real-time updates

## Advanced Usage

### Custom Server URL

For different environments:

```json
{
  "mockforge.serverUrl": "http://localhost:3001"
}
```

### Disable Auto-Connect

If you want to connect manually:

```json
{
  "mockforge.autoConnect": false
}
```

### Disable Notifications

To reduce notification noise:

```json
{
  "mockforge.showNotifications": false
}
```

### Disable Peek Response

If you don't want inline previews:

```json
{
  "mockforge.inlinePreview.enabled": false
}
```

## Troubleshooting

### Extension Not Connecting

**Symptoms:**
- Mocks Explorer shows "Disconnected"
- Peek response doesn't work
- Server Control shows disconnected

**Solutions:**
1. Check that MockForge server is running
2. Verify `mockforge.serverUrl` setting is correct
3. Check VS Code output panel for connection errors
4. Try restarting the extension: Command Palette → "MockForge: Restart Extension"

### Validation Not Working

**Symptoms:**
- No inline errors in `mockforge.yaml`
- Validation errors not showing

**Solutions:**
1. Ensure you have a `mockforge.yaml` file open
2. Check that schemas are available:
   - Look for `schemas/` directory
   - Or run `mockforge schema generate`
3. Check VS Code output panel for validation errors
4. Try reloading the window: Command Palette → "Developer: Reload Window"

### Peek Response Not Showing

**Symptoms:**
- Hovering over endpoints doesn't show tooltip
- Tooltip shows "No mock configured"

**Solutions:**
1. Ensure `mockforge.inlinePreview.enabled` is `true`
2. Check that MockForge server is connected
3. Verify the endpoint path matches a configured mock
4. Try hovering over different endpoint patterns
5. Check VS Code output panel for errors

### Schema Not Found

**Symptoms:**
- Validation says "Schema not available"
- Auto-detection fails

**Solutions:**
1. Run `mockforge schema generate` to create schemas
2. Ensure schemas are in `schemas/` directory
3. Check that schema file names match expected patterns
4. Verify schema JSON is valid

## Keyboard Shortcuts

The extension doesn't define custom keyboard shortcuts by default, but you can add them:

```json
{
  "key": "ctrl+shift+m",
  "command": "mockforge.refreshMocks"
}
```

## Commands

All extension commands are available via Command Palette (Ctrl+Shift+P / Cmd+Shift+P):

- `MockForge: Refresh Mocks` - Refresh mocks list
- `MockForge: Create Mock` - Create a new mock
- `MockForge: Open Playground` - Open playground in browser
- `MockForge: Show Logs` - Show extension logs
- `MockForge: Restart Extension` - Restart the extension

## Best Practices

### 1. Keep Server Running

For best experience, keep MockForge server running while developing.

### 2. Generate Schemas

Run `mockforge schema generate` to enable config validation.

### 3. Use Workspace Settings

Configure extension per-project in `.vscode/settings.json`.

### 4. Enable Auto-Connect

Set `mockforge.autoConnect: true` for seamless workflow.

### 5. Check Output Panel

If something doesn't work, check VS Code output panel for errors.

## Related Documentation

- [IDE Integration Guide](../user-guide/ide-integration.md) - Detailed feature reference
- [VS Code Extension README](../../vscode-extension/README.md) - Extension documentation
- [Configuration Files](../user-guide/configuration/files.md) - Config file format
- [Getting Started](../getting-started/getting-started.md) - MockForge basics

## Next Steps

1. **Install the extension** and configure it
2. **Start MockForge server** and connect
3. **Try peek response** by hovering over endpoints
4. **Validate your config** by editing `mockforge.yaml`
5. **Explore Mocks Explorer** to manage your mocks
6. **Use Playground** to test endpoints interactively

