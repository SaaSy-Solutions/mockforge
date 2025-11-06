# ForgeConnect - Browser SDK for MockForge

ForgeConnect enables front-end developers to mock API endpoints directly from the browser without running a local MockForge server. It intercepts network requests, connects to MockForge (local or remote), and provides one-click mock creation.

## Features

- 🔍 **Automatic Request Interception** - Captures fetch and XMLHttpRequest calls
- 🚀 **Auto-Discovery** - Automatically finds MockForge server on localhost
- ⚡ **One-Click Mock Creation** - Create mocks from failed requests instantly
- 🎯 **Configurable Behavior** - Auto-create, prompt, or hybrid modes
- 🔌 **Framework Integrations** - Works with React Query, Next.js, Vue.js, Angular, and vanilla JS
- 📦 **Zero Dependencies** - Lightweight browser SDK
- 🔄 **Service Worker Support** - Comprehensive request capture via Service Worker
- 🌐 **WebSocket Support** - Real-time updates from MockForge server

## Installation

```bash
npm install @mockforge/forgeconnect
# or
yarn add @mockforge/forgeconnect
# or
pnpm add @mockforge/forgeconnect
```

## Quick Start

### Basic Usage

```typescript
import { ForgeConnect } from '@mockforge/forgeconnect';

// Initialize ForgeConnect
const forgeConnect = new ForgeConnect({
  // Auto-discover MockForge on localhost:3000
  // Or specify: serverUrl: 'http://localhost:3000'
  mockMode: 'auto', // 'auto' | 'prompt' | 'hybrid'
});

// Connect and start intercepting
await forgeConnect.initialize();
```

### Auto-Create Mocks for Failed Requests

```typescript
const forgeConnect = new ForgeConnect({
  mockMode: 'auto',
  autoMockStatusCodes: [404, 500, 502, 503, 504],
  autoMockNetworkErrors: true,
});

await forgeConnect.initialize();
// Now all failed requests will automatically create mocks!
```

### Prompt Mode (User Confirmation)

```typescript
const forgeConnect = new ForgeConnect({
  mockMode: 'prompt',
  promptMockCreation: async (request) => {
    // Show your custom UI prompt
    return confirm(`Create mock for ${request.method} ${request.path}?`);
  },
});

await forgeConnect.initialize();
```

### Hybrid Mode (Auto + Prompt)

```typescript
const forgeConnect = new ForgeConnect({
  mockMode: 'hybrid',
  // Auto-create for failed requests
  autoMockStatusCodes: [404, 500],
  autoMockNetworkErrors: true,
  // Prompt for successful requests
  promptMockCreation: async (request) => {
    return confirm(`Create mock for successful request ${request.method} ${request.path}?`);
  },
});

await forgeConnect.initialize();
```

### With Service Worker (Comprehensive Capture)

```typescript
const forgeConnect = new ForgeConnect({
  enableServiceWorker: true, // Enable Service Worker for all requests
  mockMode: 'auto',
});

await forgeConnect.initialize();
```

### With WebSocket (Real-Time Updates)

```typescript
const forgeConnect = new ForgeConnect({
  enableWebSocket: true, // Enable WebSocket for real-time updates
  serverUrl: 'http://localhost:3000',
  onMockCreated: (mock) => {
    console.log('Mock created via WebSocket:', mock);
  },
});

await forgeConnect.initialize();
```

## API Reference

### ForgeConnect

Main SDK class for intercepting requests and creating mocks.

#### Constructor

```typescript
new ForgeConnect(config?: ForgeConnectConfig)
```

**Config Options:**

- `serverUrl?: string` - MockForge server URL (auto-discovered if not provided)
- `discoveryPorts?: number[]` - Ports to try for auto-discovery (default: [3000, 3001, 8080, 9080])
- `mockMode?: 'auto' | 'prompt' | 'hybrid'` - Mock creation behavior (default: 'hybrid')
- `autoMockStatusCodes?: number[]` - HTTP status codes that trigger auto-mock (default: [404, 500, 502, 503, 504])
- `autoMockNetworkErrors?: boolean` - Auto-mock network errors (default: true)
- `enableLogging?: boolean` - Enable console logging (default: true)
- `enableServiceWorker?: boolean` - Enable Service Worker for comprehensive capture (default: true if supported)
- `enableWebSocket?: boolean` - Enable WebSocket for real-time updates (default: false)
- `onMockCreated?: (mock: MockConfig) => void` - Callback when mock is created
- `onConnectionChange?: (connected: boolean, url?: string) => void` - Callback when connection status changes
- `promptMockCreation?: (request: CapturedRequest) => Promise<boolean>` - Custom prompt function

#### Methods

##### `initialize(): Promise<boolean>`

Initialize ForgeConnect and connect to MockForge. Returns `true` if connection successful.

```typescript
const connected = await forgeConnect.initialize();
if (!connected) {
  console.error('Failed to connect to MockForge');
}
```

##### `createMockFromRequest(request: CapturedRequest): Promise<MockConfig>`

