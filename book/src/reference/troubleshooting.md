# Troubleshooting

This guide helps you diagnose and resolve common issues with MockForge. If you're experiencing problems, follow the steps below to identify and fix the issue.

## Quick Diagnosis

### Check Server Status

First, verify that MockForge is running and accessible:

```bash
# Check if processes are running
ps aux | grep mockforge

# Check listening ports
netstat -tlnp | grep -E ":(3000|3001|50051|9080)"

# Test basic connectivity
curl -I http://localhost:3000/health 2>/dev/null || echo "HTTP server not responding"
curl -I http://localhost:9080/health 2>/dev/null || echo "Admin UI not responding"
```

### Check Logs

Enable verbose logging to see detailed information:

```bash
# Run with debug logging
RUST_LOG=mockforge=debug mockforge serve --spec api-spec.yaml

# View recent logs
tail -f mockforge.log

# Filter logs by component
grep "ERROR" mockforge.log
grep "WARN" mockforge.log
```

## HTTP API Issues

### Server Won't Start

**Symptoms**: `mockforge serve` exits immediately with error

**Common causes and solutions**:

1. **Port already in use**:
   ```bash
   # Find what's using the port
   lsof -i :3000

   # Kill conflicting process
   kill -9 <PID>

   # Or use different port
   mockforge serve --http-port 3001
   ```

2. **Invalid OpenAPI specification**:
   ```bash
   # Validate YAML syntax
   yamllint api-spec.yaml

   # Validate OpenAPI structure
   swagger-cli validate api-spec.yaml

   # Test with minimal spec
   mockforge serve --spec examples/openapi-demo.json
   ```

3. **File permissions**:
   ```bash
   # Check file access
   ls -la api-spec.yaml

   # Fix permissions if needed
   chmod 644 api-spec.yaml
   ```

### 404 Errors for Valid Routes

**Symptoms**: API returns 404 for endpoints that should exist

**Possible causes**:

1. **OpenAPI spec not loaded correctly**:
   ```bash
   # Check if spec was loaded
   grep "OpenAPI spec loaded" mockforge.log

   # Verify file path
   ls -la api-spec.yaml
   ```

2. **Path matching issues**:
   - Ensure paths in spec match request URLs
   - Check for trailing slashes
   - Verify HTTP methods match

3. **Template expansion disabled**:
   ```bash
   # Enable template expansion
   mockforge serve --response-template-expand --spec api-spec.yaml
   ```

### Template Variables Not Working

**Symptoms**: `{{variable}}` appears literally in responses

**Solutions**:

1. **Enable template expansion**:
   ```bash
   # Via command line
   mockforge serve --response-template-expand --spec api-spec.yaml

   # Via environment variable
   MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve --spec api-spec.yaml

   # Via config file
   echo "response:\n  template_expand: true" > config.yaml
   mockforge serve --config config.yaml --spec api-spec.yaml
   ```

2. **Check template syntax**:
   - Use `{{variable}}` not `${variable}`
   - Ensure variables are defined in spec examples
   - Check for typos in variable names

### Validation Errors

**Symptoms**: Requests return 400/422 with validation errors

**Solutions**:

1. **Adjust validation mode**:
   ```bash
   # Disable validation
   mockforge serve --validation off --spec api-spec.yaml

   # Use warning mode
   mockforge serve --validation warn --spec api-spec.yaml
   ```

2. **Fix request format**:
   - Ensure Content-Type header matches request body format
   - Verify required fields are present
   - Check parameter formats match OpenAPI spec

## WebSocket Issues

### Connection Fails

**Symptoms**: WebSocket client cannot connect

**Common causes**:

1. **Wrong port or path**:
   ```bash
   # Check WebSocket port
   netstat -tlnp | grep :3001

   # Test connection
   websocat ws://localhost:3001/ws
   ```

2. **Replay file not found**:
   ```bash
   # Check file exists
   ls -la ws-replay.jsonl

   # Run without replay file
   mockforge serve --ws-port 3001  # No replay file specified
   ```

