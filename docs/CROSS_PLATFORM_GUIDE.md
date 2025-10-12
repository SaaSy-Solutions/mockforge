# Cross-Platform Compatibility Guide

MockForge is built in Rust and designed to work seamlessly across Linux, macOS, and Windows. This guide addresses platform-specific considerations and best practices.

## Platform Support

MockForge is officially tested and supported on:

- **Linux** (Ubuntu 20.04+, Debian 10+, RHEL 8+, and other distributions)
- **macOS** (macOS 11 Big Sure and later)
- **Windows** (Windows 10, Windows 11, Windows Server 2019+)

All platforms receive the same features and functionality, with automatic CI/CD testing ensuring consistent behavior.

## Installation

### Linux and macOS

```bash
# Install from crates.io
cargo install mockforge-cli

# Or use pre-built binaries
wget https://github.com/SaaSy-Solutions/mockforge/releases/latest/download/mockforge-linux-x64-mockforge
chmod +x mockforge-linux-x64-mockforge
sudo mv mockforge-linux-x64-mockforge /usr/local/bin/mockforge
```

### Windows

```powershell
# Install from crates.io
cargo install mockforge-cli

# Or use pre-built binaries
# Download from: https://github.com/SaaSy-Solutions/mockforge/releases/latest
# Extract mockforge.exe to a directory in your PATH

# Or use with winget (if available)
# winget install MockForge
```

## Path Handling

MockForge uses Rust's cross-platform `Path` and `PathBuf` types, which automatically handle platform-specific path separators.

### General Best Practices

```bash
# Forward slashes work on all platforms (recommended)
mockforge workspace sync --target-dir ./git-sync

# Use quotes for paths with spaces
mockforge workspace sync --target-dir "C:/My Documents/API Configs"
```

### Windows-Specific Considerations

