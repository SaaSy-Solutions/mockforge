# Change Log

All notable changes to the MockForge VS Code Extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-12-XX

### Added

- **Mocks Explorer**: Tree view for browsing all mocks
- **Real-time Updates**: WebSocket connection for live mock synchronization
- **Mock Management**: Create, edit, enable/disable, and delete mocks
- **Export/Import**: Export mocks to JSON/YAML and import them back
- **Server Control**: Monitor server status, stats, and configuration
- **Quick Actions**: Context menu actions for common tasks
- **Peek Mock Response**: Hover over API endpoint references to see mock responses inline
  - Works with `fetch()`, `axios`, and other HTTP client calls
  - Shows mock response headers and body
  - Quick link to open endpoint in MockForge Playground
- **Config Validation**: Real-time validation of `mockforge.yaml` files
  - Inline error reporting with accurate line/column positions
  - Validates against JSON Schema for all config types
  - Supports main config, reality config, persona config, and blueprint config
  - Auto-detects schema type based on file patterns
- **Playground Integration**: Quick access to MockForge Playground from hover tooltips

### Configuration

- `mockforge.serverUrl`: MockForge server URL (default: `http://localhost:3000`)
- `mockforge.autoConnect`: Automatically connect on startup (default: `true`)
- `mockforge.showNotifications`: Show notifications for mock changes (default: `true`)
- `mockforge.inlinePreview.enabled`: Enable inline preview of mock responses (default: `true`)

### Requirements

- MockForge server running (v0.1.0 or higher)
- VS Code 1.85.0 or higher

[0.1.0]: https://github.com/SaaSy-Solutions/mockforge/releases/tag/vscode-extension-v0.1.0
