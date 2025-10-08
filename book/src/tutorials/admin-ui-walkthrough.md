# Admin UI Walkthrough

**Goal**: Use MockForge's Admin UI to visually manage your mock server, view live logs, and configure settings without editing files.

**Time**: 5 minutes

## What You'll Learn

- Access the Admin UI
- View real-time request logs
- Monitor server metrics
- Manage fixtures with drag-and-drop
- Configure latency and fault injection
- Search and filter logs

## Prerequisites

- MockForge installed and running
- A basic understanding of MockForge concepts

## Step 1: Start MockForge with Admin UI

You can run the Admin UI in two modes:

### Standalone Mode (Separate Port)

```bash
mockforge serve --admin --admin-port 9080 --http-port 3000
```

Access at: **http://localhost:9080**

### Embedded Mode (Under HTTP Server)

```bash
mockforge serve --admin-embed --admin-mount-path /admin --http-port 3000
```

Access at: **http://localhost:3000/admin**

For this tutorial, we'll use standalone mode for simplicity.

## Step 2: Access the Dashboard

Open your browser and navigate to **http://localhost:9080**.

You'll see the **Dashboard** with:

### Server Status Section
- **HTTP Server**: Running on port 3000
- **WebSocket Server**: Status and port
- **gRPC Server**: Status and port
- **Uptime**: How long the server has been running

### Quick Stats
- **Total Requests**: Request counter
- **Active Connections**: Current open connections
- **Average Response Time**: Performance metrics
- **Error Rate**: Failed requests percentage

### Recent Activity
- Last 10 requests with timestamps, methods, paths, and status codes

## Step 3: View Live Logs

Click on the **"Logs"** tab in the navigation.

### Features:
- **Real-time updates**: Logs stream via Server-Sent Events (SSE)
- **Color-coded levels**: INFO (blue), WARN (yellow), ERROR (red)
- **Request details**: Method, path, status code, response time
- **Search**: Filter logs by keyword
- **Auto-scroll**: Automatically scroll to newest logs

### Try It:
1. Keep the logs tab open
2. In another terminal, send a request:
   ```bash
   curl http://localhost:3000/users
   ```
3. Watch the log appear instantly in the UI!

### Log Search
Use the search box to filter:
- Search by path: `/users`
- Search by method: `POST`
- Search by status: `404`
- Search by error message: `validation failed`

## Step 4: Explore Metrics

Click on the **"Metrics"** tab.

### Available Metrics:
- **Request Rate**: Requests per second over time
- **Response Times**: P50, P95, P99 latencies
- **Status Code Distribution**: 2xx, 4xx, 5xx breakdown
- **Endpoint Performance**: Slowest endpoints
- **Error Trends**: Error rates over time

### Use Cases:
- **Performance testing**: Monitor response times under load
- **Debugging**: Identify which endpoints are failing
- **Capacity planning**: See throughput limits

## Step 5: Manage Fixtures

Click on the **"Fixtures"** tab.

### What are Fixtures?
Fixtures are saved mock scenarios - collections of requests and expected responses for testing.

### Tree View Interface:
```
📁 Fixtures
  📁 User Management
    ✅ Create User - Happy Path
    ✅ Create User - Validation Error
    ✅ Get User - Not Found
  📁 Order Processing
    ✅ Create Order
    ✅ Update Order Status
```

### Actions:
1. **Drag and Drop**: Reorganize fixtures into folders
2. **Run Fixture**: Test a specific scenario
3. **Run Folder**: Execute all fixtures in a folder
4. **Export**: Download fixtures as JSON
5. **Import**: Upload fixture collections

### Try It:
1. Click **"New Fixture"**
2. Name it: "Test User Creation"
3. Configure:
   - **Method**: POST
   - **Path**: `/users`
   - **Expected Status**: 201
   - **Request Body**:
     ```json
     {"name": "Test User", "email": "test@example.com"}
     ```
4. Click **"Save"**
5. Click **"Run"** to test it

## Step 6: Configure Latency Simulation

Click on the **"Configuration"** tab, then **"Latency"**.

### Latency Profiles:
MockForge can simulate various network conditions:

| Profile | Description | Latency |
|---------|-------------|---------|
| **None** | No artificial delay | 0ms |
| **Fast** | Local network | 10-30ms |
| **Normal** | Good internet | 50-150ms |
| **Slow** | Poor connection | 300-800ms |
| **Very Slow** | Bad mobile | 1000-3000ms |

