# Your First Mock API in 5 Minutes

**Scenario**: Your frontend team needs a `/users` API to continue development, but the backend isn't ready. Let's create a working mock in 5 minutes.

## Step 1: Install MockForge (30 seconds)

```bash
cargo install mockforge-cli
```

Or use the pre-built binary from the [releases page](https://github.com/SaaSy-Solutions/mockforge/releases).

## Step 2: Create a Simple Config (1 minute)

You can either create a config manually or use the `init` command:

```bash
# Option A: Use the init command (recommended)
mockforge init .

# This creates mockforge.yaml with sensible defaults
# Then edit it to match the config below

# Option B: Create manually
```

Create a file called `my-api.yaml` (or edit the generated `mockforge.yaml`):

```yaml
http:
  port: 3000
  routes:
    - path: /users
      method: GET
      response:
        status: 200
        body: |
          [
            {
              "id": "{{uuid}}",
              "name": "Alice Johnson",
              "email": "alice@example.com",
              "createdAt": "{{now}}"
            },
            {
              "id": "{{uuid}}",
              "name": "Bob Smith",
              "email": "bob@example.com",
              "createdAt": "{{now}}"
            }
          ]

    - path: /users/{id}
      method: GET
      response:
        status: 200
        body: |
          {
            "id": "{{request.path.id}}",
            "name": "Alice Johnson",
            "email": "alice@example.com",
            "createdAt": "{{now}}"
          }

    - path: /users
      method: POST
      response:
        status: 201
        body: |
          {
            "id": "{{uuid}}",
            "name": "{{request.body.name}}",
            "email": "{{request.body.email}}",
            "createdAt": "{{now}}"
          }
```

## Step 3: Validate Your Config (Optional but Recommended)

```bash
mockforge config validate --config my-api.yaml
```

You should see:
```
✅ Configuration is valid

📊 Summary:
   Found 3 HTTP routes
```

## Step 4: Start the Server (10 seconds)

```bash
mockforge serve --config my-api.yaml
```

You'll see:
```
MockForge v1.0.0 starting...
HTTP server listening on 0.0.0.0:3000
Ready to serve requests at http://localhost:3000
```

## Step 5: Test It (30 seconds)

Open a new terminal and test your endpoints:

```bash
# Get all users
curl http://localhost:3000/users

# Get a specific user
curl http://localhost:3000/users/123

# Create a new user
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Charlie Brown", "email": "charlie@example.com"}'
```

**What just happened?**
- `{{uuid}}` generates a unique ID each time
- `{{now}}` adds the current timestamp
- `{{request.path.id}}` captures the ID from the URL
- `{{request.body.name}}` reads data from POST requests

## Step 6: Enable Dynamic Data (1 minute)

Want different data each time? Enable template expansion:

```bash
# Stop the server (Ctrl+C), then restart:
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve --config my-api.yaml
```

Now every request returns unique UUIDs and timestamps!

## Step 7: Add the Admin UI (30 seconds)

Want to see requests in real-time?

```bash
mockforge serve --config my-api.yaml --admin --admin-port 9080
```

Open http://localhost:9080 in your browser to see:
- Live request logs
- API metrics
- Configuration controls

## What's Next?

**In the next 5 minutes**, you could:

1. **Use an OpenAPI Spec** instead of YAML routes:
   ```bash
   mockforge serve --spec your-api.json --admin
   ```

2. **Add a Plugin** for custom data generation:
   ```bash
   mockforge plugin install auth-jwt
   mockforge serve --config my-api.yaml --admin
   ```

3. **Mock a WebSocket** for real-time features:
   ```yaml
   websocket:
     port: 3001
     replay_file: chat-messages.jsonl
   ```

4. **Share with Your Team** using workspace sync:
   ```bash
   mockforge sync start --directory ./team-mocks
   git add team-mocks && git commit -m "Add user API mocks"
   ```

## Common Next Steps

| What You Need | Where to Go |
|---------------|-------------|
| OpenAPI/Swagger integration | [OpenAPI Guide](../user-guide/http-mocking/openapi.md) |
| More realistic fake data | [Dynamic Data Guide](../user-guide/http-mocking/dynamic-data.md) |
| WebSocket/real-time mocking | [WebSocket Guide](../user-guide/websocket-mocking.md) |
| gRPC service mocking | [gRPC Guide](../user-guide/grpc-mocking.md) |
| Custom authentication | [Security Guide](../user-guide/security.md) |
| Team collaboration | [Sync Guide](../user-guide/sync.md) |

## Troubleshooting

**Port already in use?**
```bash
mockforge serve --config my-api.yaml --http-port 8080
```

**Templates not working?**
Make sure you set `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true` or add it to your config:
```yaml
http:
  response_template_expand: true
```

**Config errors?**
```bash
# Validate your configuration
mockforge config validate --config my-api.yaml

# See all available options
# https://github.com/SaaSy-Solutions/mockforge/blob/main/config.template.yaml
```

**Need help?**
- Check the [Configuration Validation Guide](../reference/config-validation.md)
- Review the [Complete Config Template](https://github.com/SaaSy-Solutions/mockforge/blob/main/config.template.yaml)
- See [Troubleshooting Guide](../reference/troubleshooting.md)
- Check the [FAQ](../reference/faq.md)

---

**Congratulations!** You now have a working mock API that your frontend team can use immediately. The best part? As the real API evolves, just update your config file to match.
