# MockForge UI Builder - Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         User's Browser                           │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              React SPA (Port 5173 - Dev)                  │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────────────┐  │  │
│  │  │  Dashboard  │  │  Endpoint   │  │  Config Editor   │  │  │
│  │  │    Page     │  │   Builder   │  │                  │  │  │
│  │  └─────────────┘  └─────────────┘  └──────────────────┘  │  │
│  │         │                 │                   │            │  │
│  │         └─────────────────┴───────────────────┘            │  │
│  │                           │                                │  │
│  │                  ┌────────▼────────┐                       │  │
│  │                  │   API Client    │                       │  │
│  │                  │  (Axios + RQ)   │                       │  │
│  │                  └────────┬────────┘                       │  │
│  └───────────────────────────┼───────────────────────────────┘  │
└────────────────────────────────┼──────────────────────────────┘
                                 │ HTTP/JSON
                                 │
                ┌────────────────▼────────────────┐
                │   Vite Dev Proxy (Dev Only)     │
                │  /api → localhost:9080          │
                └────────────────┬────────────────┘
                                 │
       ┌─────────────────────────▼──────────────────────────┐
       │          MockForge Admin Server (Port 9080)         │
       │  ┌──────────────────────────────────────────────┐  │
       │  │        UI Builder API (Rust/Axum)            │  │
       │  │                                              │  │
       │  │  GET    /endpoints                          │  │
       │  │  POST   /endpoints                          │  │
       │  │  GET    /endpoints/:id                      │  │
       │  │  PUT    /endpoints/:id                      │  │
       │  │  DELETE /endpoints/:id                      │  │
       │  │  POST   /endpoints/validate                 │  │
       │  │  GET    /config                             │  │
       │  │  PUT    /config                             │  │
       │  │  GET    /config/export                      │  │
       │  │  POST   /config/import                      │  │
       │  │                                              │  │
       │  └────────────────┬─────────────────────────────┘  │
       │                   │                                 │
       │         ┌─────────▼──────────┐                     │
       │         │   UIBuilderState   │                     │
       │         │  - Endpoints store │                     │
       │         │  - Server config   │                     │
       │         └────────────────────┘                     │
       └────────────────────────────────────────────────────┘
```

## Frontend Architecture

```
ui-builder/frontend/src/
│
├── main.tsx                    # Entry point, React Query setup
│
├── App.tsx                     # Root component, routing
│   └── BrowserRouter
│       ├── Layout              # App shell with sidebar
│       │   ├── Dashboard       # Main endpoint list
│       │   ├── EndpointBuilder # Create/edit endpoints
│       │   └── ConfigEditor    # Full config editor
│       └── Toaster             # Toast notifications
│
├── components/
│   ├── Layout.tsx              # Sidebar + main content area
│   ├── ProtocolSelector.tsx    # Protocol chooser grid
│   ├── HttpEndpointForm.tsx    # HTTP-specific form
│   ├── GrpcEndpointForm.tsx    # gRPC-specific form
│   └── WebsocketEndpointForm.tsx # WebSocket-specific form
│
├── pages/
│   ├── Dashboard.tsx           # List view with stats
│   ├── EndpointBuilder.tsx     # Endpoint creation wizard
│   └── ConfigEditor.tsx        # Monaco-based config editor
│
├── lib/
│   ├── api.ts                  # API client + TypeScript types
│   └── utils.ts                # Helper functions (cn, formatters)
│
└── store/
    └── useEndpointStore.ts     # Zustand store for current endpoint
```

## Backend Architecture

```
crates/mockforge-http/src/ui_builder.rs

