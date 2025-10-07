# MockForge VS Code Extension

Control and visualize your MockForge API mocks directly from Visual Studio Code.

## Features

- **Mocks Explorer**: View all your mocks in a tree view
- **Real-time Updates**: WebSocket connection for live mock synchronization
- **Mock Management**: Create, edit, enable/disable, and delete mocks
- **Export/Import**: Export mocks to JSON/YAML and import them back
- **Server Control**: Monitor server status, stats, and configuration
- **Quick Actions**: Context menu actions for common tasks

## Installation

1. Install the extension from VS Code Marketplace
2. Configure your MockForge server URL in settings (default: `http://localhost:3000`)
3. The extension will auto-connect on startup if enabled

## Configuration

- `mockforge.serverUrl`: MockForge server URL (default: `http://localhost:3000`)
- `mockforge.autoConnect`: Automatically connect on startup (default: `true`)
- `mockforge.showNotifications`: Show notifications for mock changes (default: `true`)

## Usage

### View Mocks

1. Open the MockForge sidebar (click the MockForge icon in the activity bar)
2. Browse your mocks in the "Mocks Explorer" view
3. Click on a mock to see details

### Create a Mock

1. Click the "+" icon in the Mocks Explorer toolbar
2. Enter mock details (name, method, path, response)
3. The mock will be created and appear in the list

### Edit a Mock

1. Right-click a mock in the explorer
2. Select "Edit Mock"
3. Modify the JSON configuration
4. Save to update the mock

### Export/Import Mocks

**Export:**
1. Run command "MockForge: Export Mocks"
2. Select format (JSON/YAML)
3. Choose save location

**Import:**
1. Run command "MockForge: Import Mocks"
2. Select file to import
3. Choose import strategy (Replace/Merge)

### Monitor Server

View server statistics in the "Server Control" panel:
- Connection status
- Server version and port
- Uptime and request count
- Active mocks count

## Requirements

- MockForge server running (v0.1.0 or higher)
- Node.js 18+ (for WebSocket support)

## Known Issues

- WebSocket reconnection not yet implemented
- Large mock lists may slow down the tree view

## Release Notes

### 0.1.0

- Initial release
- Basic mock management
- WebSocket live updates
- Export/import functionality
- Server monitoring

## Contributing

Found a bug or have a feature request? Please open an issue on our [GitHub repository](https://github.com/SaaSy-Solutions/mockforge).

## License

MIT OR Apache-2.0