### Configure:
1. Select **"Slow"** profile
2. Click **"Apply"**
3. Test an endpoint:
   ```bash
   time curl http://localhost:3000/users
   ```
4. Notice the delay!

### Per-Endpoint Latency:
You can also configure latency for specific endpoints:

```yaml
# In your config file
http:
  latency:
    enabled: true
    default_profile: normal
    endpoint_overrides:
      "POST /orders": slow       # Simulate slow order processing
      "GET /products": fast      # Fast product catalog
```

## Step 7: Enable Fault Injection

Still in the **"Configuration"** tab, click **"Fault Injection"**.

### Fault Types:
- **Random Failures**: Randomly return 500 errors
- **Timeouts**: Simulate request timeouts
- **Malformed Responses**: Return invalid JSON
- **Connection Drops**: Close connections unexpectedly

### Configure:
1. **Enable Fault Injection**: Toggle ON
2. **Error Rate**: Set to 20% (1 in 5 requests fails)
3. **Fault Type**: Select "Random Failures"
4. Click **"Apply"**

### Test It:
```bash
# Run this multiple times - some will fail!
for i in {1..10}; do
  curl http://localhost:3000/users
  echo ""
done
```

You'll see some requests return 500 errors, simulating an unreliable backend.

## Step 8: Search Across Services

Click on the **"Search"** tab.

### Full-Text Search:
Search across:
- Service names
- Endpoint paths
- Request/response bodies
- Log messages
- Configuration values

### Try It:
1. Search for `users` - finds all user-related endpoints
2. Search for `POST` - finds all POST endpoints
3. Search for `validation` - finds validation errors in logs

## Step 9: Proxy Configuration (Advanced)

Click **"Configuration"** → **"Proxy"**.

### Hybrid Mode:
MockForge can act as a proxy, forwarding unknown requests to a real backend:

1. **Enable Proxy**: Toggle ON
2. **Target URL**: `https://api.example.com`
3. **Fallback Mode**: "Forward unknown requests"
4. Click **"Apply"**

Now:
- Mocked endpoints return mock data
- Unknown endpoints are forwarded to the real API
- Perfect for gradual migration!

## Common Workflows

### Workflow 1: Debug a Failing Test
1. Open **Logs** tab
2. Enable **"Error Only"** filter
3. Run your failing test
4. Find the error in real-time
5. Copy the request details
6. Fix your test or mock configuration

### Workflow 2: Create Test Fixtures
1. Run your application manually (e.g., click through the UI)
2. Admin UI captures all requests in **Logs**
3. Click **"Save as Fixture"** on interesting requests
4. Organize fixtures into folders
5. Run fixtures as smoke tests before deployment

### Workflow 3: Performance Testing
1. Clear metrics (**Metrics** → **"Reset"**)
2. Run load test against MockForge
3. Monitor **Metrics** tab in real-time
4. Identify performance bottlenecks
5. Adjust mock configuration for better performance

### Workflow 4: Demo Preparation
1. **Fixtures**: Create realistic demo scenarios
2. **Latency**: Set to "Fast" for smooth demos
3. **Fault Injection**: Disable to prevent unexpected errors
4. **Logs**: Keep open to show real-time activity

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+K` | Open search |
| `Ctrl+L` | Jump to logs |
| `Ctrl+M` | Jump to metrics |
| `Ctrl+R` | Refresh dashboard |
| `Esc` | Close modals |

## Troubleshooting

**Admin UI not loading?**
- Check that the admin port (9080) isn't blocked
- Verify MockForge is running with `--admin` flag
- Check browser console for JavaScript errors

**Logs not updating?**
- Ensure Server-Sent Events (SSE) aren't blocked by your browser or proxy
- Try refreshing the page
- Check that `/__mockforge/logs` endpoint is accessible

**Fixtures not saving?**
- Verify you have write permissions to the MockForge data directory
- Check disk space availability
- Review logs for error messages

## What's Next?

- [Custom Response Configuration](../user-guide/http-mocking/custom-responses.md) - Build advanced mock responses
- [Security Features](../user-guide/security.md) - Add authentication to Admin UI (v1.1+)
- [Workspace Sync](../user-guide/sync.md) - Share fixtures with your team
- [Plugin System](../user-guide/plugins.md) - Extend Admin UI functionality

---

**Pro Tip**: Use browser bookmarks for quick access:
- `http://localhost:9080/` - Dashboard
- `http://localhost:9080/?tab=logs` - Jump directly to logs
- `http://localhost:9080/?tab=metrics` - Jump directly to metrics