┌──────────────────────────────────────────────────┐
│              Data Models (Serde)                 │
├──────────────────────────────────────────────────┤
│  EndpointConfig                                  │
│  ├── id: String                                  │
│  ├── protocol: Protocol                          │
│  ├── name: String                                │
│  ├── description: Option<String>                 │
│  ├── enabled: bool                               │
│  └── config: EndpointProtocolConfig              │
│      ├── Http(HttpEndpointConfig)                │
│      ├── Grpc(GrpcEndpointConfig)                │
│      └── Websocket(WebsocketEndpointConfig)      │
├──────────────────────────────────────────────────┤
│  ResponseBody                                    │
│  ├── Static { content: Value }                   │
│  ├── Template { template: String }               │
│  ├── Faker { schema: Value }                     │
│  └── AI { prompt: String }                       │
├──────────────────────────────────────────────────┤
│  EndpointBehavior                                │
│  ├── latency: LatencyConfig                      │
│  ├── failure: FailureConfig                      │
│  └── traffic_shaping: TrafficShapingConfig       │
└──────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────┐
│            API Route Handlers                    │
├──────────────────────────────────────────────────┤
│  list_endpoints()       → JSON<EndpointList>     │
│  get_endpoint(id)       → JSON<Endpoint>         │
│  create_endpoint(data)  → JSON<Endpoint>         │
│  update_endpoint(id, data) → JSON<Endpoint>      │
│  delete_endpoint(id)    → StatusCode             │
│  validate_endpoint(endpoint) → JSON<Validation>  │
│  export_config()        → YAML String            │
│  import_config(config, format) → JSON<Result>    │
│  get_config()           → JSON<ServerConfig>     │
│  update_config(config)  → JSON<ServerConfig>     │
└──────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────┐
│           State Management                       │
├──────────────────────────────────────────────────┤
│  UIBuilderState                                  │
│  ├── endpoints: Arc<RwLock<Vec<EndpointConfig>>> │
│  └── server_config: Arc<RwLock<ServerConfig>>    │
└──────────────────────────────────────────────────┘
```

## Data Flow

### Creating an Endpoint

```
User Action                Frontend                 Backend
    │                         │                        │
    ├──1. Click "New"────────>│                        │
    │                         │                        │
    │                    [Select Protocol]            │
    │                         │                        │
    ├──2. Fill Form──────────>│                        │
    │                         │                        │
    ├──3. Click "Save"───────>│                        │
    │                         │                        │
    │                    [Validate Client-Side]       │
    │                         │                        │
    │                         ├──POST /endpoints──────>│
    │                         │    validate            │
    │                         │                        │
    │                         │<──ValidationResult─────┤
    │                         │                        │
    │                    [Check Errors]               │
    │                         │                        │
    │                         ├──POST /endpoints──────>│
    │                         │                        │
    │                         │                   [Store in State]
    │                         │                        │
    │                         │<──Created Endpoint─────┤
    │                         │                        │
    │<─4. Toast Success───────┤                        │
    │                         │                        │
    │<─5. Redirect to Dashboard┤                       │
```

### Exporting Configuration

```
User Action                Frontend                 Backend
    │                         │                        │
    ├──1. Click "Export"─────>│                        │
    │                         │                        │
    │                         ├──GET /config/export───>│
    │                         │                        │
    │                         │                   [Serialize to YAML]
    │                         │                        │
    │                         │<──YAML String──────────┤
    │                         │                        │
    │                    [Create Blob]                │
    │                    [Download File]              │
    │                         │                        │
    │<─2. File Downloaded─────┤                        │
```

## Technology Choices

### Frontend

| Technology | Purpose | Why? |
|-----------|---------|------|
| React 18 | UI Framework | Industry standard, great ecosystem |
| TypeScript | Type Safety | Catch errors at compile-time |
| Vite | Build Tool | Fastest HMR, modern defaults |
| TailwindCSS | Styling | Rapid development, consistent design |
| React Query | Server State | Caching, optimistic updates |
| Monaco Editor | Code Editor | Powers VS Code, full IntelliSense |
| React Router | Routing | De-facto standard for React SPAs |
| Zustand | Local State | Lightweight, simple API |
| Axios | HTTP Client | Better API than fetch, interceptors |
| Sonner | Notifications | Beautiful toast messages |

### Backend

| Technology | Purpose | Why? |
|-----------|---------|------|
| Rust | Language | Memory safety, performance |
| Axum | Web Framework | Already used in MockForge |
| Serde | Serialization | Industry standard for Rust |
| Tokio | Async Runtime | Best async runtime for Rust |
| UUID | IDs | Unique identifiers |

## Protocol Support Matrix

| Protocol | Status | Endpoint Form | Response Types | Behavior |
|----------|--------|---------------|----------------|----------|
| HTTP | ✅ Complete | ✅ | Static, Template, Faker, AI | ✅ |
| gRPC | ✅ Complete | ✅ | Static, Template | ✅ |
| WebSocket | ✅ Complete | ✅ | Static, Template | ✅ |
| GraphQL | ⏳ Planned | ❌ | - | - |
| MQTT | ⏳ Planned | ❌ | - | - |
| SMTP | ⏳ Planned | ❌ | - | - |
| Kafka | ⏳ Planned | ❌ | - | - |
| AMQP | ⏳ Planned | ❌ | - | - |
| FTP | ⏳ Planned | ❌ | - | - |

## Security Model

```
┌─────────────────────────────────────────────────┐
│              Security Layers                    │
├─────────────────────────────────────────────────┤
│  1. Input Validation                            │
│     - Frontend: TypeScript types + React forms  │
│     - Backend: Serde validation + custom logic  │
├─────────────────────────────────────────────────┤
│  2. Authentication (TODO)                       │
│     - Inherit from admin server auth            │
│     - JWT tokens or session cookies             │
├─────────────────────────────────────────────────┤
│  3. Authorization (TODO)                        │
│     - Role-based access control                 │
│     - Read-only vs. Admin users                 │
├─────────────────────────────────────────────────┤
│  4. XSS Prevention                              │
│     - React automatic escaping                  │
│     - Content Security Policy headers           │
├─────────────────────────────────────────────────┤
│  5. CSRF Protection (TODO)                      │
│     - CSRF tokens for state-changing operations │
├─────────────────────────────────────────────────┤
│  6. Rate Limiting                               │
│     - Leverage existing MockForge rate limits   │
└─────────────────────────────────────────────────┘
```

## Performance Characteristics

| Metric | Value | Notes |
|--------|-------|-------|
| Frontend Bundle Size | ~500KB gzipped | Acceptable for admin UI |
| Initial Load Time | <2s | On fast connection |
| Time to Interactive | <3s | Includes Monaco loading |
| API Response Time | <10ms | CRUD operations |
| Memory Usage (Frontend) | ~50MB | Monaco is heaviest |
| Memory Usage (Backend) | ~10MB | Shared with admin server |
| Concurrent Users | 100+ | Limited by admin server |

## Deployment Strategies

### Development
```
Frontend (Vite Dev Server)  Backend (Cargo Run)
    Port 5173          →      Port 9080
         ↓                         ↓
    Vite Proxy         →    Admin API
