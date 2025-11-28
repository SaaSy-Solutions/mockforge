# Automated Test Scripts - Dependencies and Setup

This document describes the dependencies and setup required for running MockForge's automated test scripts.

## Quick Start

If you just want to run the tests, install the optional dependencies:

```bash
# On Arch Linux / Manjaro
sudo pacman -S gnu-netcat curl  # or: openbsd-netcat (OpenBSD variant)

# On Ubuntu/Debian
sudo apt-get install netcat-openbsd curl

# On macOS
brew install netcat curl

# On Fedora/RHEL
sudo dnf install nc curl
```

## Required Dependencies

### Essential (Automatically Checked)
- **`mockforge` CLI**: The MockForge command-line tool (installed via `make install` or `cargo install`)
- **`bash`**: Version 4.0 or later (standard on most Unix-like systems)
- **`curl`**: HTTP client (used for HTTP health checks and API testing)
  - Usually pre-installed on Linux/macOS
  - Install: `sudo apt-get install curl` (Ubuntu) or `brew install curl` (macOS)

### Optional (Enhanced Testing)
- **`nc` (netcat)**: Network utility for port checking
  - **Purpose**: Verifies that servers have actually bound to their ports
  - **Impact**: Tests will still work without it, but port verification is skipped
  - **Installation**:
    ```bash
    # Arch Linux
    sudo pacman -S gnu-netcat  # or: openbsd-netcat (OpenBSD variant)

    # Ubuntu/Debian
    sudo apt-get install netcat-openbsd

    # macOS
    brew install netcat

    # Fedora/RHEL
    sudo dnf install nc
    ```

## What Happens Without Optional Dependencies?

### Without `nc` (netcat)
- Tests will still run successfully
- Server startup verification will check if the process is running instead of checking if the port is bound
- A warning message will appear: `"(port check skipped - nc not available)"`
- All functional tests (CLI commands, API endpoints) will still work normally

### Without `curl`
- HTTP protocol tests will fail
- Most other protocol tests will still work
- You should install `curl` for full test coverage

## Testing Individual Scripts

Each test script can be run independently:

```bash
# Test Kafka broker
./scripts/automated-tests/test-kafka.sh

# Test MQTT broker
./scripts/automated-tests/test-mqtt.sh

# Test AMQP broker
./scripts/automated-tests/test-amqp.sh

# Test FTP server
./scripts/automated-tests/test-ftp.sh

# Test API Flight Recorder
./scripts/automated-tests/test-recorder.sh
```

## Troubleshooting

### "nc: command not found"
- **Impact**: Low - port checking is skipped, but functionality tests still run
- **Solution**: Install netcat (see above) or ignore the warning

### "curl: command not found"
- **Impact**: High - HTTP tests will fail
- **Solution**: Install curl (usually pre-installed, but if not: `sudo apt-get install curl`)

### "mockforge: command not found"
- **Impact**: Critical - tests cannot run
- **Solution**:
  ```bash
  # Build and install from source
  cargo build --release
  cargo install --path crates/mockforge-cli

  # Or use make
  make install
  ```

### Port Already in Use Errors
- **Impact**: Tests may fail if ports are already bound
- **Solution**: The test scripts automatically clean up processes, but if you see persistent errors:
  ```bash
  # Kill any processes on test ports
  ./scripts/clear-ports.sh

  # Or manually
  sudo fuser -k 9092/tcp  # Kafka
  sudo fuser -k 1883/tcp  # MQTT
  sudo fuser -k 5672/tcp  # AMQP
  sudo fuser -k 2121/tcp  # FTP
  ```

## CI/CD Integration

For CI/CD pipelines, install dependencies:

```yaml
# Example GitHub Actions
- name: Install dependencies
  run: |
    sudo apt-get update
    sudo apt-get install -y netcat-openbsd curl

# Example GitLab CI
before_script:
  - apt-get update -qq
  - apt-get install -y netcat-openbsd curl
```

## Minimal Test Environment

If you want to run tests with minimal dependencies:

1. **Required**: `mockforge` CLI + `bash` + `curl`
2. **Skip**: `nc` (tests will use process checking instead)

The tests are designed to be resilient and will work with just the essentials.
