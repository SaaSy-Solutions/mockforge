# UI/UX Improvements Guide

This document details all identified UI/UX issues in the MockForge frontend with specific remediation steps.

---

## Summary

| Category | Count | Priority |
|----------|-------|----------|
| TypeScript Type Safety | 50+ | High |
| Form Validation | 20+ | High |
| Accessibility (a11y) | 30+ | High |
| Error Handling | 10 | Medium |
| Loading States | 5 | Medium |
| Display Bugs | 4 | Medium |
| Missing Features | 10 | Low |

---

## 1. TypeScript Type Safety

### 1.1 Replace `any` Types in API Layer

**File:** `ui-builder/frontend/src/lib/api.ts`

#### Current (Lines 353-365)
```typescript
export interface ServerConfig {
  http?: any;
  grpc?: any;
  websocket?: any;
  graphql?: any;
  mqtt?: any;
  smtp?: any;
  kafka?: any;
  amqp?: any;
}
```

#### Fix - Create Proper Types
```typescript
// types/protocols.ts
export interface HttpEndpointConfig {
  path: string;
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE' | 'HEAD' | 'OPTIONS';
  response: {
    status: number;
    headers?: Record<string, string>;
    body?: unknown;
    bodyType: 'json' | 'text' | 'xml' | 'html' | 'binary';
  };
  behavior?: {
    latency?: { min: number; max: number };
    failureRate?: number;
  };
}

export interface GrpcEndpointConfig {
  service: string;
  method: string;
  protoFile?: string;
  requestType: string;
  responseType: string;
  response: {
    body: unknown;
    metadata?: Record<string, string>;
  };
}

export interface MqttEndpointConfig {
  topic: string;
  qos: 0 | 1 | 2;
  retained?: boolean;
  payload: {
    type: 'json' | 'text' | 'binary';
    content: unknown;
  };
}

export interface SmtpEndpointConfig {
  port?: number;
  hostname?: string;
  tls?: boolean;
  authentication?: {
    required: boolean;
    users?: Array<{ username: string; password: string }>;
  };
  messageHandling: {
    storageEnabled: boolean;
    maxMessages: number;
  };
}

export interface WebSocketEndpointConfig {
  path: string;
  protocol?: string;
  messageHandlers: Array<{
    pattern: string;
    response: unknown;
  }>;
}

export interface GraphqlEndpointConfig {
  path: string;
  schema: string;
  resolvers: Array<{
    operationType: 'query' | 'mutation' | 'subscription';
    operationName: string;
    response: unknown;
  }>;
}

// Updated ServerConfig
export interface ServerConfig {
  http?: HttpEndpointConfig[];
  grpc?: GrpcEndpointConfig[];
  websocket?: WebSocketEndpointConfig[];
  graphql?: GraphqlEndpointConfig[];
  mqtt?: MqttEndpointConfig[];
  smtp?: SmtpEndpointConfig;
  kafka?: KafkaEndpointConfig[];
  amqp?: AmqpEndpointConfig[];
}
```

### 1.2 Fix Form Component Props

**Files:** All `*EndpointForm.tsx` components

#### Current
```typescript
interface HttpEndpointFormProps {
  config: any;
  onChange: (config: any) => void;
}
```

#### Fix
```typescript
import { HttpEndpointConfig } from '@/types/protocols';

interface HttpEndpointFormProps {
  config: Partial<HttpEndpointConfig>;
  onChange: (config: HttpEndpointConfig) => void;
}
```

### 1.3 Remove Type Casting

**Locations:**
- `HttpEndpointForm.tsx:183` - `setBodyType(type.id as any)`
- `MqttEndpointForm.tsx:184` - `setPayloadType(type.id as any)`
- `GraphqlEndpointForm.tsx:247,282` - Multiple `as any` casts
- `EndpointBuilder.tsx:61` - `data as any`

#### Fix
Use proper discriminated unions and type guards instead of casting.

---

## 2. Form Validation

### 2.1 HttpEndpointForm Validation

**File:** `ui-builder/frontend/src/components/HttpEndpointForm.tsx`

