# ForgeConnect Browser Extension

Chrome/Firefox browser extension for ForgeConnect that provides a DevTools panel for managing MockForge mocks directly from your browser.

## Features

- 🔍 **Request Capture** - Automatically captures all fetch and XHR requests
- 🎯 **One-Click Mock Creation** - Create mocks from captured requests
- 📊 **DevTools Panel** - Visual interface for managing mocks
- 🔌 **Auto-Discovery** - Automatically finds MockForge server
- 📝 **Request Viewer** - View request/response details

## Installation

### Development

1. **Build the extension:**
   ```bash
   npm install
   npm run build
   ```

2. **Load in Chrome:**
   - Open `chrome://extensions/`
   - Enable "Developer mode"
   - Click "Load unpacked"
   - Select the `dist/` directory

3. **Load in Firefox:**
   - Open `about:debugging`
   - Click "This Firefox"
   - Click "Load Temporary Add-on"
   - Select `dist/manifest.json`

### Production

```bash
npm run package
# Creates forgeconnect-extension.zip
```

## Usage

1. **Start MockForge:**
   ```bash
   mockforge serve --http-port 3000 --admin
   ```

2. **Open DevTools:**
   - Open any webpage
   - Open DevTools (F12)
   - Click the "ForgeConnect" tab

3. **Capture Requests:**
   - Make API calls on the page
   - Requests appear in the "Captured Requests" panel

4. **Create Mocks:**
   - Click on a captured request
   - Click "Create Mock from Request"
   - Mock is created in MockForge

5. **Manage Mocks:**
   - View all mocks in the "Mocks" panel
   - Delete mocks with the delete button
   - Refresh to see latest mocks

## Configuration

The extension auto-discovers MockForge on:
- `http://localhost:3000`
- `http://localhost:3001`
- `http://localhost:8080`
- `http://localhost:9080`

To use a custom URL, the extension will save it in storage after first successful connection.

## Development

### Build

```bash
npm run build
```

### Watch Mode

```bash
npm run watch
```

### Package

```bash
npm run package
```

## Architecture

- **Background Service Worker** - Manages connection to MockForge
- **Content Script** - Injects SDK into page context
- **DevTools Panel** - React-based UI for mock management
- **Popup** - Quick status and actions

## License

MIT License - see [LICENSE](../../LICENSE-MIT) for details.
