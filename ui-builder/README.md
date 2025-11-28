# MockForge UI Builder

A low-code, drag-and-drop web interface for creating and managing mock endpoints without writing code.

## Features

- **Visual Endpoint Builder**: Create mock endpoints for HTTP, gRPC, and WebSocket protocols
- **Protocol Support**:
  - HTTP/REST with full request/response configuration
  - gRPC with proto file integration
  - WebSocket with event handlers
- **Response Types**:
  - Static JSON responses
  - Template-based responses with variable substitution
  - Faker-powered data generation
  - AI-generated responses
- **Chaos Engineering**: Built-in latency, failure injection, and traffic shaping
- **Config Management**: Import/export YAML/JSON configurations
- **Live Preview**: Test endpoints in real-time
- **Monaco Editor**: Full-featured code editor for JSON/YAML editing

## Architecture

### Backend (Rust)

Located in `/crates/mockforge-http/src/ui_builder.rs`

- RESTful API built with Axum
- Endpoints for CRUD operations on mock endpoints
- Configuration validation
- Import/export functionality

**API Endpoints**:
- `GET /endpoints` - List all endpoints
- `POST /endpoints` - Create endpoint
- `GET /endpoints/:id` - Get endpoint
- `PUT /endpoints/:id` - Update endpoint
- `DELETE /endpoints/:id` - Delete endpoint
- `POST /endpoints/validate` - Validate endpoint
- `GET /config` - Get server config
- `PUT /config` - Update server config
- `GET /config/export` - Export as YAML
- `POST /config/import` - Import YAML/JSON

### Frontend (React + TypeScript)

Located in `/ui-builder/frontend/`

**Tech Stack**:
- React 18
- TypeScript
- Vite (build tool)
- TailwindCSS (styling)
- React Query (data fetching)
- Monaco Editor (code editing)
- Zustand (state management)
- React Router (routing)

**Pages**:
- **Dashboard**: List and manage all endpoints
- **Endpoint Builder**: Create/edit endpoints
- **Config Editor**: Edit full server configuration

**Components**:
- `ProtocolSelector`: Choose protocol type
- `HttpEndpointForm`: Configure HTTP endpoints
- `GrpcEndpointForm`: Configure gRPC endpoints
- `WebsocketEndpointForm`: Configure WebSocket endpoints

## Getting Started

### Prerequisites

- Node.js 18+ (for frontend)
- Rust 1.70+ (for backend)
- MockForge server running

### Installation

1. **Install frontend dependencies**:
```bash
cd ui-builder/frontend
npm install
```

2. **Start the development server**:
```bash
npm run dev
```

The UI will be available at `http://localhost:5173`

3. **Build for production**:
```bash
npm run build
```

The compiled files will be in `ui-builder/frontend/dist/`

### Integration with MockForge

The UI Builder backend is integrated into the MockForge admin API. When you run MockForge with the admin server enabled:

```bash
mockforge serve --admin-enabled --admin-port 9080
```

The UI Builder API will be available at:
```
http://localhost:9080/__mockforge/ui-builder/
```

## Usage

### Creating an HTTP Endpoint

1. Click **"New Endpoint"** on the dashboard
2. Select **HTTP/REST** protocol
3. Configure:
   - **Name**: "Get Users"
   - **Method**: GET
   - **Path**: `/api/users`
   - **Response Status**: 200
   - **Body Type**: Choose from:
     - **Static**: Fixed JSON response
     - **Template**: Use variables like `{{uuid}}`, `{{faker.name}}`
     - **Faker**: Schema-based fake data
     - **AI**: AI-generated response
4. (Optional) Add behavior:
   - **Latency**: Add delay with jitter
   - **Failures**: Inject random errors
5. Click **Save**

### Creating a gRPC Endpoint

1. Click **"New Endpoint"**
2. Select **gRPC** protocol
3. Configure:
   - **Service Name**: `UserService`
   - **Method Name**: `GetUser`
   - **Proto File**: `user.proto`
   - **Request Type**: `GetUserRequest`
   - **Response Type**: `GetUserResponse`
   - **Response Body**: Define as JSON
4. Click **Save**

### Creating a WebSocket Endpoint

1. Click **"New Endpoint"**
2. Select **WebSocket** protocol
3. Configure:
   - **Path**: `/ws`
   - **On Connect**: Send welcome message
   - **On Message**: Choose action (Echo, Send, Broadcast)
   - **On Disconnect**: (auto-handled)
4. Click **Save**

### Importing/Exporting Configuration

**Export**:
1. Go to **Config** page
2. Click **Export**
3. Downloads `mockforge-config.yaml`

**Import**:
1. Go to **Config** page
2. Click **Import**
3. Select YAML or JSON file
4. Click **Save**

## Development

### Project Structure

```
ui-builder/
├── backend/              # (Reference to Rust code in crates/)
├── frontend/
│   ├── src/
│   │   ├── components/   # React components
│   │   ├── pages/        # Page components
│   │   ├── lib/          # Utilities and API client
│   │   ├── store/        # State management
│   │   ├── App.tsx       # Main app component
│   │   └── main.tsx      # Entry point
│   ├── public/
│   ├── index.html
│   ├── package.json
│   └── vite.config.ts
└── README.md
```

### API Client

The API client (`src/lib/api.ts`) provides typed methods for all backend operations:

```typescript
import { endpointsApi } from '@/lib/api'

// List endpoints
const { data } = await endpointsApi.list()

// Create endpoint
await endpointsApi.create({
  protocol: 'http',
  name: 'My Endpoint',
  enabled: true,
  config: { /* ... */ }
})

// Validate endpoint
const result = await endpointsApi.validate(endpoint)
if (!result.data.valid) {
  console.error(result.data.errors)
}
```

### Adding a New Protocol

1. **Update backend** (`ui_builder.rs`):
   - Add to `Protocol` enum
   - Add config struct (e.g., `KafkaEndpointConfig`)
   - Add to `EndpointProtocolConfig` enum

2. **Update frontend**:
   - Add protocol to `ProtocolSelector.tsx`
   - Create form component (e.g., `KafkaEndpointForm.tsx`)
   - Add to `EndpointBuilder.tsx` routing

3. **Update types** (`src/lib/api.ts`):
   - Add protocol to type unions
   - Add config interface

## Testing

### Backend Tests

```bash
cd crates/mockforge-http
cargo test ui_builder
```

### Frontend Tests

```bash
cd ui-builder/frontend
npm test
```

## Deployment

### Production Build

1. Build the frontend:
```bash
cd ui-builder/frontend
npm run build
```

2. Serve the static files from MockForge admin server (future enhancement)

### Docker

A Dockerfile will be provided to bundle the UI Builder with MockForge:

```dockerfile
# Coming soon
FROM rust:1.70 AS builder
# Build MockForge with UI Builder included
```

## Roadmap

- [x] HTTP endpoint builder
- [x] gRPC endpoint builder
- [x] WebSocket endpoint builder
- [x] Config import/export
- [x] Response template editor
- [ ] Live endpoint testing
- [ ] Visual flow designer (React Flow integration)
- [ ] GraphQL endpoint builder
- [ ] MQTT endpoint builder
- [ ] Request history viewer
- [ ] Analytics dashboard
- [ ] Collaborative editing
- [ ] OpenAPI import wizard
- [ ] Proto file editor

## Contributing

See the main [MockForge CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## License

Same as MockForge - see [LICENSE](../LICENSE)
