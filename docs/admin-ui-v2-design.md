# MockForge Admin UI v2 Design

## Overview

Admin UI v2 is a modern React-based dashboard using Shadcn UI components that provides enhanced UX for power users. It focuses on eliminating the need for manual file editing by providing comprehensive controls through the web interface.

## Architecture

### Technology Stack
- **Frontend Framework**: React 18 with TypeScript
- **UI Components**: Shadcn UI (built on Radix UI primitives)
- **Styling**: Tailwind CSS
- **State Management**: React Query (TanStack Query) + Zustand
- **Real-time Updates**: WebSocket connection with reconnection logic
- **Authentication**: JWT tokens with refresh mechanism
- **Build Tool**: Vite
- **Testing**: Vitest + React Testing Library

### Component Structure
```
src/
├── components/
│   ├── ui/              # Shadcn UI components
│   ├── layout/          # Layout components (header, sidebar, etc.)
│   ├── dashboard/       # Dashboard-specific components
│   ├── routes/          # Route management components
│   ├── fixtures/        # Fixture management components
│   ├── logs/            # Live logs components
│   └── auth/            # Authentication components
├── hooks/               # Custom React hooks
├── stores/              # Zustand stores
├── services/            # API services and WebSocket
├── utils/               # Utility functions
└── types/               # TypeScript type definitions
```

## Key Features

### 1. Tag/Service-Level Toggles with Per-Route Visibility

#### Service Toggle Component
```typescript
interface ServiceToggle {
  id: string;
  name: string;
  enabled: boolean;
  routes: RouteInfo[];
  tags: string[];
}
```

#### Route Visibility Control
- **Hierarchical View**: Services → Routes → Methods
- **Bulk Operations**: Enable/disable entire services or tag groups
- **Granular Control**: Per-route toggle with visual indicators
- **Search & Filter**: Quick access to specific routes
- **Status Indicators**: Visual cues for enabled/disabled states

#### UI Components
- `ServiceToggleCard`: Service-level control with route count
- `RouteToggleList`: Expandable route list with method badges
- `TagFilter`: Multi-select tag filtering
- `BulkActions`: Batch enable/disable operations

### 2. Fixture Diffing & Rename/Move

#### Fixture Management Features
- **Visual Diff**: Side-by-side comparison of fixture changes
- **Version History**: Track fixture modifications over time
- **Rename/Move**: Drag-and-drop interface for organization
- **Search & Filter**: Full-text search across fixture content
- **Batch Operations**: Select multiple fixtures for actions

#### Diff Component
```typescript
interface FixtureDiff {
  id: string;
  name: string;
  oldContent: string;
  newContent: string;
  changes: DiffChange[];
  timestamp: Date;
}
```

#### UI Components
- `FixtureDiffViewer`: Monaco Editor with diff visualization
- `FixtureTree`: Hierarchical file browser with drag-drop
- `FixtureSearch`: Advanced search with filters
- `VersionHistory`: Timeline view of fixture changes

### 3. Live Logs Panel

#### Real-time Log Streaming
- **WebSocket Connection**: Persistent connection for log streaming
- **Auto-scroll**: Smart scrolling with pause/resume controls
- **Filtering**: Real-time filtering by level, service, method
- **Export**: Download filtered logs as JSON/CSV
- **Performance**: Virtual scrolling for large log volumes

#### Log Entry Structure
```typescript
interface LogEntry {
  id: string;
  timestamp: Date;
  level: 'debug' | 'info' | 'warn' | 'error';
  service: string;
  method: string;
  path: string;
  statusCode?: number;
  responseTime?: number;
  error?: string;
  metadata: Record<string, any>;
}
```

#### UI Components
- `LiveLogPanel`: Main log display with virtual scrolling
- `LogFilters`: Advanced filtering controls
- `LogEntry`: Individual log entry component
- `LogExport`: Export functionality

### 4. Metrics Dashboard

#### Latency Histograms
- **Real-time Updates**: Live latency distribution charts
- **Percentile Views**: P50, P95, P99 response times
- **Service Breakdown**: Per-service latency analysis
- **Time Range Selection**: Configurable time windows
- **Alerting**: Visual indicators for SLA breaches

#### Failure Counters
- **Error Rate Tracking**: Success/failure ratios
- **Status Code Distribution**: HTTP status code breakdown
- **Failure Categorization**: Error type classification
- **Trend Analysis**: Historical failure patterns

#### Metrics Components
```typescript
interface LatencyMetrics {
  service: string;
  route: string;
  p50: number;
  p95: number;
  p99: number;
  histogram: HistogramBucket[];
}

interface FailureMetrics {
  service: string;
  totalRequests: number;
  successCount: number;
  failureCount: number;
  errorRate: number;
  statusCodes: Record<number, number>;
}
```

#### UI Components
- `LatencyHistogram`: Recharts-based histogram visualization
- `FailureCounter`: Error rate displays with trends
- `MetricsDashboard`: Comprehensive metrics overview
- `AlertIndicator`: Visual SLA breach indicators

### 5. Authentication System

#### Token-Based Authentication
- **JWT Tokens**: Secure token-based authentication
- **Refresh Mechanism**: Automatic token renewal
- **Role-Based Access**: Admin/viewer role separation
- **Session Management**: Proper logout and cleanup

#### Authentication Flow
```typescript
interface AuthState {
  user: User | null;
  token: string | null;
  refreshToken: string | null;
  isAuthenticated: boolean;
  role: 'admin' | 'viewer';
}
```

