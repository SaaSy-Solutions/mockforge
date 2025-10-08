# MockForge Admin UI v2

![MockForge Logo](../assets/mockforge-logo.png)

A modern React-based administrative interface for MockForge that provides comprehensive service management capabilities without requiring manual file editing.

## Features

### 🔐 Authentication & Authorization _(Planned for v1.1)_
> **Status**: The frontend UI components for authentication are implemented, but backend JWT authentication and role-based access control are planned for v1.1. The Admin UI is currently accessible without authentication in v1.0.

- JWT-based authentication with role-based access control _(Backend pending)_
- Admin and Viewer roles with appropriate permissions _(Backend pending)_
- Session persistence and automatic token refresh _(Frontend ready)_
- Demo accounts for quick testing _(Frontend ready)_

### 🎛️ Service Management
- Visual enable/disable toggles for services and individual routes
- Tag-based filtering and bulk operations
- Real-time service status indicators
- Request counts, latency metrics, and error tracking per route

### 📁 Fixture Management
- Rich text editor with syntax highlighting
- Visual diff viewer for tracking changes
- Drag-and-drop file organization
- Inline rename and folder management
- Version history and rollback capabilities

### 📊 Live Monitoring
- Real-time log streaming with advanced filtering
- Interactive latency histograms and performance metrics
- Success/failure analysis with status code breakdown
- SLA monitoring with visual compliance indicators

### 🔍 Advanced Search
- Full-text search across services, fixtures, and logs
- Multi-criteria filtering with persistent state
- Tag-based service organization
- Export capabilities for data analysis

## Quick Start

> **Note**: Authentication is not required in v1.0. The Admin UI is accessible directly without login.

### Service Management
1. Navigate to **Services** tab
2. Use toggle switches to enable/disable services
3. Expand service cards to manage individual routes
4. Use tag filters for bulk operations

### Fixture Editing
1. Go to **Fixtures** tab
2. Select a fixture from the tree view
3. Edit content in the rich editor
4. Save with Ctrl+S or the Save button
5. View changes in the diff viewer

### Live Monitoring
1. Access **Live Logs** for real-time request monitoring
2. Use filters to focus on specific endpoints or errors
3. Click log entries for detailed inspection
4. Check **Metrics** for performance analysis

## Technical Architecture

### Frontend Stack
- **React 18** with TypeScript
- **Shadcn UI** components
- **Tailwind CSS** styling
- **Zustand** state management
- **Recharts** data visualization
- **Server-Sent Events (SSE)** for real-time log streaming

### Backend Integration
- RESTful API endpoints for all operations
- Server-Sent Events (SSE) for real-time log streaming
- JWT authentication middleware _(Planned for v1.1)_
- File system operations through API abstraction

### Key Components

#### Authentication System _(Frontend UI only in v1.0)_
```typescript
// Frontend auth components (backend integration pending)
const { login, user, isAuthenticated } = useAuthStore();
// Note: Currently bypasses authentication in v1.0

// Role-based component access (UI ready, backend pending)
<RoleGuard allowedRoles={['admin']}>
  <ServiceManagement />
</RoleGuard>
```

#### Service Management
```typescript
// Toggle service state
const { updateService, toggleRoute } = useServiceStore();
updateService('user-service', { enabled: false });
toggleRoute('user-service', 'GET-/api/users', true);
```

#### Fixture Operations
```typescript
// Edit and track changes
const { updateFixture, generateDiff } = useFixtureStore();
const diff = generateDiff(fixtureId, newContent);
updateFixture(fixtureId, newContent);
```

#### Live Data Streaming
```typescript
// Real-time log monitoring
const { logs, setFilter, isPaused } = useLogStore();
setFilter({ method: 'GET', status_code: 404 });
```

## API Endpoints

### Authentication _(Planned for v1.1)_
```http
POST /api/v2/auth/login      # User authentication (not yet implemented)
POST /api/v2/auth/refresh    # Token refresh (not yet implemented)
POST /api/v2/auth/logout     # Session termination (not yet implemented)
```

### Service Management
```http
GET    /api/v2/services           # List services
PUT    /api/v2/services/{id}      # Update service
POST   /api/v2/services/bulk      # Bulk operations
```

### Fixture Management
```http
GET    /api/v2/fixtures           # List fixtures
PUT    /api/v2/fixtures/{id}      # Update fixture
POST   /api/v2/fixtures/move      # Move/rename
GET    /api/v2/fixtures/{id}/diff # Get diff
```

