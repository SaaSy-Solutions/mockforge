# MockForge UI Developer Guide

This guide provides information for developers who want to understand, extend, or contribute to the MockForge Admin UI.

## Architecture Overview

### Backend (Rust)

The backend is built with:
- **Axum**: Web framework for handling HTTP requests
- **Tokio**: Async runtime for concurrent operations
- **Serde**: JSON serialization/deserialization
- **Tower**: Middleware and utilities
- **Tracing**: Logging and observability

#### Key Components

```
mockforge-ui/
├── src/
│   ├── lib.rs              # Main library interface
│   ├── handlers.rs         # Request handlers and business logic
│   ├── routes.rs           # Route definitions and middleware
│   ├── models.rs           # Data models and types
│   └── main.rs             # CLI entry point (if separate)
├── ui/                     # React frontend
├── tests/                  # Integration tests
└── build.rs               # Build script for asset embedding
```

### Frontend (React + TypeScript)

The frontend uses modern React patterns:
- **React 19**: Latest React with concurrent features
- **TypeScript**: Type safety and better DX
- **Vite**: Fast build tool and dev server
- **Tailwind CSS**: Utility-first CSS framework
- **React Query**: Data fetching and caching
- **Radix UI**: Accessible component primitives

#### Static Asset Serving

The React frontend is built and embedded into the Rust binary at compile time:

1. **Build Process**: Vite builds the React app into static files (`index.html`, `assets/index.js`, `assets/index.css`)
2. **Asset Embedding**: Build script (`build.rs`) embeds these files using `include_str!()` macros
3. **Runtime Serving**: Assets are served directly from memory without filesystem access
4. **SPA Fallback**: Catch-all route (`/{*path}`) serves `index.html` for client-side routing

**Advantages:**
- No filesystem dependencies in production
- Faster startup (assets loaded with binary)
- Simplified deployment (single binary)
- Version consistency (assets match binary version)

**Route Configuration:**
```rust
let mut router = Router::new()
    .route("/", get(serve_admin_html))
    .route("/assets/index.css", get(serve_admin_css))
    .route("/assets/index.js", get(serve_admin_js))
    // SPA fallback for client-side routing
    .route("/{*path}", get(serve_admin_html));
```

This enables proper React Router functionality where routes like `/dashboard` or `/config` serve the `index.html` file, allowing the React app to handle routing client-side.

#### Key Components

```
ui/src/
├── components/             # Reusable UI components
│   ├── ui/                # Basic UI primitives
│   ├── dashboard/         # Dashboard-specific components
│   ├── error/             # Error handling components
│   └── layout/            # Layout components
├── pages/                 # Page components
├── hooks/                 # Custom React hooks
├── services/              # API service layer
├── stores/                # State management (Zustand)
├── types/                 # TypeScript type definitions
└── utils/                 # Utility functions
```

## Development Setup

### Prerequisites

- **Rust**: 1.70+ with wasm target
- **Node.js**: 18+ (preferably 20+)
- **pnpm**: Package manager
- **Docker**: For containerized development

### Backend Development

1. **Clone and setup**:
   ```bash
   git clone <repository>
   cd mockforge
   cargo build
   ```

2. **Run tests**:
   ```bash
   cargo test --package mockforge-ui
   ```

3. **Run with feature flags**:
   ```bash
   cargo run --features ui-dev
   ```

### Frontend Development

1. **Install dependencies**:
   ```bash
   cd crates/mockforge-ui/ui
   pnpm install
   ```

2. **Start dev server**:
   ```bash
   pnpm dev
   ```

3. **Build for production**:
   ```bash
   pnpm build
   ```

4. **Run tests**:
   ```bash
   pnpm test
   ```

## Adding New Features

### Backend API Endpoints

1. **Add route handler** in `handlers.rs`:
   ```rust
   pub async fn my_new_endpoint(
       State(state): State<AdminState>
   ) -> Json<ApiResponse<String>> {
       // Implementation
       Json(ApiResponse::success("Hello World".to_string()))
   }
   ```

2. **Add route** in `routes.rs`:
   ```rust
   .route("/__mockforge/my-endpoint", get(my_new_endpoint))
   ```

3. **Add tests** in `tests/integration.rs`:
   ```rust
   #[tokio::test]
   async fn test_my_new_endpoint() {
       let app = create_admin_router(None, None, None, true);
       // Test implementation
   }
   ```

### Frontend Components