#### Path Separators
- Both forward slashes (`/`) and backslashes (`\`) work on Windows
- **Recommendation**: Use forward slashes for better cross-platform compatibility
- MockForge automatically normalizes paths internally

```powershell
# All of these work on Windows:
mockforge workspace sync --target-dir C:/sync
mockforge workspace sync --target-dir C:\sync
mockforge workspace sync --target-dir "C:\Program Files\MockForge\sync"
```

#### Drive Letters
Windows paths with drive letters are fully supported:

```powershell
# Absolute paths with drive letters
mockforge workspace sync --target-dir D:/workspace-sync

# UNC paths (network shares)
mockforge workspace sync --target-dir \\server\share\mockforge
```

#### Long Paths

Windows has historically limited paths to 260 characters (MAX_PATH). Modern Windows 10/11 with long path support enabled can handle longer paths.

**To enable long path support on Windows 10/11:**

1. Open Registry Editor (`regedit`)
2. Navigate to: `HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\FileSystem`
3. Set `LongPathsEnabled` to `1`
4. Restart your computer

Alternatively, use Group Policy:
- Open Group Policy Editor (`gpedit.msc`)
- Navigate to: Computer Configuration → Administrative Templates → System → Filesystem
- Enable "Enable Win32 long paths"

**Workaround for systems without long path support:**

If you encounter path length issues:
- Use shorter directory names
- Place MockForge data closer to drive root (e.g., `C:/mockforge` instead of `C:/Users/username/Documents/Projects/API/mockforge`)
- Use the `subst` command to create a virtual drive:
  ```powershell
  subst M: "C:\very\long\path\to\mockforge\data"
  mockforge workspace sync --target-dir M:/sync
  ```

#### Special Characters

Windows has restrictions on certain characters in filenames. MockForge automatically sanitizes workspace names when creating directories.

**Reserved characters on Windows:**
- `< > : " / \ | ? *`

**Reserved names:**
- `CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`

MockForge handles these automatically, but avoid using them in manual file creation.

### Linux and macOS Considerations

#### Case Sensitivity
- **Linux**: File systems are typically case-sensitive (`Workspace` ≠ `workspace`)
- **macOS**: File systems are case-insensitive but case-preserving by default (`Workspace` = `workspace`)
- **Windows**: File systems are case-insensitive (`Workspace` = `workspace`)

**Best Practice**: Always use consistent casing in configuration files and paths.

#### Permissions
Unix-like systems (Linux, macOS) have more granular file permissions:

```bash
# Ensure MockForge binary is executable
chmod +x mockforge

# Workspace sync directories need write permissions
chmod 755 ~/mockforge-data
```

#### Home Directory
Use `~` or `$HOME` in Unix-like systems:

```bash
# Unix (Linux/macOS)
mockforge workspace sync --target-dir ~/mockforge-sync

# Windows equivalent
mockforge workspace sync --target-dir %USERPROFILE%\mockforge-sync
```

## Workspace Synchronization

The `workspace sync` feature works identically on all platforms, with automatic path normalization.

### Git Integration

Git commands work the same across platforms. Ensure Git is installed:

```bash
# Linux (Ubuntu/Debian)
sudo apt-get install git

# macOS
brew install git

# Windows
# Download from: https://git-scm.com/download/win
```

**Example: Cross-platform sync to Git**

```bash
# Works on all platforms
mkdir api-configs
cd api-configs
git init
mockforge workspace sync --target-dir . --structure nested
git add .
git commit -m "Initial sync"
```

### Docker Considerations

When using Docker, be aware of volume mount differences:

```bash
# Linux/macOS
docker run -v $(pwd)/data:/data mockforge

# Windows (PowerShell)
docker run -v ${PWD}/data:/data mockforge

# Windows (CMD)
docker run -v %cd%/data:/data mockforge
```

**Best Practice**: Use Docker Compose with relative paths for cross-platform compatibility:

```yaml
version: '3.8'
services:
  mockforge:
    image: mockforge:latest
    volumes:
      - ./data:/data
      - ./config:/config
```

## Environment Variables

### Setting Variables

```bash
# Linux/macOS
export MOCKFORGE_CONFIG=/path/to/config.yaml

# Windows (PowerShell)
$env:MOCKFORGE_CONFIG = "C:\path\to\config.yaml"

# Windows (CMD)
set MOCKFORGE_CONFIG=C:\path\to\config.yaml
```

### Cross-Platform .env Files

MockForge loads `.env` files automatically. Use forward slashes for paths:

```bash
# .env (works on all platforms)
MOCKFORGE_CONFIG=./config/config.yaml
MOCKFORGE_DATA_DIR=./data
MOCKFORGE_LOG_LEVEL=info
```

## Testing

Run the test suite to verify your installation:

```bash
# All platforms
cargo test --workspace

# Run cross-platform specific tests
cargo test --test sync_cross_platform_tests
```

## Troubleshooting

### Windows

**Problem**: "Access denied" errors when running MockForge
- **Solution**: Run PowerShell or Command Prompt as Administrator, or adjust permissions on the installation directory

**Problem**: Git commands fail with "command not found"
- **Solution**: Ensure Git is installed and in your PATH. Restart your terminal after installation.

**Problem**: "The filename, directory name, or volume label syntax is incorrect"
- **Solution**: Check for invalid characters in paths. Use quotes around paths with spaces.

**Problem**: Path too long errors (>260 characters)
- **Solution**: Enable long path support (see "Long Paths" section above) or use shorter paths

### Linux

**Problem**: "Permission denied" when running MockForge binary
- **Solution**: Make the binary executable: `chmod +x mockforge`

**Problem**: Cannot write to sync directory
- **Solution**: Check directory permissions: `ls -la` and adjust with `chmod`

### macOS

**Problem**: "mockforge cannot be opened because the developer cannot be verified"
- **Solution**: Right-click the binary, select "Open", then click "Open" again in the dialog

**Problem**: Sync directory permissions issues
- **Solution**: Grant Full Disk Access in System Preferences → Security & Privacy → Privacy → Full Disk Access

## Performance Considerations

### File System Performance

- **Linux**: Generally fastest file I/O
- **Windows**: May be slower with antivirus scanning enabled (add MockForge directories to exclusions)
- **macOS**: Good performance, but APFS encryption may add overhead

### Recommendations

1. **Exclude from Antivirus**: Add MockForge installation and data directories to antivirus exclusions
2. **Use SSD**: Store workspace data on SSD for best performance
3. **Local Sync**: For workspace sync, use local directories rather than network drives when possible

## CI/CD Integration

MockForge includes GitHub Actions workflows that test on all three platforms. See `.github/workflows/test.yml` for examples.

### Example: Cross-Platform CI

```yaml
name: Cross-Platform Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace
```

## Additional Resources

- [Rust Path Documentation](https://doc.rust-lang.org/std/path/index.html)
- [Git for Windows](https://git-scm.com/download/win)
- [Windows Long Paths](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation)
- [Docker Desktop for Windows](https://docs.docker.com/desktop/windows/install/)

## Getting Help

If you encounter platform-specific issues:

1. Check this guide for known solutions
2. Search [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)
3. Open a new issue with:
   - Platform and version (e.g., Windows 11, Ubuntu 22.04)
   - Full error message
   - Steps to reproduce
   - Output of `mockforge --version` and `cargo --version`