### Messages Not Received

**Symptoms**: WebSocket connection established but no messages

**Solutions**:

1. **Check replay file format**:
   ```bash
   # Validate JSONL syntax
   node -e "
   const fs = require('fs');
   const lines = fs.readFileSync('ws-replay.jsonl', 'utf8').split('\n');
   lines.forEach((line, i) => {
     if (line.trim()) {
       try { JSON.parse(line); }
       catch (e) { console.log(\`Line \${i+1}: \${e.message}\`); }
     }
   });
   "
   ```

2. **Verify message timing**:
   - Check `ts` values are in milliseconds
   - Ensure messages have required fields (`ts`, `dir`, `text`)

### Interactive Mode Issues

**Symptoms**: Client messages not triggering responses

**Debug steps**:

1. **Check regex patterns**:
   ```bash
   # Test regex patterns
   node -e "
   const pattern = '^HELLO';
   const test = 'HELLO world';
   console.log('Match:', test.match(new RegExp(pattern)));
   "
   ```

2. **Verify state management**:
   - Check that state variables are properly set
   - Ensure conditional logic is correct

## gRPC Issues

### Service Not Found

**Symptoms**: `grpcurl list` shows no services

**Solutions**:

1. **Check proto directory**:
   ```bash
   # Verify proto files exist
   find proto/ -name "*.proto"

   # Check directory path
   MOCKFORGE_PROTO_DIR=proto/ mockforge serve --grpc-port 50051
   ```

2. **Compilation errors**:
   ```bash
   # Check for proto compilation errors
   cargo build --verbose 2>&1 | grep -i proto
   ```

3. **Reflection disabled**:
   ```bash
   # Enable gRPC reflection
   MOCKFORGE_GRPC_REFLECTION_ENABLED=true mockforge serve --grpc-port 50051
   ```

### Method Calls Fail

**Symptoms**: gRPC calls return errors

**Debug steps**:

1. **Check service definition**:
   ```bash
   # List service methods
   grpcurl -plaintext localhost:50051 describe mockforge.user.UserService
   ```

2. **Validate request format**:
   ```bash
   # Test with verbose output
   grpcurl -plaintext -v -d '{"user_id": "123"}' localhost:50051 mockforge.user.UserService/GetUser
   ```

3. **Check proto compatibility**:
   - Ensure client and server use same proto definitions
   - Verify message field names and types match

## Admin UI Issues

### UI Not Loading

**Symptoms**: Browser shows connection error

**Solutions**:

1. **Check admin port**:
   ```bash
   # Verify port is listening
   curl -I http://localhost:9080 2>/dev/null || echo "Admin UI not accessible"

   # Try different port
   mockforge serve --admin --admin-port 9090
   ```

2. **CORS issues**:
   - Admin UI should work from any origin by default
   - Check browser console for CORS errors

3. **Embedded vs standalone**:
   ```bash
   # Force standalone mode
   mockforge serve --admin --admin-standalone

   # Or embedded mode
   mockforge serve --admin --admin-embed
   ```

### API Endpoints Not Working

**Symptoms**: UI loads but API calls fail

**Solutions**:

1. **Check admin API**:
   ```bash
   # Test admin API directly
   curl http://localhost:9080/__mockforge/status
   ```

2. **Enable admin API**:
   ```bash
   # Ensure admin API is not disabled
   mockforge serve --admin  # Don't use --disable-admin-api
   ```

## Configuration Issues

### Config File Not Loading

**Symptoms**: Settings from config file are ignored

**Solutions**:

1. **Validate YAML syntax**:
   ```bash
   # Check YAML format
   python3 -c "import yaml; yaml.safe_load(open('config.yaml'))"

   # Or use yamllint
   yamllint config.yaml
   ```

2. **Check file path**:
   ```bash
   # Use absolute path
   mockforge serve --config /full/path/to/config.yaml

   # Verify file permissions
   ls -la config.yaml
   ```