1. **Create component**:
   ```tsx
   import React from 'react';
   import { useMyApi } from '../hooks/useApi';

   export function MyComponent() {
     const { data, isLoading, error } = useMyApi();

     if (isLoading) return <div>Loading...</div>;
     if (error) return <div>Error: {error.message}</div>;

     return <div>{data}</div>;
   }
   ```

2. **Add to routing** in `App.tsx`:
   ```tsx
   switch (activeTab) {
     case 'my-feature':
       return <MyComponent />;
   }
   ```

3. **Add API hook** in `hooks/useApi.ts`:
   ```ts
   export function useMyApi() {
     return useQuery({
       queryKey: ['my-api'],
       queryFn: () => api.myEndpoint(),
     });
   }
   ```

## Performance Optimization

### Backend

1. **Async/Await Best Practices**:
   - Use `tokio::spawn` for CPU-intensive tasks
   - Avoid blocking operations in async functions
   - Use `RwLock` for concurrent read access

2. **Memory Management**:
   - Limit log history to prevent memory leaks
   - Use streaming for large responses
   - Implement proper cleanup in drop handlers

3. **Database Optimization**:
   - Use connection pooling
   - Implement proper indexing
   - Batch operations when possible

### Frontend

1. **React Query Optimization**:
   ```ts
   // Optimized query configuration
   const queryClient = new QueryClient({
     defaultOptions: {
       queries: {
         staleTime: 5 * 60 * 1000, // 5 minutes
         gcTime: 10 * 60 * 1000,   // 10 minutes
         retry: (count, error) => count < 3 && !isClientError(error),
       },
     },
   });
   ```

2. **Component Optimization**:
   ```tsx
   // Use React.memo for expensive components
   const MyComponent = React.memo(({ data }) => {
     return <div>{data.value}</div>;
   });

   // Use useMemo for expensive calculations
   const processedData = useMemo(() => {
     return expensiveCalculation(data);
   }, [data]);
   ```

3. **Lazy Loading**:
   ```tsx
   const MyHeavyComponent = lazy(() => import('./MyHeavyComponent'));

   function App() {
     return (
       <Suspense fallback={<div>Loading...</div>}>
         <MyHeavyComponent />
       </Suspense>
     );
   }
   ```

## Testing Strategy

### Backend Tests

1. **Unit Tests**:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_my_function() {
           assert_eq!(my_function(2), 4);
       }
   }
   ```

2. **Integration Tests**:
   ```rust
   #[tokio::test]
   async fn test_api_endpoint() {
       let app = create_admin_router(None, None, None, true);
       // Test HTTP interactions
   }
   ```

3. **Smoke Tests**:
   - Test basic functionality
   - Verify static assets are served
   - Check health endpoints

### Frontend Tests

1. **Component Tests**:
   ```tsx
   import { render, screen } from '@testing-library/react';

   test('renders component', () => {
     render(<MyComponent />);
     expect(screen.getByText('Hello')).toBeInTheDocument();
   });
   ```

2. **API Integration Tests**:
   ```tsx
   // Mock fetch for API testing
   global.fetch = jest.fn();

   test('fetches data', async () => {
     mockFetch.mockResolvedValue({
       ok: true,
       json: () => Promise.resolve({ success: true, data: 'test' }),
     });

     render(<TestComponent />);
     await waitFor(() => {
       expect(screen.getByText('test')).toBeInTheDocument();
     });
   });
   ```

## Error Handling

### Backend Error Handling

1. **Custom Error Types**:
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum Error {
       #[error("IO error: {0}")]
       Io(#[from] std::io::Error),

       #[error("Parse error: {0}")]
       Parse(#[from] serde_json::Error),
   }
   ```

2. **Graceful Degradation**:
   ```rust
   pub async fn robust_handler() -> Result<Json<Value>, StatusCode> {
       match fallible_operation().await {
           Ok(data) => Ok(Json(data)),
           Err(_) => Ok(Json(json!({"fallback": "data"}))),
       }
   }
   ```

### Frontend Error Handling

1. **Error Boundaries**:
   ```tsx
   class ErrorBoundary extends Component {
     componentDidCatch(error: Error, info: React.ErrorInfo) {
       logError(error, info);
     }

     render() {
       if (this.state.hasError) {
         return <ErrorFallback />;
       }
       return this.props.children;
     }
   }
   ```

2. **API Error Handling**:
   ```tsx
   try {
     const result = await apiCall();
     return result;
   } catch (error) {
     if (error.status === 401) {
       // Handle auth error
     } else if (error.status >= 500) {
       // Handle server error
     } else {
       // Handle client error
     }
   }
   ```