### Monitoring
```http
GET /__mockforge/logs                   # Get logs
GET /__mockforge/logs/sse              # Live log stream (SSE)
GET /__mockforge/metrics               # Metrics data
```

> **Note**: The actual endpoints use `__mockforge` namespace. API v2 endpoints are planned for future versions.

## Development

### Project Structure
```
ui-v2/
├── src/
│   ├── components/           # React components
│   │   ├── ui/              # Shadcn UI primitives
│   │   ├── auth/            # Authentication components
│   │   ├── services/        # Service management
│   │   ├── fixtures/        # Fixture operations
│   │   ├── logs/            # Log monitoring
│   │   └── metrics/         # Performance metrics
│   ├── stores/              # Zustand state stores
│   ├── types/               # TypeScript definitions
│   └── utils/               # Utility functions
├── public/                  # Static assets
└── dist/                    # Build output
```

### Building
```bash
cd ui-v2
npm install
npm run build
```

### Development Server
```bash
npm run dev  # Start Vite dev server
```

### Testing
```bash
npm run test  # Run test suite
```

## Configuration

### Environment Variables
```bash
MOCKFORGE_ADMIN_UI_V2_ENABLED=true    # Enable v2 interface
MOCKFORGE_ADMIN_PORT=9080             # Admin port
MOCKFORGE_ADMIN_AUTH_ENABLED=true     # Enable authentication
MOCKFORGE_ADMIN_JWT_SECRET=secret     # JWT signing key
```

### Customization
- Replace authentication provider for custom user systems
- Modify theme variables for brand customization
- Extend API endpoints for additional functionality
- Add custom components for domain-specific features

## Migration from Admin UI v1

### Automatic Replacement
Admin UI v2 automatically replaces the static HTML interface when enabled. No manual migration required.

### Feature Comparison
| Feature | v1 | v2 |
|---------|----|----|
| Authentication | ❌ | ✅ Role-based |
| Service Toggle | ❌ | ✅ Visual controls |
| Fixture Editing | ❌ | ✅ Rich editor + diff |
| Live Logs | ⚠️ Basic | ✅ Real-time + filters |
| Metrics | ⚠️ Simple | ✅ Interactive charts |
| Search | ❌ | ✅ Full-text across all data |
| Mobile Support | ❌ | ✅ Responsive design |

### Rollback
```bash
MOCKFORGE_ADMIN_UI_V2_ENABLED=false mockforge serve --admin-ui
```

## Performance

### Optimizations
- Virtual scrolling for large datasets
- Optimistic UI updates
- Background data synchronization
- Efficient state management with Zustand
- Lazy loading of components

### Scalability
- Handles thousands of log entries efficiently
- Supports hundreds of services and routes
- Real-time updates without performance degradation
- Memory-efficient fixture management

## Security

### Authentication
- JWT tokens with configurable expiration
- Automatic token refresh
- Secure session management
- CSRF protection

### Authorization
- Role-based access control
- Component-level permission guards
- API endpoint protection
- Audit logging for admin actions

### Production Considerations
- Change default admin credentials
- Use HTTPS for admin interface
- Configure appropriate session timeouts
- Regularly rotate JWT secrets
- Monitor admin access logs

## Troubleshooting

### Common Issues

#### Authentication Failed
- Verify credentials match configured users
- Check JWT secret configuration
- Ensure admin authentication is enabled

#### WebSocket Connection Failed
- Verify WebSocket endpoint accessibility
- Check proxy configuration for WebSocket support
- Ensure firewall allows WebSocket connections

#### Performance Issues
- Enable browser dev tools to identify bottlenecks
- Check network tab for slow API calls
- Monitor memory usage for large datasets
- Verify WebSocket connection stability

### Debug Mode
```bash
MOCKFORGE_LOG_LEVEL=debug mockforge serve --admin-ui
```

### Browser Requirements
- Modern browser with ES2020 support
- WebSocket API support
- Local Storage enabled
- JavaScript enabled

## Contributing

### Development Setup
1. Clone repository
2. Install dependencies: `npm install`
3. Start dev server: `npm run dev`
4. Run tests: `npm run test`

### Code Standards
- TypeScript for type safety
- ESLint + Prettier for code formatting
- Component testing with React Testing Library
- E2E testing with Playwright

### Submitting Changes
1. Fork repository
2. Create feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit pull request

## License

Admin UI v2 is part of MockForge and follows the same licensing terms.