```

### Production (Option 1: Separate Deployment)
```
Frontend (Static CDN)   Backend (MockForge Server)
    CDN URL          →      Port 9080
         ↓                         ↓
   Static Files      →    Admin API
```

### Production (Option 2: Embedded)
```
Frontend (Embedded in Binary)  Backend (MockForge Server)
    /ui-builder/*           →      Port 9080
         ↓                              ↓
   Serve from Binary       →    Admin API
```

## Error Handling

```
Frontend Error Flow:
┌─────────────────────────────────┐
│  User Action / API Call         │
└────────────┬────────────────────┘
             │
    ┌────────▼─────────┐
    │  Try/Catch       │
    └────────┬─────────┘
             │
    ┌────────▼──────────────────┐
    │  React Query Error Handler│
    └────────┬──────────────────┘
             │
    ┌────────▼─────────┐
    │  Toast Error Msg │
    └──────────────────┘

Backend Error Flow:
┌─────────────────────────────────┐
│  API Request                    │
└────────────┬────────────────────┘
             │
    ┌────────▼─────────┐
    │  Validation       │
    └────────┬─────────┘
             │
    ┌────────▼──────────┐
    │  Business Logic   │
    └────────┬──────────┘
             │
    ┌────────▼──────────────────┐
    │  Result<T, StatusCode>    │
    └────────┬──────────────────┘
             │
    ┌────────▼─────────┐
    │  JSON Response   │
    └──────────────────┘
```

## Extension Points

### Adding a New Protocol

1. **Backend** (`ui_builder.rs`):
   ```rust
   // 1. Add to Protocol enum
   pub enum Protocol {
       // ...
       Kafka,
   }

   // 2. Create config struct
   pub struct KafkaEndpointConfig {
       pub topic: String,
       pub message: ResponseBody,
   }

   // 3. Add to EndpointProtocolConfig
   pub enum EndpointProtocolConfig {
       // ...
       Kafka(KafkaEndpointConfig),
   }
   ```

2. **Frontend**:
   ```typescript
   // 1. Update Protocol type (lib/api.ts)
   protocol: 'http' | 'grpc' | 'websocket' | 'kafka'

   // 2. Add to ProtocolSelector (components/ProtocolSelector.tsx)
   {
     id: 'kafka',
     name: 'Kafka',
     description: 'Message streaming',
     icon: Database,
     color: 'text-orange-500',
   }

   // 3. Create form component (components/KafkaEndpointForm.tsx)
   export default function KafkaEndpointForm({ config, onChange }) {
     // Form fields for topic, message, etc.
   }

   // 4. Add to EndpointBuilder (pages/EndpointBuilder.tsx)
   {endpoint.protocol === 'kafka' && (
     <KafkaEndpointForm config={config} onChange={...} />
   )}
   ```

### Adding a New Response Type

1. **Backend**:
   ```rust
   pub enum ResponseBody {
       // ...
       Database { query: String },
   }
   ```

2. **Frontend**:
   ```typescript
   // Add button and editor in HttpEndpointForm.tsx
   { id: 'database', label: 'Database', icon: Database }
   ```

## Future Architecture Enhancements

1. **WebSocket Live Updates**
   - Real-time endpoint list updates
   - Collaborative editing
   - Live request monitoring

2. **Plugin System**
   - Custom response generators
   - Custom validators
   - Custom UI components

3. **Microservices Split**
   - Separate UI Builder service
   - gRPC communication with MockForge
   - Independent scaling

4. **GraphQL API**
   - Replace REST with GraphQL
   - Better type safety
   - Efficient queries

---

**Version**: 1.0.0
**Last Updated**: October 2025
