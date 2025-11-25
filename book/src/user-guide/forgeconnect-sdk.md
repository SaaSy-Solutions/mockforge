# ForgeConnect SDK

ForgeConnect SDK provides browser extension and framework SDKs for capturing network traffic, auto-generating mocks, and integrating with popular frontend frameworks. Develop and test frontend applications with seamless mock integration.

## Overview

ForgeConnect includes:

- **Browser Extension**: Capture network traffic and create mocks automatically
- **Browser SDK**: JavaScript/TypeScript SDK for framework integration
- **Auto-Mock Generation**: Automatically create mocks for unhandled requests
- **Framework Adapters**: React, Vue, Angular, Next.js support
- **Auth Passthrough**: Support for OAuth flows and authentication

## Quick Start

### Install Browser Extension

1. Install from Chrome Web Store or Firefox Add-ons
2. Open browser DevTools
3. Navigate to "MockForge" tab
4. Connect to MockForge server

### Install Browser SDK

```bash
npm install @mockforge/forgeconnect
```

### Basic Usage

```typescript
import { ForgeConnect } from '@mockforge/forgeconnect';

// Initialize ForgeConnect
const forgeConnect = new ForgeConnect({
  serverUrl: 'http://localhost:3000',
  autoMock: true
});

// Start intercepting requests
forgeConnect.start();
```

## Browser Extension

### Features

- **Request Capture**: Automatically capture all network requests
- **Mock Creation**: Create mocks from captured requests with one click
- **DevTools Integration**: Full DevTools panel with React UI
- **Auto-Discovery**: Automatically discover MockForge server
- **Request Filtering**: Filter requests by URL, method, status
- **Live Response Modification**: Modify responses on-the-fly in DevTools
- **Persona/Scenario Toggling**: Switch personas and scenarios directly from DevTools
- **Reverse Injection**: Automatically inject mocks back into workspace
- **Snapshot Diff**: Compare mock behavior between environments

### DevTools Panel

The ForgeConnect extension adds a dedicated "MockForge" tab to your browser's DevTools with the following features:

#### Request Capture Tab

- **Live Request Monitoring**: See all fetch/XHR requests in real-time
- **Request Details**: View request method, URL, headers, body, and response
- **Filter & Search**: Filter requests by URL pattern, method, or status code
- **One-Click Mock Creation**: Click "Mock this endpoint" to create a mock from any request

#### Mocks Management Tab

- **View All Mocks**: See all mocks in your MockForge workspace
- **Edit Mocks**: Modify mock responses directly in DevTools
- **Delete Mocks**: Remove mocks with one click
- **Toggle Mocks**: Enable/disable mocks without leaving the browser

#### Mock Preview Tab

- **Create/Edit Mocks**: Visual interface for creating and editing mocks
- **Response Editor**: JSON editor with syntax highlighting
- **Status Code Selection**: Choose response status codes
- **Headers Configuration**: Add/modify response headers
- **Save to Workspace**: Automatically reverse-inject mocks into your workspace

#### X-Ray Tab

- **Request Analysis**: Deep dive into request/response details
- **Timing Information**: View request latency and timing breakdown
- **Header Inspection**: Inspect all request and response headers
- **Body Analysis**: View and analyze request/response bodies

#### Snapshot Diff Tab

- **Environment Comparison**: Compare mock behavior between test and prod
- **Persona Comparison**: Compare responses for different personas
- **Reality Level Comparison**: Compare behavior at different reality levels
- **Side-by-Side Visualization**: See differences highlighted side-by-side

### Usage

1. **Open DevTools**: Press F12 or right-click → Inspect
2. **Navigate to MockForge Tab**: Click "MockForge" in DevTools
3. **Connect to Server**: Enter MockForge server URL or use auto-discovery
4. **Capture Requests**: Requests are automatically captured
5. **Create Mocks**: Click "Mock this endpoint" on any captured request

### "Mock this Endpoint" Feature

The "Mock this endpoint" button appears on every captured request:

1. **Click "Mock this endpoint"**: Opens the mock preview panel
2. **Review Request Details**: See the original request method, path, headers, and body
3. **Edit Response**: Modify the response status, headers, and body
4. **Save Mock**: Click "Save" to create the mock in your workspace
5. **Auto-Injection**: The mock is automatically reverse-injected into your MockForge workspace

**Example Workflow:**
```
1. Make API call: GET /api/users/123
2. Request appears in DevTools "Captured Requests" tab
3. Click "Mock this endpoint" button
4. Edit response in preview panel
5. Click "Save" → Mock created in workspace
6. Future requests to /api/users/123 use the mock
```

### Live Response Modification

Modify responses on-the-fly without leaving the browser:

1. **Select a Mock**: Click on a mock in the "Mocks" tab
2. **Edit Response**: Modify the response JSON directly
3. **Save Changes**: Changes are immediately applied
4. **Test**: Refresh the page to see the new response

### Persona/Scenario Toggling

Switch personas and scenarios directly from DevTools:

1. **Open Mocks Tab**: Navigate to the "Mocks" tab
2. **Select Mock**: Click on a mock to view details
3. **Change Persona**: Use the persona dropdown to switch personas
4. **Change Scenario**: Use the scenario dropdown to switch scenarios
5. **Apply**: Changes are immediately applied to the mock

### Reverse Injection into Workspace

When you create or modify a mock in DevTools, it's automatically reverse-injected into your MockForge workspace:

- **Automatic Sync**: Changes sync to your workspace immediately
- **Workspace Integration**: Mocks appear in your workspace configuration
- **Version Control**: Mocks can be committed to version control
- **Team Sharing**: Other team members see the mocks in shared workspaces

### Auto-Mock Generation

When a request fails or returns an error, ForgeConnect can automatically create a mock:

```typescript
const forgeConnect = new ForgeConnect({
  serverUrl: 'http://localhost:3000',
  autoMock: true,
  autoMockOnError: true  // Create mock on 4xx/5xx errors
});
```

## Browser SDK

### Installation

```bash
npm install @mockforge/forgeconnect
```

### Basic Setup

```typescript
import { ForgeConnect } from '@mockforge/forgeconnect';

const forgeConnect = new ForgeConnect({
  serverUrl: 'http://localhost:3000',
  autoMock: true,
  interceptFetch: true,
  interceptXHR: true
});

// Start intercepting
forgeConnect.start();
```

### Framework Adapters

#### React

```typescript
import { useForgeConnect } from '@mockforge/forgeconnect/react';

function App() {
  const { isConnected, mocks } = useForgeConnect({
    serverUrl: 'http://localhost:3000'
  });

  return (
    <div>
      {isConnected ? 'Connected' : 'Disconnected'}
      <ul>
        {mocks.map(mock => (
          <li key={mock.id}>{mock.path}</li>
        ))}
      </ul>
    </div>
  );
}
```

#### Vue

```typescript
import { useForgeConnect } from '@mockforge/forgeconnect/vue';

export default {
  setup() {
    const { isConnected, mocks } = useForgeConnect({
      serverUrl: 'http://localhost:3000'
    });

    return { isConnected, mocks };
  }
};
```

#### Next.js

```typescript
// pages/_app.tsx
import { ForgeConnectProvider } from '@mockforge/forgeconnect/next';

function MyApp({ Component, pageProps }) {
  return (
    <ForgeConnectProvider serverUrl="http://localhost:3000">
      <Component {...pageProps} />
    </ForgeConnectProvider>
  );
}
```

### Request Interception

ForgeConnect intercepts both `fetch` and `XMLHttpRequest`:

```typescript
const forgeConnect = new ForgeConnect({
  serverUrl: 'http://localhost:3000',
  interceptFetch: true,
  interceptXHR: true
});

// All fetch requests are intercepted
fetch('/api/users')
  .then(response => response.json())
  .then(data => console.log(data));

// All XHR requests are intercepted
const xhr = new XMLHttpRequest();
xhr.open('GET', '/api/users');
xhr.send();
```

### Mock Management

#### List Mocks

```typescript
const mocks = await forgeConnect.listMocks();
console.log('Available mocks:', mocks);
```

#### Create Mock

```typescript
const mock = await forgeConnect.createMock({
  method: 'GET',
  path: '/api/users',
  response: {
    status: 200,
    body: { users: [] }
  }
});
```

#### Update Mock

```typescript
await forgeConnect.updateMock(mockId, {
  response: {
    status: 200,
    body: { users: [{ id: 1, name: 'Alice' }] }
  }
});
```

#### Delete Mock

```typescript
await forgeConnect.deleteMock(mockId);
```

## Auth Passthrough

ForgeConnect supports OAuth flows and authentication:

```typescript
const forgeConnect = new ForgeConnect({
  serverUrl: 'http://localhost:3000',
  authPassthrough: true,
  authPaths: ['/auth', '/oauth', '/login']
});
```

Requests to auth paths are passed through to the real server without interception.

## Configuration

### SDK Configuration

```typescript
interface ForgeConnectConfig {
  serverUrl: string;
  autoMock?: boolean;
  autoMockOnError?: boolean;
  interceptFetch?: boolean;
  interceptXHR?: boolean;
  authPassthrough?: boolean;
  authPaths?: string[];
  mockPaths?: string[];
  excludePaths?: string[];
}
```

### Extension Configuration

Configure via extension options:

1. Right-click extension icon
2. Select "Options"
3. Configure server URL and settings

## Use Cases

### Frontend Development

Develop frontend without backend:

```typescript
// Start ForgeConnect
const forgeConnect = new ForgeConnect({
  serverUrl: 'http://localhost:3000',
  autoMock: true
});

forgeConnect.start();

// Develop frontend - mocks created automatically
```

### API Testing

Test API integration:

```typescript
// Capture real API calls
const forgeConnect = new ForgeConnect({
  serverUrl: 'http://localhost:3000',
  autoMock: false  // Don't auto-create, capture only
});

// Review captured requests
const captures = await forgeConnect.getCaptures();

// Create mocks from captures
for (const capture of captures) {
  await forgeConnect.createMockFromCapture(capture);
}
```

### Debugging

Debug API issues:

```typescript
// Enable detailed logging
const forgeConnect = new ForgeConnect({
  serverUrl: 'http://localhost:3000',
  debug: true
});

// View intercepted requests in console
forgeConnect.on('request', (request) => {
  console.log('Intercepted:', request);
});
```

## Best Practices

1. **Use Auto-Mock Sparingly**: Only enable for development
2. **Filter Requests**: Use `mockPaths` and `excludePaths` to control interception
3. **Auth Passthrough**: Always enable for authentication flows
4. **Version Control Mocks**: Export and commit mocks to version control
5. **Test with Real APIs**: Periodically test against real APIs

## Troubleshooting

### Extension Not Connecting

- Verify MockForge server is running
- Check server URL is correct
- Review browser console for errors

### Requests Not Intercepted

- Verify interception is enabled
- Check request paths match configuration
- Review SDK logs for errors

### Mocks Not Working

- Verify mock is created correctly
- Check mock path matches request path
- Review MockForge server logs

## Related Documentation

- [Browser Proxy Mode](advanced-behavior.md#browser-proxy-with-conditional-forwarding) - Proxy mode features
- [Configuration Guide](../configuration/files.md) - Complete configuration reference
- [SDK Documentation](../../sdk/README.md) - Complete SDK reference

