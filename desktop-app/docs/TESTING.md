# Desktop App Testing Guide

Comprehensive testing guide for MockForge Desktop application.

## Quick Test Checklist

### Basic Functionality
- [ ] App launches without errors
- [ ] Window appears and is responsive
- [ ] System tray icon visible
- [ ] Server starts successfully
- [ ] Server stops successfully
- [ ] Status updates in real-time

### Native Features
- [ ] File associations work (.yaml, .yml, .json)
- [ ] File drag & drop works
- [ ] Notifications appear (Windows/Linux)
- [ ] Keyboard shortcuts work
- [ ] Window minimize to tray works

### Error Handling
- [ ] Port conflicts handled gracefully
- [ ] Invalid config files show errors
- [ ] Network errors handled
- [ ] Error notifications appear

## Platform-Specific Tests

### Windows
```powershell
# Test file association
Start-Process "test-config.yaml"

# Test system tray
# Right-click tray icon, test menu

# Test notifications
# Start/stop server, verify notifications
```

### macOS
```bash
# Test file association
open test-config.yaml

# Test menu bar
# Click menu bar icon, test menu

# Test notifications
# Note: May use window title instead
```

### Linux
```bash
# Test file association
xdg-open test-config.yaml

# Test system tray
# Right-click tray icon, test menu

# Test notifications
# Verify notifications appear
```

## Automated Testing

### Unit Tests
```bash
cd desktop-app
cargo test
```

### Integration Tests
```bash
# Test server integration
cargo test --test server_integration

# Test commands
cargo test --test commands
```

## Performance Testing

### Memory Usage
```bash
# Monitor memory
# Windows: Task Manager
# macOS: Activity Monitor
# Linux: htop or top
```

### Startup Time
- Target: < 3 seconds
- Measure: Time from launch to window visible

### Server Startup
- Target: < 2 seconds
- Measure: Time from start command to server responding

## User Acceptance Testing

### Test Scenarios

1. **First Launch**
   - App opens successfully
   - No errors in console
   - Window is properly sized

2. **Server Management**
   - Start server → Server runs, status updates
   - Stop server → Server stops, status updates
   - Restart server → Works correctly

3. **File Operations**
   - Open config file → Loads correctly
   - Save config file → Saves correctly
   - Drag & drop file → Opens correctly

4. **System Integration**
   - Minimize to tray → Window hides
   - Restore from tray → Window shows
   - Quit from tray → App closes

5. **Error Scenarios**
   - Port in use → Shows error
   - Invalid config → Shows error
   - Network error → Handles gracefully

## Reporting Bugs

Include:
- OS and version
- App version
- Steps to reproduce
- Expected vs actual behavior
- Error messages
- Screenshots
- System logs