Create a mock from a captured request.

```typescript
const mock = await forgeConnect.createMockFromRequest(capturedRequest);
```

##### `createMock(mock: MockConfig): Promise<MockConfig>`

Manually create a mock.

```typescript
const mock = await forgeConnect.createMock({
  name: 'GET /api/users',
  method: 'GET',
  path: '/api/users',
  response: {
    body: { users: [] },
  },
});
```

##### `listMocks(): Promise<MockConfig[]>`

List all mocks from MockForge.

```typescript
const mocks = await forgeConnect.listMocks();
```

##### `getMock(id: string): Promise<MockConfig>`

Get a specific mock by ID.

```typescript
const mock = await forgeConnect.getMock('mock-id');
```

##### `updateMock(id: string, mock: MockConfig): Promise<MockConfig>`

Update an existing mock.

```typescript
const updated = await forgeConnect.updateMock('mock-id', {
  ...mock,
  enabled: false,
});
```

##### `deleteMock(id: string): Promise<void>`

Delete a mock.

```typescript
await forgeConnect.deleteMock('mock-id');
```

##### `getConnectionStatus(): ConnectionStatus`

Get current connection status.

```typescript
const status = forgeConnect.getConnectionStatus();
console.log(status.connected, status.url);
```

##### `reconnect(): Promise<boolean>`

Reconnect to MockForge.

```typescript
const reconnected = await forgeConnect.reconnect();
```

##### `start(): void`

Start intercepting requests (if already initialized).

##### `stop(): void`

Stop intercepting requests.

## Framework Integrations

### React Query

```typescript
import { ForgeConnect } from '@mockforge/forgeconnect';
import { useQuery } from '@tanstack/react-query';

const forgeConnect = new ForgeConnect({
  mockMode: 'auto',
});

// Initialize in your app
useEffect(() => {
  forgeConnect.initialize();
}, []);

// React Query will automatically use mocks for failed requests
const { data } = useQuery({
  queryKey: ['users'],
  queryFn: () => fetch('/api/users').then(r => r.json()),
});
```

### Next.js

```typescript
// app/layout.tsx or pages/_app.tsx
import { ForgeConnect } from '@mockforge/forgeconnect';
import { useEffect } from 'react';

export default function App({ Component, pageProps }) {
  useEffect(() => {
    if (process.env.NODE_ENV === 'development') {
      const forgeConnect = new ForgeConnect({
        serverUrl: process.env.NEXT_PUBLIC_MOCKFORGE_URL || 'http://localhost:3000',
        mockMode: 'auto',
      });
      forgeConnect.initialize();
    }
  }, []);

  return <Component {...pageProps} />;
}
```

### Vanilla JavaScript

```html
<script type="module">
  import { ForgeConnect } from '@mockforge/forgeconnect';

  const forgeConnect = new ForgeConnect({
    mockMode: 'auto',
  });

  await forgeConnect.initialize();

  // All fetch and XHR requests are now intercepted
  fetch('/api/users')
    .then(r => r.json())
    .then(data => console.log(data));
</script>
```

### Vue.js

```vue
<script setup>
import { useForgeConnect } from '@mockforge/forgeconnect/adapters/vue';

const { forgeConnect, connected } = useForgeConnect({
  mockMode: 'auto',
});
</script>

<template>
  <div>
    <p v-if="connected">Connected to MockForge</p>
  </div>
</template>
```

### Angular

```typescript
// app.module.ts or standalone component
import { provideForgeConnect } from '@mockforge/forgeconnect/adapters/angular';

@NgModule({
  providers: [
    provideForgeConnect({ mockMode: 'auto' })
  ]
})
export class AppModule {}
```

Or use the injectable service:

```typescript
import { Injectable } from '@angular/core';
import { ForgeConnectService } from '@mockforge/forgeconnect/adapters/angular';

@Injectable({
  providedIn: 'root'
})
export class MyService {
  constructor(private forgeConnect: ForgeConnectService) {
    // Service auto-initializes
  }
}
```

## Types

### MockConfig

```typescript
interface MockConfig {
  id?: string;
  name: string;
  method: string;
  path: string;
  response: MockResponse;
  enabled?: boolean;
  latency_ms?: number;
  status_code?: number;
}
```

### CapturedRequest

```typescript
interface CapturedRequest {
  method: string;
  url: string;
  path: string;
  queryParams?: Record<string, string>;
  headers?: Record<string, string>;
  body?: any;
  statusCode?: number;
  responseBody?: any;
  responseHeaders?: Record<string, string>;
  error?: {
    type: 'network' | 'timeout' | 'cors' | 'http';
    message: string;
  };
  timestamp: number;
}
```

## Examples

See the `examples/` directory for complete working examples:

- React app with React Query
- Next.js app
- Vanilla JavaScript app

## Requirements

- MockForge server running (local or remote)
- Modern browser with fetch API support
- TypeScript 5.0+ (for TypeScript projects)

## License

MIT License - see [LICENSE](../../LICENSE-MIT) for details.

