# Common Issues & Solutions

This guide addresses the most frequently encountered issues when using MockForge and provides quick solutions.

## Server Issues

### Port Already in Use

**Problem**: `Error: Address already in use (os error 98)`

**Solutions**:

```bash
# Find what's using the port
lsof -i :3000
# On Windows: netstat -ano | findstr :3000

# Kill the process
kill -9 <PID>

# Or use a different port
mockforge serve --spec api.json --http-port 3001
```

**Prevention**: Check ports before starting:
```bash
# Quick check script
ports=(3000 3001 9080 50051)
for port in "${ports[@]}"; do
  if lsof -i :$port > /dev/null; then
    echo "Port $port is in use"
  fi
end
```

### Server Won't Start

**Problem**: MockForge exits immediately or fails silently

**Debugging Steps**:

1. **Check configuration**
```bash
# Validate config file
mockforge config validate --config mockforge.yaml
```

2. **Check logs**
```bash
# Enable verbose logging
RUST_LOG=debug mockforge serve --spec api.json 2>&1 | tee mockforge.log
```

3. **Test with minimal config**
```bash
# Start with just the spec
mockforge serve --spec examples/openapi-demo.json --http-port 3000
```

4. **Check file permissions**
```bash
ls -la api.json mockforge.yaml
chmod 644 api.json mockforge.yaml
```

## Template & Data Issues

### Template Variables Not Expanding

**Problem**: `{{uuid}}` appears literally in responses instead of generating UUIDs

**Solutions**:

```bash
# Enable template expansion via environment variable
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve --spec api.json

# Or via config file
# mockforge.yaml
http:
  response_template_expand: true

# Or via CLI flag
mockforge serve --spec api.json --response-template-expand
```

**Common Mistake**: Forgetting that template expansion is opt-in for security reasons.

### Faker Functions Not Working

**Problem**: `{{faker.name}}` not generating fake data

**Solutions**:

1. **Enable template expansion** (see above)
2. **Check faker function name**: Use lowercase, e.g., `{{faker.name}}` not `{{Faker.Name}}`
3. **Install faker if required**: Some advanced faker features may require additional setup

**Valid faker functions**:
- `{{faker.name}}` - Person name
- `{{faker.email}}` - Email address
- `{{faker.address}}` - Street address
- `{{faker.phone}}` - Phone number
- `{{faker.company}}` - Company name

See [Templating Reference](templating.md) for complete list.

### Invalid Date/Timestamp Format

**Problem**: `{{now}}` generates invalid date format

**Solutions**:

```yaml
# Use proper format in OpenAPI spec
properties:
  createdAt:
    type: string
    format: date-time  # Important!
    example: "{{now}}"
```

**Alternative**: Use custom format
```json
{
  "timestamp": "{{now | date:'%Y-%m-%d'}}"
}
```

## OpenAPI Spec Issues

### Spec Not Loading

**Problem**: `Error: Failed to parse OpenAPI specification`

**Solutions**:

1. **Validate spec syntax**
```bash
# Using swagger-cli
swagger-cli validate api.json

# Or online
# https://editor.swagger.io/
```

2. **Check file format**
```bash
# JSON
cat api.json | jq .

# YAML
yamllint api.yaml
```

3. **Check OpenAPI version**
```json
{
  "openapi": "3.0.3",  // Not "3.0" or "swagger": "2.0"
  ...
}
```

4. **Resolve JSON schema references**
```bash
# Use json-schema-ref-resolver if needed
npm install -g json-schema-ref-resolver
json-schema-ref-resolver api.json > resolved-api.json
```

### 404 for Valid Routes

**Problem**: Endpoints return 404 even though they exist in the spec

**Debugging**:

1. **Check path matching**
```bash
# Verify paths don't have trailing slashes mismatch
# Spec: /users (should match request: GET /users)
curl http://localhost:3000/users  # ✅
curl http://localhost:3000/users/ # ❌ May not match
```

2. **Check HTTP method**
```bash
# Ensure method matches spec
# Spec defines GET but you're using POST
curl -X GET http://localhost:3000/users  # ✅
curl -X POST http://localhost:3000/users # ❌ May not match
```

3. **Enable debug logging**
```bash
RUST_LOG=mockforge_http=debug mockforge serve --spec api.json
```

## CORS Issues

### CORS Errors in Browser

**Problem**: `Access to fetch at 'http://localhost:3000/users' from origin 'http://localhost:3001' has been blocked by CORS policy`

**Solutions**:

