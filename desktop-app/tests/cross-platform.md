# Cross-Platform Testing Guide

This document outlines testing procedures for MockForge Desktop on Windows, macOS, and Linux.

## Test Matrix

| Feature | Windows | macOS | Linux | Notes |
|---------|---------|-------|-------|-------|
| **Build** | ✅ | ✅ | ✅ | All platforms build |
| **Launch** | ⏳ | ⏳ | ⏳ | Test app startup |
| **System Tray** | ⏳ | ⏳ | ⏳ | Test tray icon and menu |
| **Server Start** | ⏳ | ⏳ | ⏳ | Test embedded server |
| **File Associations** | ⏳ | ⏳ | ⏳ | Test .yaml/.yml/.json |
| **Notifications** | ⏳ | ⏳ | ⏳ | Test native notifications |
| **File Drop** | ⏳ | ⏳ | ⏳ | Test drag & drop |
| **Keyboard Shortcuts** | ⏳ | ⏳ | ⏳ | Test global shortcuts |
| **Auto-Update** | ⏳ | ⏳ | ⏳ | Test update check |
| **Icons** | ⏳ | ⏳ | ⏳ | Verify icon display |

## Windows Testing

### Prerequisites
- Windows 10/11
- Visual Studio Build Tools
- WebView2 Runtime

### Test Steps

1. **Build**
   ```powershell
   cargo tauri build
   ```

2. **Install**
   - Run the generated MSI installer
   - Verify installation location
   - Check Start Menu entry

3. **Launch**
   - Launch from Start Menu
   - Verify window appears
   - Check system tray icon

4. **File Associations**
   - Right-click a .yaml file
   - Select "Open with" → MockForge
   - Verify file opens in app

5. **System Tray**
   - Right-click tray icon
   - Test all menu items
   - Test show/hide window

6. **Server**
   - Start server from UI
   - Verify HTTP server responds
   - Stop server
   - Verify shutdown

7. **Notifications**
   - Start server → Should show notification
   - Stop server → Should show notification

8. **Shortcuts**
   - Test Ctrl+Shift+S (start server)
   - Test Ctrl+Shift+X (stop server)
   - Test Ctrl+Shift+H (show/hide)

## macOS Testing

### Prerequisites
- macOS 10.13+
- Xcode Command Line Tools
- Code signing certificate (for distribution)

### Test Steps

1. **Build**
   ```bash
   cargo tauri build
   ```

2. **Install**
   - Open the generated .app bundle
   - Drag to Applications (if needed)
   - Verify Gatekeeper allows execution

3. **Launch**
   - Launch from Applications
   - Verify window appears
   - Check menu bar icon

4. **File Associations**
   - Double-click a .yaml file
   - Verify opens in MockForge
   - Check "Open with" context menu

5. **System Tray (Menu Bar)**
   - Click menu bar icon
   - Test all menu items
   - Test show/hide window

6. **Server**
   - Start server from UI
   - Verify HTTP server responds
   - Test in browser

7. **Notifications**
   - Note: Native notifications require entitlements
   - Current: Window title updates
   - Future: Add entitlements for native notifications

8. **Shortcuts**
   - Test Cmd+Shift+S (start server)
   - Test Cmd+Shift+X (stop server)
   - Test Cmd+Shift+H (show/hide)

## Linux Testing

### Prerequisites
- Ubuntu 20.04+ / Fedora 34+ / Debian 11+
- WebKit2GTK development libraries
- libssl-dev

### Test Steps

1. **Build**
   ```bash
   cargo tauri build
   ```

2. **Install**
   - Install AppImage or DEB package
   - Verify desktop entry created
   - Check application menu

3. **Launch**
   - Launch from application menu
   - Verify window appears
   - Check system tray icon

4. **File Associations**
   - Double-click a .yaml file
   - Verify opens in MockForge
   - Check file manager integration

5. **System Tray**
   - Right-click tray icon
   - Test all menu items
   - Test show/hide window

6. **Server**
   - Start server from UI
   - Verify HTTP server responds
   - Test with curl

7. **Notifications**
   - Start server → Should show notification
   - Verify notification appears
   - Test notification actions

8. **Shortcuts**
   - Test Ctrl+Shift+S (start server)
   - Test Ctrl+Shift+X (stop server)
   - Test Ctrl+Shift+H (show/hide)

## Automated Testing

### Unit Tests
```bash
cd desktop-app
cargo test
```

### Integration Tests
```bash
# Test server startup
cargo test --test server_tests

# Test commands
cargo test --test command_tests
```

## Known Platform-Specific Issues

### Windows
- WebView2 may need manual installation on older systems
- File associations require admin privileges to register
- Notifications work natively

### macOS
- Native notifications require entitlements and code signing
- Gatekeeper may block unsigned apps
- File associations work after first launch

### Linux
- System tray requires compatible desktop environment
- Notifications depend on notification daemon
- File associations vary by distribution

## Test Checklist

Before release, verify:

- [ ] App builds on all platforms
- [ ] App launches without errors
- [ ] System tray icon appears
- [ ] Server starts and stops correctly
- [ ] File associations work
- [ ] Notifications display
- [ ] Keyboard shortcuts work
- [ ] File drag & drop works
- [ ] Window minimize to tray works
- [ ] Auto-update check works (if enabled)
- [ ] Icons display correctly
- [ ] No console errors
- [ ] Memory leaks (basic check)
- [ ] Performance is acceptable

## Reporting Issues

When reporting platform-specific issues, include:
- OS version
- App version
- Steps to reproduce
- Error messages
- Screenshots (if applicable)
- System logs