#### UI Components
- `LoginForm`: Authentication form with validation
- `AuthGuard`: Route protection component
- `UserProfile`: User info and logout
- `RoleIndicator`: Visual role display

## WebSocket API Extensions

### New Endpoints for Real-time Features

#### Log Streaming
```
WS /api/v2/logs/stream
- Subscribe to filtered log streams
- Real-time log delivery
- Backpressure handling
```

#### Metrics Updates
```
WS /api/v2/metrics/stream
- Live metrics updates
- Configurable update intervals
- Historical data requests
```

#### Configuration Changes
```
WS /api/v2/config/stream
- Real-time config update notifications
- Change conflict resolution
- Multi-user editing coordination
```

## REST API Extensions

### Service Management
```
GET    /api/v2/services           # List all services with routes
PUT    /api/v2/services/{id}      # Update service configuration
POST   /api/v2/services/bulk      # Bulk service operations
```

### Fixture Management
```
GET    /api/v2/fixtures/{id}/diff   # Get fixture diff
POST   /api/v2/fixtures/move       # Move/rename fixtures
GET    /api/v2/fixtures/search     # Search fixtures
POST   /api/v2/fixtures/batch      # Batch fixture operations
```

### Authentication
```
POST   /api/v2/auth/login         # User authentication
POST   /api/v2/auth/refresh       # Token refresh
POST   /api/v2/auth/logout        # Logout
GET    /api/v2/auth/profile       # User profile
```

### Metrics
```
GET    /api/v2/metrics/latency    # Latency histogram data
GET    /api/v2/metrics/failures   # Failure counter data
GET    /api/v2/metrics/export     # Export metrics data
```

## State Management Strategy

### Zustand Stores
```typescript
// Auth Store
interface AuthStore {
  user: User | null;
  login: (credentials: Credentials) => Promise<void>;
  logout: () => void;
  refreshToken: () => Promise<void>;
}

// UI Store
interface UIStore {
  sidebarOpen: boolean;
  theme: 'light' | 'dark';
  activeTab: string;
  toggleSidebar: () => void;
  setTheme: (theme: 'light' | 'dark') => void;
}

// Services Store
interface ServicesStore {
  services: ServiceInfo[];
  toggles: Record<string, boolean>;
  updateService: (id: string, data: Partial<ServiceInfo>) => void;
  toggleRoute: (serviceId: string, routeId: string) => void;
}
```

### React Query Integration
- **Server State**: Manage server-side data with caching
- **Mutations**: Handle CRUD operations with optimistic updates
- **Background Sync**: Keep data fresh with background refetching
- **Error Handling**: Centralized error management

## Performance Considerations

### Optimization Strategies
- **Virtual Scrolling**: Handle large datasets efficiently
- **Memoization**: Prevent unnecessary re-renders
- **Code Splitting**: Lazy load components and routes
- **Bundle Optimization**: Tree shaking and minification
- **Caching**: Intelligent data caching strategies

### Scalability
- **WebSocket Pooling**: Efficient connection management
- **Data Pagination**: Handle large datasets
- **Memory Management**: Proper cleanup and garbage collection
- **Background Processing**: Non-blocking operations

## Development Workflow

### Project Setup
```bash
# Create React app with Vite
npm create vite@latest mockforge-admin-ui -- --template react-ts
cd mockforge-admin-ui

# Install dependencies
npm install @shadcn/ui @tanstack/react-query zustand
npm install recharts monaco-editor @monaco-editor/react
npm install tailwindcss @tailwindcss/typography
npm install react-router-dom @types/react-router-dom

# Setup Shadcn UI
npx shadcn-ui@latest init
```

### Component Development
1. **Shadcn Component Setup**: Install and configure UI components
2. **Storybook Integration**: Component documentation and testing
3. **Type Safety**: Full TypeScript coverage
4. **Testing**: Unit and integration tests

### Integration Points
- **Backend API**: RESTful API with OpenAPI documentation
- **WebSocket Gateway**: Real-time communication layer
- **Authentication**: JWT token integration
- **Deployment**: Docker containerization

## Migration Strategy

### Phase 1: Foundation (Week 1-2)
- Setup React project with Shadcn UI
- Implement basic layout and navigation
- Setup authentication system
- Create service toggle functionality

### Phase 2: Core Features (Week 3-4)
- Implement fixture management with diffing
- Add live logs panel with WebSocket
- Create basic metrics dashboard
- Setup state management

### Phase 3: Advanced Features (Week 5-6)
- Enhanced metrics with histograms
- Advanced filtering and search
- Performance optimizations
- Comprehensive testing

### Phase 4: Polish & Documentation (Week 7)
- UI/UX refinements
- Documentation updates
- Deployment configuration
- User acceptance testing

## Acceptance Criteria

### Power-User Workflows
1. **Service Management**: Enable/disable services without file editing
2. **Route Configuration**: Manage route visibility through UI controls
3. **Fixture Management**: Create, edit, and organize fixtures visually
4. **Real-time Monitoring**: Monitor service health and performance live
5. **Troubleshooting**: Debug issues through logs and metrics
6. **Configuration**: Modify server settings through the interface

### Success Metrics
- **Task Completion**: 95% of admin tasks completable through UI
- **User Satisfaction**: Positive feedback on UX improvements
- **Performance**: Sub-second response times for all operations
- **Reliability**: 99.9% uptime for admin interface
- **Accessibility**: WCAG 2.1 AA compliance

This design provides a comprehensive foundation for Admin UI v2 that addresses all the requirements while maintaining scalability and user experience quality.