3. **Environment variable override**:
   - Remember that environment variables override config file settings
   - Command-line arguments override both

### Environment Variables Not Working

**Symptoms**: Environment variables are ignored

**Common issues**:

1. **Shell not reloaded**:
   ```bash
   # Export variable and reload shell
   export MOCKFORGE_HTTP_PORT=3001
   exec $SHELL
   ```

2. **Variable name typos**:
   ```bash
   # Check variable is set
   echo $MOCKFORGE_HTTP_PORT

   # List all MockForge variables
   env | grep MOCKFORGE
   ```

## Performance Issues

### High Memory Usage

**Symptoms**: MockForge consumes excessive memory

**Solutions**:

1. **Reduce concurrent connections**:
   ```bash
   # Limit connection pool
   MOCKFORGE_MAX_CONNECTIONS=100 mockforge serve
   ```

2. **Disable unnecessary features**:
   ```bash
   # Run with minimal features
   mockforge serve --validation off --response-template-expand false
   ```

3. **Monitor resource usage**:
   ```bash
   # Check memory usage
   ps aux | grep mockforge

   # Monitor over time
   htop -p $(pgrep mockforge)
   ```

### Slow Response Times

**Symptoms**: API responses are slow

**Debug steps**:

1. **Enable latency logging**:
   ```bash
   RUST_LOG=mockforge=debug mockforge serve --spec api-spec.yaml 2>&1 | grep -i latency
   ```

2. **Check template complexity**:
   - Complex templates can slow response generation
   - Consider caching for frequently used templates

3. **Profile performance**:
   ```bash
   # Use cargo flamegraph for profiling
   cargo flamegraph --bin mockforge-cli -- serve --spec api-spec.yaml
   ```

## Docker Issues

### Container Won't Start

**Symptoms**: Docker container exits immediately

**Solutions**:

1. **Check container logs**:
   ```bash
   docker logs <container-id>

   # Run with verbose output
   docker run --rm mockforge mockforge serve --spec api-spec.yaml
   ```

2. **Volume mounting issues**:
   ```bash
   # Ensure spec file is accessible
   docker run -v $(pwd)/api-spec.yaml:/app/api-spec.yaml \
     mockforge mockforge serve --spec /app/api-spec.yaml
   ```

3. **Port conflicts**:
   ```bash
   # Use different ports
   docker run -p 3001:3000 -p 3002:3001 mockforge
   ```

## Getting Help

### Log Analysis

```bash
# Extract error patterns
grep "ERROR" mockforge.log | head -10

# Find recent issues
tail -100 mockforge.log | grep -E "(ERROR|WARN)"

# Count error types
grep "ERROR" mockforge.log | sed 's/.*ERROR //' | sort | uniq -c | sort -nr
```

### Debug Commands

```bash
# Full system information
echo "=== System Info ==="
uname -a
echo "=== Rust Version ==="
rustc --version
echo "=== Cargo Version ==="
cargo --version
echo "=== Running Processes ==="
ps aux | grep mockforge
echo "=== Listening Ports ==="
netstat -tlnp | grep -E ":(3000|3001|50051|9080)"
echo "=== Disk Space ==="
df -h
echo "=== Memory Usage ==="
free -h
```

### Community Support

If you can't resolve the issue:

1. **Check existing issues**: Search GitHub issues for similar problems
2. **Create a minimal reproduction**: Isolate the issue with minimal configuration
3. **Include debug information**: Attach logs, configuration, and system details
4. **Use descriptive titles**: Clearly describe the problem in issue titles

### Emergency Stop

If MockForge is causing issues:

```bash
# Kill all MockForge processes
pkill -f mockforge

# Kill specific process
kill -9 <mockforge-pid>

# Clean up any leftover files
rm -f mockforge.log
```

This troubleshooting guide covers the most common issues. For more specific problems, check the logs and consider creating an issue on GitHub with detailed information about your setup and the problem you're experiencing.
