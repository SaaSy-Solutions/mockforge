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

### Usage

1. **Open DevTools**: Press F12 or right-click → Inspect
2. **Navigate to MockForge Tab**: Click "MockForge" in DevTools
3. **Connect to Server**: Enter MockForge server URL or use auto-discovery
4. **Capture Requests**: Requests are automatically captured
5. **Create Mocks**: Click "Create Mock" on any captured request

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