```typescript
// Add validation schema
import { z } from 'zod';

const httpEndpointSchema = z.object({
  path: z.string()
    .min(1, 'Path is required')
    .regex(/^\//, 'Path must start with /')
    .regex(/^[a-zA-Z0-9\/_\-{}:]+$/, 'Path contains invalid characters'),
  method: z.enum(['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS']),
  response: z.object({
    status: z.number().min(100).max(599),
    headers: z.record(z.string()).optional(),
    body: z.unknown().optional(),
  }),
  behavior: z.object({
    latency: z.object({
      min: z.number().min(0),
      max: z.number().min(0),
    }).refine(data => data.min <= data.max, {
      message: 'Min latency must be less than or equal to max',
    }).optional(),
    failureRate: z.number().min(0).max(100).optional(),
  }).optional(),
});

// Validate on submit
const handleSave = () => {
  const result = httpEndpointSchema.safeParse(config);
  if (!result.success) {
    setErrors(result.error.flatten().fieldErrors);
    return;
  }
  onSave(result.data);
};
```

### 2.2 MqttEndpointForm Validation

**File:** `ui-builder/frontend/src/components/MqttEndpointForm.tsx`

```typescript
const mqttEndpointSchema = z.object({
  topic: z.string()
    .min(1, 'Topic is required')
    .regex(/^[^#]*#?$/, 'Multi-level wildcard # must be at the end')
    .regex(/^[^+]*(\+[^+]*)*$/, 'Single-level wildcards + must be separated'),
  qos: z.union([z.literal(0), z.literal(1), z.literal(2)]),
  behavior: z.object({
    latency: z.object({
      minMs: z.number().min(0),
      maxMs: z.number().min(0),
    }).refine(data => data.minMs <= data.maxMs, {
      message: 'Min latency must be less than max latency',
    }).optional(),
  }).optional(),
});
```

### 2.3 GraphqlEndpointForm Validation

**File:** `ui-builder/frontend/src/components/GraphqlEndpointForm.tsx`

```typescript
import { parse as parseGraphQL, buildSchema } from 'graphql';

const validateGraphQLSchema = (schema: string): string | null => {
  try {
    buildSchema(schema);
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : 'Invalid GraphQL schema';
  }
};

// Add to form
const [schemaError, setSchemaError] = useState<string | null>(null);

const handleSchemaChange = (value: string) => {
  setConfig({ ...config, schema: value });
  const error = validateGraphQLSchema(value);
  setSchemaError(error);
};
```

### 2.4 SmtpEndpointForm Validation

**File:** `ui-builder/frontend/src/components/SmtpEndpointForm.tsx`

```typescript
const smtpEndpointSchema = z.object({
  port: z.number()
    .min(1).max(65535)
    .refine(port => [25, 465, 587, 2525].includes(port) || true, {
      message: 'Consider using standard SMTP ports: 25, 465, 587, or 2525',
    }),
  connectionTimeout: z.number().min(10000, 'Timeout should be at least 10 seconds'),
  messageHandling: z.object({
    maxMessageSize: z.number()
      .min(1024, 'Min size is 1KB')
      .max(50 * 1024 * 1024, 'Max size is 50MB'),
    maxRecipients: z.number().min(1).max(1000),
  }),
});
```

---

## 3. Accessibility (a11y)

### 3.1 Add ARIA Labels to Form Controls

**File:** `ui-builder/frontend/src/components/HttpEndpointForm.tsx`

```tsx
// Before
<select value={config.method} onChange={handleMethodChange}>

// After
<select
  id="http-method"
  aria-label="HTTP Method"
  value={config.method}
  onChange={handleMethodChange}
>

// Before
<input value={config.path} onChange={handlePathChange} />

// After
<input
  id="endpoint-path"
  aria-label="Endpoint path"
  aria-describedby="path-hint"
  value={config.path}
  onChange={handlePathChange}
/>
<span id="path-hint" className="sr-only">
  Enter the URL path for this endpoint, starting with /
</span>
```

### 3.2 Add Focus Trap to Dialogs

**File:** `ui-builder/frontend/src/pages/Dashboard.tsx`

```tsx
import { FocusTrap } from '@headlessui/react';
// or use @radix-ui/react-focus-trap

// Before
<div className="modal">
  <button onClick={onClose}>✕</button>
  {children}
</div>

// After
<FocusTrap>
  <div
    className="modal"
    role="dialog"
    aria-modal="true"
    aria-labelledby="modal-title"
  >
    <button
      onClick={onClose}
      aria-label="Close dialog"
    >
      <span aria-hidden="true">✕</span>
    </button>
    {children}
  </div>
</FocusTrap>
```