```yaml
# mockforge.yaml
http:
  cors:
    enabled: true
    allowed_origins:
      - "http://localhost:3000"
      - "http://localhost:3001"
      - "http://localhost:5173"  # Vite default
    allowed_methods: ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"]
    allowed_headers: ["Content-Type", "Authorization"]
```

**Or via environment variable**:
```bash
MOCKFORGE_CORS_ENABLED=true \
MOCKFORGE_CORS_ALLOWED_ORIGINS="http://localhost:3001,http://localhost:5173" \
mockforge serve --spec api.json
```

**Debugging**: Check browser console for exact CORS error message - it will tell you which header is missing.

## Validation Issues

### Valid Requests Getting Rejected

**Problem**: Requests return 422/400 even though they look correct

**Solutions**:

1. **Check validation mode**
```bash
# Use 'warn' instead of 'enforce' for development
MOCKFORGE_REQUEST_VALIDATION=warn mockforge serve --spec api.json
```

2. **Check Content-Type header**
```bash
# Ensure Content-Type matches spec
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "John"}'
```

3. **Check required fields**
```bash
# Spec may require fields you're not sending
# Check spec for 'required' array
```

4. **Validate request body structure**
```bash
# Use Admin UI to see exact request received
# Visit http://localhost:9080 to inspect requests
```

### Validation Too Strict

**Problem**: Validation rejects requests that should be valid

**Solutions**:

1. **Temporarily disable validation**
```bash
mockforge serve --spec api.json --validation off
```

2. **Fix spec if it's incorrect**
```json
// Spec might mark optional fields as required
"properties": {
  "name": { "type": "string" },
  "email": { "type": "string" }
},
"required": []  // Empty array = all optional
```

## WebSocket Issues

### Connection Refused

**Problem**: WebSocket connection fails immediately

**Solutions**:

1. **Check WebSocket port**
```bash
# Verify port is open
netstat -tlnp | grep :3001
```

2. **Check replay file exists**
```bash
# Ensure file path is correct
ls -la ws-replay.jsonl
MOCKFORGE_WS_REPLAY_FILE=./ws-replay.jsonl mockforge serve --ws-port 3001
```

3. **Check WebSocket enabled**
```bash
# Ensure WebSocket server is started
mockforge serve --ws-port 3001  # Explicit port needed
```

### Messages Not Received

**Problem**: WebSocket connects but no messages arrive

**Solutions**:

1. **Check replay file format**
```bash
# Validate JSONL syntax
cat ws-replay.jsonl | jq -r '.'  # Should parse each line as JSON
```

2. **Check message timing**
```json
// Replay file format
{"ts": 0, "dir": "out", "text": "Welcome"}
{"ts": 1000, "dir": "out", "text": "Next message"}
```

3. **Check waitFor patterns**
```json
// Ensure regex patterns match
{"waitFor": "^CLIENT_READY$", "text": "Acknowledged"}
```

## Configuration Issues

### Config File Not Found

**Problem**: `Error: Configuration file not found`

**Solutions**:

1. **Use absolute path**
```bash
mockforge serve --config /full/path/to/mockforge.yaml
```

2. **Check file name**
```bash
# Valid names
mockforge.yaml
mockforge.yml
.mockforge.yaml
.mockforge.yml
mockforge.config.ts
mockforge.config.js
```

3. **Check current directory**
```bash
pwd
ls -la mockforge.yaml
```

### Environment Variables Not Applied

**Problem**: Environment variables seem to be ignored

**Solutions**:

1. **Check variable names**
```bash
# Correct format: MOCKFORGE_<SECTION>_<OPTION>
MOCKFORGE_HTTP_PORT=3000       # ✅
MOCKFORGE_PORT=3000            # ❌ Wrong
```

2. **Check shell reload**
```bash
# Export and verify
export MOCKFORGE_HTTP_PORT=3000
echo $MOCKFORGE_HTTP_PORT  # Should show 3000

# Or use inline
MOCKFORGE_HTTP_PORT=3000 mockforge serve --spec api.json
```

3. **Check precedence**
```bash
# CLI flags override env vars
mockforge serve --spec api.json --http-port 3001
# Even if MOCKFORGE_HTTP_PORT=3000, port will be 3001
```

## Performance Issues

### Slow Response Times

**Problem**: API responses are slow

**Solutions**:

1. **Disable template expansion if not needed**
```bash
# Template expansion adds overhead
mockforge serve --spec api.json  # No templates = faster
```

2. **Reduce validation overhead**
```bash
# Validation adds latency
mockforge serve --spec api.json --validation warn  # Faster than 'enforce'
```

