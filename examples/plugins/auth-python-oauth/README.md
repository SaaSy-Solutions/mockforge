# OAuth2 Authentication Plugin (Python Remote)

A production-ready OAuth2 authentication plugin for MockForge, written in Python as a remote service. This plugin validates OAuth2 access tokens by calling an external authorization server's introspection endpoint.

## 🎯 Features

- ✅ OAuth2 token validation via introspection
- ✅ User information retrieval
- ✅ Token caching for performance
- ✅ Configurable via environment variables
- ✅ Health checks and monitoring
- ✅ Docker support
- ✅ Full Python ecosystem access (requests, etc.)
- ✅ Async/await support

## 🚀 Quick Start

### Prerequisites

- Python 3.9 or later
- pip

### Local Development

```bash
# Install dependencies
pip install -r requirements.txt

# Set environment variables
export AUTH_SERVER_URL="https://your-auth-server.com"
export OAUTH_CLIENT_ID="your-client-id"
export OAUTH_CLIENT_SECRET="your-client-secret"

# Run the plugin
python plugin.py

# Plugin will start on http://localhost:8080
```

### Docker Deployment

```bash
# Build the image
docker build -t mockforge-oauth-plugin .

# Run the container
docker run -d \
  -p 8080:8080 \
  -e AUTH_SERVER_URL="https://your-auth-server.com" \
  -e OAUTH_CLIENT_ID="your-client-id" \
  -e OAUTH_CLIENT_SECRET="your-client-secret" \
  --name mockforge-oauth-plugin \
  mockforge-oauth-plugin

# Check health
curl http://localhost:8080/health
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mockforge-oauth-plugin
spec:
  replicas: 2
  selector:
    matchLabels:
      app: mockforge-oauth-plugin
  template:
    metadata:
      labels:
        app: mockforge-oauth-plugin
    spec:
      containers:
      - name: plugin
        image: mockforge-oauth-plugin:latest
        ports:
        - containerPort: 8080
        env:
        - name: AUTH_SERVER_URL
          valueFrom:
            configMapKeyRef:
              name: mockforge-config
              key: auth_server_url
        - name: OAUTH_CLIENT_ID
          valueFrom:
            secretKeyRef:
              name: oauth-credentials
              key: client_id
        - name: OAUTH_CLIENT_SECRET
          valueFrom:
            secretKeyRef:
              name: oauth-credentials
              key: client_secret
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: mockforge-oauth-plugin
spec:
  selector:
    app: mockforge-oauth-plugin
  ports:
  - port: 80
    targetPort: 8080
  type: ClusterIP
```

## 📝 Configuration

### Environment Variables

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `AUTH_SERVER_URL` | OAuth2 authorization server URL | Yes | - |
| `OAUTH_CLIENT_ID` | OAuth2 client ID for introspection | Yes | `mockforge` |
| `OAUTH_CLIENT_SECRET` | OAuth2 client secret | No | - |
| `HOST` | Host to bind to | No | `0.0.0.0` |
| `PORT` | Port to listen on | No | `8080` |

### MockForge Configuration

Add to your `config.yaml`:

```yaml
plugins:
  - id: auth-python-oauth
    enabled: true
    config:
      auth_server_url: "https://auth.example.com"
      client_id: "mockforge"
      client_secret: "${OAUTH_CLIENT_SECRET}"  # From env

endpoints:
  - path: "/api/*"
    methods: ["GET", "POST", "PUT", "DELETE"]
    auth:
      plugin: auth-python-oauth
      required: true
```

## 🔧 How It Works

1. **Token Extraction**: Plugin extracts the Bearer token from the `Authorization` header
2. **Token Introspection**: Calls the OAuth2 introspection endpoint to validate the token
3. **User Info Retrieval**: Optionally fetches additional user information
4. **Result Caching**: Caches valid tokens for performance (configurable TTL)
5. **Response**: Returns authentication result to MockForge

### Flow Diagram

```
MockForge → HTTP POST /plugin/authenticate
    ↓
OAuth Plugin
    ↓
Token Introspection → OAuth2 Server
    ↓
User Info Request → OAuth2 Server
    ↓
Cache Result
    ↓
Return AuthResult → MockForge
```