### 3.3 Add Keyboard Navigation

```tsx
// Add keyboard handler to dialogs
const handleKeyDown = (event: React.KeyboardEvent) => {
  if (event.key === 'Escape') {
    onClose();
  }
};

<div onKeyDown={handleKeyDown} tabIndex={-1}>
```

### 3.4 Color Contrast & Non-Color Indicators

**File:** `ui-builder/frontend/src/pages/Dashboard.tsx`

```tsx
// Before - color only
<div className="bg-blue-500">{httpCount}</div>
<div className="bg-green-500">{grpcCount}</div>

// After - color + icon + text label
<div className="bg-blue-500 flex items-center gap-2">
  <GlobeIcon aria-hidden="true" />
  <span>HTTP</span>
  <span className="font-bold">{httpCount}</span>
</div>
```

---

## 4. Error Handling

### 4.1 Add Error Boundaries

**File:** `ui-builder/frontend/src/components/ErrorBoundary.tsx` (exists, extend usage)

```tsx
// Wrap each page in error boundary
<ErrorBoundary fallback={<PageErrorFallback />}>
  <ConfigEditor />
</ErrorBoundary>

// Add specific fallbacks
const PageErrorFallback = () => (
  <div role="alert" className="p-4 bg-red-50 border border-red-200 rounded">
    <h2>Something went wrong</h2>
    <button onClick={() => window.location.reload()}>
      Reload page
    </button>
  </div>
);
```

### 4.2 Improve API Error Messages

**File:** `ui-builder/frontend/src/lib/api.ts`

```typescript
class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public code?: string,
    public details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'ApiError';
  }

  get isNetworkError() {
    return this.status === 0;
  }

  get isValidationError() {
    return this.status === 400 || this.status === 422;
  }

  get userMessage(): string {
    if (this.isNetworkError) {
      return 'Unable to connect to server. Please check your connection.';
    }
    if (this.isValidationError) {
      return this.message;
    }
    return 'An unexpected error occurred. Please try again.';
  }
}
```

### 4.3 Add Specific Error States

**File:** `ui-builder/frontend/src/pages/EndpointBuilder.tsx`

```tsx
const { data, isLoading, error } = useQuery(['endpoint', id], ...);

if (error) {
  return (
    <div role="alert" className="error-state">
      <h2>Failed to load endpoint</h2>
      <p>{error instanceof ApiError ? error.userMessage : 'Unknown error'}</p>
      <button onClick={() => refetch()}>Retry</button>
      <Link to="/dashboard">Back to Dashboard</Link>
    </div>
  );
}
```

---

## 5. Loading States

### 5.1 Add Skeleton Loaders

**File:** `ui-builder/frontend/src/components/Skeleton.tsx`

```tsx
export const FormSkeleton = () => (
  <div className="animate-pulse space-y-4">
    <div className="h-10 bg-gray-200 rounded w-1/3" />
    <div className="h-10 bg-gray-200 rounded w-full" />
    <div className="h-32 bg-gray-200 rounded w-full" />
    <div className="h-10 bg-gray-200 rounded w-1/4" />
  </div>
);

export const DashboardSkeleton = () => (
  <div className="animate-pulse">
    <div className="grid grid-cols-4 gap-4 mb-6">
      {[1, 2, 3, 4].map(i => (
        <div key={i} className="h-24 bg-gray-200 rounded" />
      ))}
    </div>
    <div className="space-y-2">
      {[1, 2, 3, 4, 5].map(i => (
        <div key={i} className="h-16 bg-gray-200 rounded" />
      ))}
    </div>
  </div>
);
```

### 5.2 Use Skeleton in Pages

```tsx
// EndpointBuilder.tsx
if (isLoading) {
  return <FormSkeleton />;
}

// Dashboard.tsx
if (isLoading) {
  return <DashboardSkeleton />;
}
```

---

## 6. Display Bugs

### 6.1 Fix SMTP Display

**File:** `ui-builder/frontend/src/pages/Dashboard.tsx:472`

#### Current (Wrong)
```tsx
<div>From: {endpoint.config.from_pattern}</div>
<div>To: {endpoint.config.to_pattern}</div>
```

#### Fix
```tsx
<div>Port: {endpoint.config.port || 25}</div>
<div>TLS: {endpoint.config.tls ? 'Enabled' : 'Disabled'}</div>
<div>Storage: {endpoint.config.messageHandling?.storageEnabled ? 'Enabled' : 'Disabled'}</div>
```