## Security Considerations

### Backend Security

1. **Input Validation**:
   ```rust
   use validator::Validate;

   #[derive(Validate)]
   struct UserInput {
       #[validate(length(min = 1, max = 100))]
       name: String,
   }
   ```

2. **Path Traversal Protection**:
   ```rust
   fn validate_path(path: &str) -> Result<(), Error> {
       if path.contains("..") {
           return Err(Error::InvalidPath);
       }
       Ok(())
   }
   ```

3. **Rate Limiting**:
   ```rust
   use tower::limit::RateLimitLayer;

   let app = Router::new()
       .layer(RateLimitLayer::new(100, std::time::Duration::from_secs(60)));
   ```

### Frontend Security

1. **XSS Protection**:
   - Use React's built-in XSS protection
   - Sanitize user input with DOMPurify
   - Use Content Security Policy headers

2. **CSRF Protection**:
   - Use SameSite cookies
   - Implement CSRF tokens for state-changing operations

## Deployment

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --package mockforge-ui

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/mockforge-ui /usr/local/bin/
EXPOSE 9080
CMD ["mockforge-ui"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mockforge-ui
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mockforge-ui
  template:
    metadata:
      labels:
        app: mockforge-ui
    spec:
      containers:
      - name: mockforge-ui
        image: mockforge-ui:latest
        ports:
        - containerPort: 9080
        env:
        - name: RUST_LOG
          value: info
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

## Monitoring and Observability

### Backend Monitoring

1. **Metrics Collection**:
   ```rust
   use metrics::{counter, histogram};

   counter!("requests_total", 1);
   histogram!("request_duration", duration.as_secs_f64());
   ```

2. **Health Checks**:
   ```rust
   async fn health_check() -> impl IntoResponse {
       Json(HealthCheck {
           status: "healthy".to_string(),
           services: HashMap::new(),
           last_check: Utc::now(),
           issues: vec![],
       })
   }
   ```

3. **Distributed Tracing**:
   ```rust
   use tracing::{info, error, instrument};

   #[instrument]
   async fn my_handler() {
       info!("Processing request");
   }
   ```

### Frontend Monitoring

1. **Error Tracking**:
   ```ts
   import * as Sentry from '@sentry/react';

   Sentry.init({
     dsn: 'your-dsn',
     integrations: [new Sentry.BrowserTracing()],
   });
   ```

2. **Performance Monitoring**:
   ```ts
   // Use Web Vitals
   import { getCLS, getFID, getFCP, getLCP, getTTFB } from 'web-vitals';

   getLCP(console.log);
   ```

## Contributing Guidelines

### Code Style

1. **Rust**:
   - Use `rustfmt` for formatting
   - Follow `clippy` linting rules
   - Write comprehensive documentation

2. **TypeScript/React**:
   - Use ESLint and Prettier
   - Follow React best practices
   - Write TypeScript interfaces for all data structures

### Git Workflow

1. Create feature branch from `main`
2. Make small, focused commits
3. Write tests for new functionality
4. Update documentation
5. Create pull request with description

### Commit Messages

```
feat: add user authentication
fix: resolve memory leak in handler
docs: update API documentation
test: add integration tests for config endpoints
refactor: simplify error handling logic
```

## Troubleshooting

### Common Issues

1. **Build Failures**:
   - Clear `target/` and `node_modules/`
   - Check Rust and Node.js versions
   - Verify all dependencies are installed

2. **Runtime Errors**:
   - Check environment variables
   - Verify configuration files
   - Review logs for error details

3. **Performance Issues**:
   - Use profiling tools (`cargo flamegraph`, React DevTools)
   - Check for memory leaks
   - Optimize database queries

### Debug Mode

Enable debug logging:
```bash
RUST_LOG=debug cargo run
```

Enable React development mode:
```bash
NODE_ENV=development npm run dev
```

## Future Enhancements

### Planned Features

1. **Real-time WebSocket Updates**
2. **Advanced Analytics Dashboard**
3. **Plugin System**
4. **Multi-tenant Support**
5. **API Versioning**
6. **GraphQL Support**

### Architecture Improvements

1. **Microservices Architecture**
2. **Event-driven Communication**
3. **Service Mesh Integration**
4. **Advanced Caching Layer**
5. **Machine Learning Integration**

This guide should provide a solid foundation for understanding and contributing to the MockForge UI. Remember to always write tests, follow security best practices, and keep performance in mind when making changes.
