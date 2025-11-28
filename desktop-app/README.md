# MockForge Desktop Application

Native desktop application for MockForge built with Tauri.

## Features

- ✅ Native desktop app (Windows, macOS, Linux)
- ✅ Embedded mock server
- ✅ System tray integration
- ✅ Native notifications
- ✅ File associations for config files
- ✅ Auto-update support (configurable)
- ✅ Uses existing Admin UI

## Prerequisites

- Rust 1.70+
- Node.js 18+ and pnpm
- Tauri CLI: `cargo install tauri-cli` or `npm install -g @tauri-apps/cli`

### Platform-Specific Requirements

**Windows:**
- Microsoft Visual C++ Build Tools
- WebView2 (usually pre-installed on Windows 10/11)

**macOS:**
- Xcode Command Line Tools: `xcode-select --install`

**Linux:**
- WebKit2GTK development libraries
- libssl-dev
- See [Tauri Linux Dependencies](https://tauri.app/v1/guides/getting-started/prerequisites#linux)

## Development

### Setup

```bash
# Install frontend dependencies
cd crates/mockforge-ui/ui
pnpm install

# Return to root
cd ../../..

# Build desktop app
cd desktop-app
cargo tauri dev
```

### Building

```bash
# Development build
cargo tauri dev

# Production build
cargo tauri build
```

Build outputs:
- **Windows**: `desktop-app/target/release/bundle/msi/MockForge_0.2.8_x64_en-US.msi`
- **macOS**: `desktop-app/target/release/bundle/macos/MockForge.app`
- **Linux**: `desktop-app/target/release/bundle/appimage/mockforge_0.2.8_amd64.AppImage`

## Architecture

### Frontend
- Uses existing React Admin UI from `crates/mockforge-ui/ui`
- Tauri integration via `@tauri-apps/api`
- Window management and system tray

### Backend (Rust)
- `src/main.rs` - Application entry point
- `src/app.rs` - Application state management
- `src/server.rs` - Embedded mock server management
- `src/commands.rs` - Tauri command handlers
- `src/system_tray.rs` - System tray event handling

## Usage

### Starting the Server

The desktop app can start the embedded mock server automatically or on demand:

1. **Automatic Start**: Server starts when app launches (configurable)
2. **Manual Start**: Use the "Start Server" button or system tray menu
3. **Config File**: Open a config file to start with custom settings

### System Tray

- **Left Click**: Show/hide window
- **Right Click**: Context menu
  - Show/Hide
  - Start/Stop Server
  - Settings
  - Quit

### File Associations

The app can open `.yaml`, `.yml`, and `.json` config files:
- Double-click a config file to open in MockForge
- Or use File → Open in the app

## Configuration

The desktop app uses the same configuration format as the CLI:
- `mockforge.yaml` - Main config file
- Supports all CLI configuration options
- Config files can be opened from the app

## Auto-Update

Auto-update is disabled by default. To enable:

1. Set up update server
2. Configure in `tauri.conf.json`:
```json
{
  "tauri": {
    "updater": {
      "active": true,
      "endpoints": ["https://updates.mockforge.dev/{{target}}/{{current_version}}"],
      "dialog": true
    }
  }
}
```

## Troubleshooting

### Build Errors

**"WebView2 not found" (Windows)**
- Install WebView2 Runtime from Microsoft

**"WebKit2GTK not found" (Linux)**
```bash
# Ubuntu/Debian
sudo apt install libwebkit2gtk-4.0-dev libssl-dev

# Fedora
sudo dnf install webkit2gtk3-devel openssl-devel
```

### Runtime Errors

**Server won't start**
- Check if ports are already in use
- Verify config file is valid YAML/JSON
- Check logs in system console

**Window won't show**
- Check system tray for hidden window
- Try right-click → Show

## Distribution

### Code Signing

**Windows:**
- Requires code signing certificate
- Configure in `tauri.conf.json` → `bundle.windows.certificateThumbprint`

**macOS:**
- Requires Apple Developer certificate
- Configure in `tauri.conf.json` → `bundle.macOS.signingIdentity`

### Notarization (macOS)

Required for distribution outside Mac App Store:
1. Archive the app
2. Notarize with Apple
3. Staple the notarization ticket

## Roadmap

- [ ] Auto-update implementation
- [ ] File associations (OS-level)
- [ ] Dark mode support
- [ ] Keyboard shortcuts
- [ ] Multi-window support
- [ ] Plugin system integration

## License

Same as MockForge: MIT OR Apache-2.0
