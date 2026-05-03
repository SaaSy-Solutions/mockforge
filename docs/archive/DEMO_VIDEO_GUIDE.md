# MockForge Demo Video Guide

This guide provides instructions for creating a demo video/screencast of MockForge using `asciinema`.

## Prerequisites

```bash
# Install asciinema
# On macOS:
brew install asciinema

# On Linux (Ubuntu/Debian):
sudo apt-get install asciinema

# On other systems, see: https://asciinema.org/docs/installation
```

## Recording Setup

### 1. Prepare Your Terminal

```bash
# Set terminal to 80x24 or 120x30 for best viewing
export COLUMNS=120
export LINES=30

# Clear terminal history
clear

# Optional: Set a nice PS1 prompt
export PS1='$ '
```

### 2. Recording Commands

```bash
# Start recording
asciinema rec mockforge-demo.cast

# When done recording
# Press Ctrl+D or type 'exit'
```

## Demo Script (2 Minutes)

### Introduction (15 seconds)

```bash
# Title card (comment in recording)
echo "MockForge - Advanced API Mocking Platform"
sleep 2

# Show version
mockforge --version
```

### Part 1: Quick Start (30 seconds)

```bash
# Start MockForge with OpenAPI spec
echo "# Starting MockForge with OpenAPI demo spec..."
mockforge serve --spec examples/openapi-demo.json --admin &
sleep 3

# Test HTTP endpoint
echo "# Testing HTTP endpoint..."
curl http://localhost:3000/ping | jq
sleep 2

# Test user creation
echo "# Creating a user..."
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice", "email": "alice@example.com"}' | jq
sleep 2
```

### Part 2: WebSocket Demo (25 seconds)

```bash
# Start WebSocket
echo "# Testing WebSocket replay..."
# In a separate terminal or screen:
mockforge serve --ws-port 3001 \
  --ws-replay-file examples/ws-demo.jsonl &
sleep 2

# Connect and interact
echo "# Connecting to WebSocket..."
websocat ws://localhost:3001/ws <<EOF
CLIENT_READY
ACK
CONFIRMED
EOF
sleep 3
```

### Part 3: Admin UI (20 seconds)

```bash
echo "# Opening Admin UI at http://localhost:9080"
# Show quick curl to admin API
curl http://localhost:9080/api/health | jq
sleep 2

echo "# View real-time logs and metrics in browser"
sleep 3
```

### Part 4: Advanced Features (20 seconds)

```bash
# Template expansion
echo "# Template expansion demo..."
curl http://localhost:3000/users/123 | jq
sleep 2

# Show dynamic timestamps and UUIDs
echo "# Notice: Dynamic UUIDs and timestamps in responses"
sleep 2

# Chain requests
echo "# Request chaining example..."
curl http://localhost:3000/health | jq
sleep 2
```

### Conclusion (10 seconds)

```bash
# Cleanup
echo "# Stopping servers..."
pkill mockforge

echo "# Learn more:"
echo "  - Documentation: https://docs.mockforge.dev"
echo "  - GitHub: https://github.com/SaaSy-Solutions/mockforge"
echo "  - Install: cargo install mockforge-cli"
sleep 3
```

## Post-Recording

### Convert to GIF

```bash
# Install agg (asciinema GIF generator)
cargo install --git https://github.com/asciinema/agg

# Convert to GIF
agg mockforge-demo.cast mockforge-demo.gif

# Optimize GIF size
# Install gifsicle first: brew install gifsicle
gifsicle -O3 --colors 256 mockforge-demo.gif -o mockforge-demo-optimized.gif
```

### Upload and Share

```bash
# Upload to asciinema.org
asciinema upload mockforge-demo.cast

# Or embed in README
# Add to README.md:
# [![asciicast](https://asciinema.org/a/YOUR_CAST_ID.svg)](https://asciinema.org/a/YOUR_CAST_ID)
```

## Alternative: Screen Recording with Zoom

If you prefer a visual/browser demo:

1. **Setup**:
   - Clean desktop environment
   - Browser window at 1920x1080 or 1280x720
   - Terminal window side-by-side with browser

2. **Recording Software**:
   - OBS Studio (free, cross-platform)
   - QuickTime (macOS)
   - SimpleScreenRecorder (Linux)

3. **Demo Flow**:
   - Show CLI commands in terminal
   - Show Admin UI in browser
   - Demonstrate real-time updates
   - Show plugin installation
   - Demonstrate data generation

## Tips for Great Demos

1. **Keep it concise**: 2-3 minutes maximum
2. **Show real value**: Focus on key features
3. **Use realistic examples**: Relatable use cases
4. **Add subtitles/annotations**: Explain what's happening
5. **Test beforehand**: Do dry runs to perfect timing
6. **Clear terminal often**: Keep output clean
7. **Use `sleep` commands**: Give viewers time to read
8. **Highlight key output**: Use `jq` for JSON formatting

## Demo Scenarios

### Scenario 1: Frontend Developer

```bash
# Problem: Backend API not ready
# Solution: MockForge with OpenAPI spec

mockforge serve --spec api-spec.yaml
# Now frontend can develop against mock API
```

### Scenario 2: Integration Testing

```bash
# Problem: Need deterministic test fixtures
# Solution: MockForge with seeded data

export MOCKFORGE_SEED=12345
mockforge data template user --rows 100 > users.json
# Same seed = same data every time
```

### Scenario 3: Load Testing

```bash
# Problem: Don't want to hit production
# Solution: MockForge with configurable latency

mockforge serve --spec api.yaml \
  --latency-mean 100ms \
  --failure-rate 0.05
# Realistic load testing without production risk
```

## Example README Badge

```markdown
## Demo

[![Demo](https://asciinema.org/a/YOUR_CAST_ID.svg)](https://asciinema.org/a/YOUR_CAST_ID)

Or view the [2-minute demo GIF](./docs/assets/mockforge-demo.gif)
```

## Checklist

- [ ] Install asciinema
- [ ] Test demo script locally
- [ ] Record demo (aim for < 2 minutes)
- [ ] Review recording
- [ ] Convert to GIF if needed
- [ ] Upload to asciinema.org
- [ ] Add link to README.md
- [ ] Consider creating additional screencasts for specific features

## Additional Demo Ideas

1. **Plugin System Demo**: Install and use a custom plugin
2. **gRPC Demo**: Show protobuf-based mocking
3. **GraphQL Demo**: Demonstrate GraphQL endpoint mocking
4. **Workspace Sync**: Show Git-based workspace synchronization
5. **Encryption Demo**: Demonstrate end-to-end encryption features

## Resources

- [asciinema Documentation](https://asciinema.org/docs/)
- [agg (GIF generator)](https://github.com/asciinema/agg)
- [OBS Studio](https://obsproject.com/)
- [Demo Automation with expect](https://likegeeks.com/expect-command/)