### 6.2 Add Missing Protocol Icons

**File:** `ui-builder/frontend/src/pages/Dashboard.tsx:182-183`

```tsx
const getProtocolIcon = (protocol: string) => {
  switch (protocol.toLowerCase()) {
    case 'http': return <GlobeIcon />;
    case 'grpc': return <ServerIcon />;
    case 'websocket': return <ArrowsRightLeftIcon />;
    case 'graphql': return <CodeBracketIcon />;
    case 'mqtt': return <SignalIcon />;
    case 'smtp': return <EnvelopeIcon />;
    case 'kafka': return <QueueListIcon />;
    case 'amqp': return <ArrowPathIcon />;
    case 'ftp': return <FolderIcon />;
    case 'tcp': return <CommandLineIcon />;
    default: return <QuestionMarkCircleIcon />;
  }
};
```

---

## 7. Missing Features (Lower Priority)

### 7.1 Endpoint Search/Filter
```tsx
const [search, setSearch] = useState('');
const [protocolFilter, setProtocolFilter] = useState<string | null>(null);

const filteredEndpoints = endpoints.filter(ep => {
  const matchesSearch = ep.name.toLowerCase().includes(search.toLowerCase()) ||
    ep.path?.toLowerCase().includes(search.toLowerCase());
  const matchesProtocol = !protocolFilter || ep.protocol === protocolFilter;
  return matchesSearch && matchesProtocol;
});
```

### 7.2 Endpoint Duplication
```tsx
const handleDuplicate = async (endpoint: Endpoint) => {
  const newEndpoint = {
    ...endpoint,
    id: undefined,
    name: `${endpoint.name} (copy)`,
  };
  await createEndpoint(newEndpoint);
};
```

### 7.3 Bulk Delete
```tsx
const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

const handleBulkDelete = async () => {
  if (!confirm(`Delete ${selectedIds.size} endpoints?`)) return;
  await Promise.all([...selectedIds].map(id => deleteEndpoint(id)));
  setSelectedIds(new Set());
};
```

### 7.4 Add Missing Protocols to Selector

**File:** `ui-builder/frontend/src/components/ProtocolSelector.tsx`

```tsx
const PROTOCOLS = [
  { id: 'http', name: 'HTTP', icon: GlobeIcon },
  { id: 'grpc', name: 'gRPC', icon: ServerIcon },
  { id: 'websocket', name: 'WebSocket', icon: ArrowsRightLeftIcon },
  { id: 'graphql', name: 'GraphQL', icon: CodeBracketIcon },
  { id: 'mqtt', name: 'MQTT', icon: SignalIcon },
  { id: 'smtp', name: 'SMTP', icon: EnvelopeIcon },
  // Add missing protocols
  { id: 'kafka', name: 'Kafka', icon: QueueListIcon },
  { id: 'amqp', name: 'AMQP', icon: ArrowPathIcon },
  { id: 'ftp', name: 'FTP', icon: FolderIcon },
  { id: 'tcp', name: 'TCP', icon: CommandLineIcon },
];
```

---

## 8. Dynamic Version Display

**File:** `ui-builder/frontend/src/components/Layout.tsx:104`

#### Current (Hardcoded)
```tsx
<span>Version 0.1.0</span>
```

#### Fix
```tsx
// vite.config.ts
export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(process.env.npm_package_version),
  },
});

// Layout.tsx
declare const __APP_VERSION__: string;

<span>Version {__APP_VERSION__}</span>
```

---

## Implementation Priority

### Phase 1 (Week 1)
- [ ] Create protocol type definitions
- [ ] Fix form component props
- [ ] Add form validation (HTTP, GraphQL)
- [ ] Fix SMTP display bug

### Phase 2 (Week 2)
- [ ] Add remaining form validations
- [ ] Add ARIA labels to all forms
- [ ] Add focus trap to dialogs
- [ ] Add error boundaries

### Phase 3 (Week 3)
- [ ] Add skeleton loaders
- [ ] Improve error messages
- [ ] Add missing protocol icons
- [ ] Add missing protocols to selector

### Phase 4 (Week 4)
- [ ] Add search/filter
- [ ] Add duplicate feature
- [ ] Add bulk operations
- [ ] Fix version display

---

*Document Version: 1.0*
*Last Updated: 2025-12-27*
