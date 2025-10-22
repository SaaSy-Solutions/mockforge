# MockForge UI Builder - Quick Start Guide

Get started with the MockForge UI Builder in 5 minutes!

## Prerequisites

- Node.js 18+ installed
- MockForge source code cloned

## Step 1: Install Dependencies (1 min)

```bash
cd ui-builder/frontend
npm install
```

## Step 2: Start Development Server (30 seconds)

```bash
npm run dev
```

The UI will open at: **http://localhost:5173**

## Step 3: Create Your First Endpoint (2 min)

### Example: Simple REST API

1. Click **"New Endpoint"** button
2. Fill in basic info:
   - **Name**: "Get User"
   - **Description**: "Returns user data"
   - **Enabled**: ✅ (checked)

3. Select **HTTP/REST** protocol

4. Configure the endpoint:
   - **Method**: GET
   - **Path**: `/api/users/123`

5. Set the response:
   - **Status Code**: 200
   - **Body Type**: Click "Template"
   - **Template**:
     ```json
     {
       "id": "{{uuid}}",
       "name": "{{faker.name}}",
       "email": "{{faker.email}}",
       "createdAt": "{{now}}"
     }
     ```

6. Click **Save**

🎉 Done! Your first mock endpoint is created.

## Step 4: Add Chaos Engineering (Optional, 1 min)

Make it more realistic by adding latency and failures:

1. In the endpoint editor, scroll to **"Behavior & Chaos Engineering"**
2. Click **"Show"**
3. Check **"Add Latency"**:
   - Base: 100ms
   - Jitter: 50ms
4. Check **"Add Failures"**:
   - Error Rate: 0.05 (5% failure rate)
5. Click **Save**

Now your endpoint will:
- Respond in 50-150ms (realistic latency)
- Fail 5% of the time (simulates real-world errors)

## What's Next?

### Create a gRPC Endpoint

1. Click **"New Endpoint"**
2. Select **gRPC**
3. Configure:
   - **Service**: `UserService`
   - **Method**: `GetUser`
   - **Proto File**: `user.proto`
   - **Response**:
     ```json
     {
       "id": 1,
       "name": "Alice",
       "email": "alice@example.com"
     }
     ```
4. Save

### Create a WebSocket Endpoint

1. Click **"New Endpoint"**
2. Select **WebSocket**
3. Configure:
   - **Path**: `/ws/chat`
   - **On Connect**: Check "Send message on connect"
     ```json
     {"message": "Welcome to chat!"}
     ```
   - **On Message**: Select "Echo back"
4. Save

### Export Your Configuration

1. Go to **Config** page
2. Click **"Export"**
3. Downloads `mockforge-config.yaml`
4. Use it with MockForge CLI:
   ```bash
   mockforge serve --config mockforge-config.yaml
   ```

## Tips & Tricks

### Template Variables

Use these in your response templates:

| Variable | Example Output |
|----------|---------------|
| `{{uuid}}` | `550e8400-e29b-41d4-a716-446655440000` |
| `{{now}}` | `2024-10-22T10:30:00Z` |
| `{{rand.int}}` | `42` |
| `{{rand.float}}` | `3.14159` |
| `{{faker.name}}` | `John Doe` |
| `{{faker.email}}` | `john@example.com` |
| `{{faker.address}}` | `123 Main St` |
| `{{params.id}}` | (from URL path) |
| `{{query.filter}}` | (from query string) |

### Keyboard Shortcuts

In the Monaco editor:
- `Ctrl/Cmd + F`: Find
- `Ctrl/Cmd + H`: Replace
- `Ctrl/Cmd + /`: Comment
- `Ctrl/Cmd + D`: Select next occurrence
- `Alt + Click`: Multiple cursors

### Response Body Types

1. **Static**: Fixed JSON response (best for simple cases)
2. **Template**: Dynamic with variables (most flexible)
3. **Faker**: Schema-based fake data (realistic data)
4. **AI**: Prompt-based generation (experimental)

## Troubleshooting

### Port 5173 already in use?

Change the port in `vite.config.ts`:
```typescript
export default defineConfig({
  server: {
    port: 5174, // Change this
  },
})
```

### API errors?

Make sure MockForge server is running:
```bash
mockforge serve --admin-enabled --admin-port 9080
```

### Monaco editor not loading?

Clear browser cache and reload (Ctrl/Cmd + Shift + R)

## Need Help?

- 📖 [Full Documentation](README.md)
- 🐛 [Report Issues](https://github.com/mockforge/mockforge/issues)
- 💬 [Discussions](https://github.com/mockforge/mockforge/discussions)

## Production Build

Ready to deploy?

```bash
npm run build
```

Output is in `dist/` folder. Serve with any static file server.

---

**That's it!** You now know how to:
- ✅ Create HTTP endpoints
- ✅ Use templates with variables
- ✅ Add chaos engineering
- ✅ Export configurations
- ✅ Build for production

Happy mocking! 🚀