## 🧪 Testing

### Unit Tests

```bash
pytest tests/test_plugin.py -v
```

### Integration Tests

```bash
# Start plugin
python plugin.py &

# Test authentication
curl -X POST http://localhost:8080/plugin/authenticate \
  -H "Content-Type: application/json" \
  -d '{
    "context": {
      "method": "GET",
      "uri": "/api/users",
      "headers": {}
    },
    "credentials": {
      "type": "bearer",
      "token": "your-access-token"
    }
  }'
```

### Load Testing

```bash
# Install artillery
npm install -g artillery

# Run load test
artillery quick --count 100 --num 10 http://localhost:8080/plugin/authenticate
```

## 📊 Performance

- **Latency**: ~50-150ms (depends on auth server)
- **Throughput**: 100-500 req/sec (single instance)
- **Memory**: ~50MB per instance
- **Token Cache**: Reduces latency to ~5-10ms for cached tokens

### Optimization Tips

1. **Enable Caching**: Token cache significantly reduces latency
2. **Scale Horizontally**: Deploy multiple instances behind a load balancer
3. **Use Connection Pooling**: The `requests` library uses connection pooling by default
4. **Tune Timeouts**: Adjust timeout values based on your auth server latency
5. **Use Redis**: For distributed caching across instances

## 🔒 Security Considerations

### Best Practices

1. **Secret Management**:
   - Use Kubernetes secrets or HashiCorp Vault
   - Never hardcode credentials
   - Rotate secrets regularly

2. **Network Security**:
   - Use TLS for all communication
   - Deploy in private network when possible
   - Use service mesh (Istio, Linkerd) for mTLS

3. **Token Handling**:
   - Tokens are never logged
   - Cache is in-memory only (not persisted)
   - Tokens expire based on OAuth2 server settings

4. **Rate Limiting**:
   - Implement rate limiting at the load balancer
   - Monitor for unusual patterns
   - Alert on authentication failures

## 🐛 Troubleshooting

### Plugin Won't Start

```bash
# Check Python version
python --version  # Should be 3.9+

# Verify dependencies
pip list | grep mockforge-plugin

# Check environment variables
env | grep OAUTH
```

### Authentication Fails

```bash
# Test auth server directly
curl -X POST https://your-auth-server.com/oauth/introspect \
  -u "client_id:client_secret" \
  -d "token=your-token"

# Check plugin logs
docker logs mockforge-oauth-plugin

# Verify token format
echo "your-token" | base64 -d  # For JWT tokens
```

### High Latency

1. Check auth server response time
2. Enable token caching
3. Increase concurrent workers: `uvicorn plugin:app --workers 4`
4. Deploy closer to auth server

## 📈 Monitoring

### Metrics

The plugin exposes these endpoints:

- `GET /health` - Health check (returns 200 if healthy)
- `GET /metrics` - Prometheus metrics (optional, requires prometheus-client)

### Prometheus Integration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'mockforge-oauth-plugin'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

### Logging

Logs are structured JSON (via `python-json-logger`):

```json
{
  "timestamp": "2025-10-09T10:30:00Z",
  "level": "INFO",
  "message": "Successfully authenticated user: user123",
  "user_id": "user123",
  "latency_ms": 45
}
```

## 🤝 Contributing

See [CONTRIBUTING.md](../../../CONTRIBUTING.md) for development guidelines.

## 📄 License

MIT OR Apache-2.0

## 🔗 Resources

- [MockForge Documentation](https://docs.mockforge.dev)
- [OAuth2 RFC 7662 (Token Introspection)](https://tools.ietf.org/html/rfc7662)
- [Python Remote Plugin SDK](../../../sdk/python/mockforge_plugin/)
- [FastAPI Documentation](https://fastapi.tiangolo.com/)

## 💬 Support

- GitHub Issues: [Report bugs](https://github.com/mockforge/mockforge/issues)
- GitHub Discussions: [Ask questions](https://github.com/mockforge/mockforge/discussions)
- Discord: [Join community](https://discord.gg/mockforge)