3. **Check response complexity**
```bash
# Large responses or complex templates slow things down
# Consider simplifying responses for development
```

4. **Monitor resource usage**
```bash
# Check CPU/memory
top -p $(pgrep mockforge)
```

### High Memory Usage

**Problem**: MockForge consumes too much memory

**Solutions**:

1. **Limit connection pool**
```bash
MOCKFORGE_MAX_CONNECTIONS=100 mockforge serve --spec api.json
```

2. **Disable features not needed**
```bash
# Minimal configuration
mockforge serve --spec api.json \
  --validation off \
  --response-template-expand false \
  --admin false
```

3. **Check for memory leaks**
```bash
# Monitor over time
watch -n 1 'ps aux | grep mockforge | grep -v grep'
```

## Docker Issues

### Container Exits Immediately

**Problem**: Docker container starts then immediately stops

**Solutions**:

1. **Check logs**
```bash
docker logs <container-id>
docker logs -f <container-id>  # Follow logs
```

2. **Run interactively**
```bash
docker run -it --rm mockforge mockforge serve --spec api.json
```

3. **Check volume mounts**
```bash
# Ensure spec file is accessible
docker run -v $(pwd)/api.json:/app/api.json \
  mockforge mockforge serve --spec /app/api.json
```

### Port Mapping Issues

**Problem**: Can't access MockForge from host

**Solutions**:

```bash
# Proper port mapping
docker run -p 3000:3000 -p 9080:9080 mockforge

# Verify ports are exposed
docker port <container-id>
```

### Permission Issues

**Problem**: Can't read/write mounted volumes

**Solutions**:

```bash
# Fix permissions
sudo chown -R 1000:1000 ./fixtures ./logs

# Or run as specific user
docker run --user $(id -u):$(id -g) \
  -v $(pwd)/fixtures:/app/fixtures \
  mockforge
```

## Admin UI Issues

### Admin UI Not Loading

**Problem**: Can't access http://localhost:9080

**Solutions**:

1. **Enable admin UI**
```bash
mockforge serve --spec api.json --admin --admin-port 9080
```

2. **Check port**
```bash
# Verify port is listening
curl http://localhost:9080
netstat -tlnp | grep :9080
```

3. **Try different port**
```bash
mockforge serve --spec api.json --admin --admin-port 9090
# Access at http://localhost:9090
```

### Admin API Not Working

**Problem**: Admin UI loads but API calls fail

**Solutions**:

```bash
# Test admin API directly
curl http://localhost:9080/__mockforge/status

# Enable admin API explicitly
mockforge serve --spec api.json --admin --admin-api-enabled
```

## Plugin Issues

### Plugin Won't Load

**Problem**: `Error: Failed to load plugin`

**Solutions**:

1. **Check plugin format**
```bash
# Validate WASM file
file plugin.wasm  # Should show: WebAssembly

# Check plugin manifest
mockforge plugin validate plugin.wasm
```

2. **Check permissions**
```bash
# Ensure plugin file is readable
chmod 644 plugin.wasm
```

3. **Check compatibility**
```bash
# Plugin may be for different MockForge version
mockforge --version
# Check plugin requirements
```

### Plugin Crashes

**Problem**: Plugin causes MockForge to crash

**Solutions**:

1. **Check plugin logs**
```bash
RUST_LOG=mockforge_plugin=debug mockforge serve --plugin ./plugin.wasm
```

2. **Check resource limits**
```yaml
# plugin.yaml
capabilities:
  resources:
    max_memory_bytes: 67108864  # 64MB
    max_cpu_time_ms: 5000      # 5 seconds
```

## Getting More Help

If none of these solutions work:

1. **Collect debug information**
```bash
# System info
uname -a
rustc --version
mockforge --version

# Check logs
RUST_LOG=debug mockforge serve --spec api.json 2>&1 | tee debug.log

# Test with minimal config
mockforge serve --spec examples/openapi-demo.json --http-port 3000
```

2. **Search existing issues**
   - Check [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)
   - Search for similar problems

3. **Create minimal reproduction**
   - Create smallest possible config that reproduces issue
   - Include OpenAPI spec (if relevant)
   - Include error logs

4. **Open GitHub issue**
   - Use descriptive title
   - Include system info, version, logs
   - Attach minimal reproduction

---

**See Also**:
- [Troubleshooting Guide](troubleshooting.md) - Detailed diagnostic steps
- [FAQ](faq.md) - Common questions and answers
- [Configuration Reference](../configuration/files.md) - All configuration options

