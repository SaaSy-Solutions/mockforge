# Desktop App Development Notes

## Tauri Version

This app uses **Tauri 1.5**. Important notes:

### Plugins
- Tauri 1.5 uses **built-in APIs**, not separate plugins
- Plugins (like `tauri-plugin-dialog`) are for Tauri 2.0+
- Use `tauri::api::dialog` and `tauri::api::fs` directly

### Single Instance
- Tauri 1.5 doesn't have a single-instance plugin
- File associations work via OS registration
- To prevent multiple instances, you'd need to:
  - Use a lock file
  - Use named pipes/sockets
  - Upgrade to Tauri 2.0

### Notifications
- Windows/Linux: Native notifications work
- macOS: Requires entitlements for native notifications
- Current implementation uses window title as fallback on macOS

## Building

```bash
# Development
cargo tauri dev

# Production
cargo tauri build
```

## File Associations

File associations are configured in `tauri.conf.json`:
- `.yaml` / `.yml` → Opens in MockForge
- `.json` → Opens in MockForge

These are registered during installation.

## Troubleshooting

### Build Errors
- Ensure Tauri CLI is installed: `cargo install tauri-cli`
- Check Rust version: `rustc --version` (should be 1.70+)
- Verify Node.js: `node --version` (should be 18+)

### Runtime Errors
- Check console for error messages
- Verify ports are available
- Check file permissions

### File Associations Not Working
- May require admin privileges to register
- Reinstall the app to re-register associations
- Check OS file association settings
