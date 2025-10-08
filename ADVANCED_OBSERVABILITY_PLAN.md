# Advanced Observability and Control - Implementation Plan

**Goal:** Transform MockForge into the most insightful mock platform with comprehensive observability and live control capabilities.

**Status:** Planning Phase
**Created:** 2025-10-07
**Estimated Timeline:** 40-60 hours

---

## Current State Analysis

### ✅ What's Already Built

1. **Basic Metrics Infrastructure**
   - gRPC reflection metrics (crates/mockforge-grpc/src/reflection/metrics.rs)
   - Success/error counting, duration tracking, in-flight requests
   - Metrics registry with snapshots

2. **UI Metrics Dashboard**
   - MetricsDashboard component with latency histograms
   - Failure analysis and SLA monitoring
   - Real-time refresh capabilities

3. **Request Logging**
   - Centralized logger with recent logs API
   - HTTP request/response logging
   - Basic filtering by method, path, status

4. **Admin UI v2**
   - React-based interface with authentication
   - Service management and fixture editing
   - Live log streaming
   - WebSocket support for real-time updates

5. **Management API**
   - REST endpoints for mock CRUD operations
   - Server stats and health endpoints
   - Export/import functionality

### ❌ What's Missing for Advanced Observability

1. **Prometheus/OpenTelemetry Integration**
   - No metrics exporter for Prometheus
   - No OpenTelemetry spans/traces
   - No distributed tracing support
   - No plugin execution metrics

2. **Replay Recording & Analysis**
   - No comprehensive request/response storage
   - No replay capabilities beyond basic logging
   - No behavior pattern analysis
   - No visualization of consumer interactions

3. **Scenario Control Center**
   - No live mode switching (Healthy/Degraded/Error)
   - No real-time latency adjustments
   - No Chaos Mode implementation
   - No centralized control interface

---

## Implementation Plan

## Phase 1: Prometheus Integration (8-10 hours)

### 1.1 Create Observability Crate
**New file:** `crates/mockforge-observability/`

```toml
# Cargo.toml
[dependencies]
prometheus = "0.13"
once_cell = "1.19"
tokio = { version = "1.35", features = ["full"] }
axum = "0.7"
```

**Structure:**
```
crates/mockforge-observability/
├── src/
│   ├── lib.rs                    # Main exports
│   ├── prometheus/
│   │   ├── mod.rs               # Prometheus integration
│   │   ├── metrics.rs           # Metric definitions
│   │   ├── exporter.rs          # /metrics endpoint
│   │   └── collectors.rs        # Custom collectors
│   ├── opentelemetry/           # Phase 2
│   ├── recorder/                # Phase 3
│   └── scenarios/               # Phase 4
└── Cargo.toml
```

### 1.2 Define Core Metrics

```rust
// src/prometheus/metrics.rs
use prometheus::{Counter, Histogram, Gauge, IntGauge};

pub struct MockForgeMetrics {
    // Request metrics
    pub http_requests_total: Counter,
    pub grpc_requests_total: Counter,
    pub ws_connections_total: Counter,
    pub graphql_requests_total: Counter,

    // Latency metrics (in seconds, Prometheus standard)
    pub request_duration_seconds: Histogram,

    // Error metrics
    pub errors_total: Counter,

    // Plugin metrics
    pub plugin_executions_total: Counter,
    pub plugin_execution_duration_seconds: Histogram,
    pub plugin_errors_total: Counter,

    // System metrics
    pub active_connections: IntGauge,
    pub memory_usage_bytes: Gauge,

    // Scenario metrics (Phase 4)
    pub active_scenario: IntGauge,
    pub chaos_mode_triggers: Counter,
}
```

### 1.3 Implement Metrics Collector

**Integration points:**
- `crates/mockforge-http/src/lib.rs` - HTTP request tracking
- `crates/mockforge-grpc/src/reflection/` - gRPC request tracking
- `crates/mockforge-ws/src/lib.rs` - WebSocket connection tracking
- `crates/mockforge-plugin-loader/src/lib.rs` - Plugin execution tracking

### 1.4 Add Prometheus Exporter Endpoint

```rust
// src/prometheus/exporter.rs
use axum::{routing::get, Router};
use prometheus::{Encoder, TextEncoder};

pub fn prometheus_router() -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
}

async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder.encode_to_string(&metric_families).unwrap()
}
```

### 1.5 Configuration

```yaml
# New config section in mockforge config
observability:
  prometheus:
    enabled: true
    port: 9090
    path: /metrics
  metrics:
    collect_plugin_stats: true
    histogram_buckets: [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
```

---

## Phase 2: OpenTelemetry Integration (10-12 hours)

### 2.1 Add OpenTelemetry Dependencies

```toml
opentelemetry = "0.21"
opentelemetry-otlp = "0.14"
opentelemetry-semantic-conventions = "0.13"
tracing-opentelemetry = "0.22"
```

### 2.2 Implement Tracer Provider

```rust
// src/opentelemetry/tracer.rs
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, trace as sdktrace};

pub fn init_tracer(endpoint: &str) -> Result<sdktrace::Tracer> {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint)
        )
        .with_trace_config(
            sdktrace::config()
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "mockforge"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
}
```

### 2.3 Add Tracing Middleware

**HTTP Middleware:**
```rust
// Integration with mockforge-http
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub async fn tracing_middleware(
    req: Request<Body>,
    next: Next<Body>,
) -> Response {
    let span = tracing::info_span!(
        "http_request",
        method = %req.method(),
        path = %req.uri().path(),
        status = tracing::field::Empty,
    );

    let _guard = span.enter();
    let response = next.run(req).await;
    span.record("status", response.status().as_u16());
    response
}
```

### 2.4 Distributed Tracing Context Propagation

```rust
// Extract trace context from incoming requests
// Inject trace context into outgoing plugin calls
pub fn extract_trace_context(headers: &HeaderMap) -> Context {
    global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(headers))
    })
}
```

### 2.5 Configuration

```yaml
observability:
  opentelemetry:
    enabled: true
    endpoint: "http://localhost:4317"  # OTLP gRPC endpoint
    protocol: grpc  # or http
    sampling_rate: 1.0
    export_interval_ms: 5000
```

---

## Phase 3: API Flight Recorder (12-15 hours)

### 3.1 Design Storage Backend

**Options:**
1. **SQLite** (simple, local, good for small-medium volume)
2. **PostgreSQL** (robust, for production)
3. **ClickHouse** (high-performance, time-series)

**Recommendation:** Start with SQLite, add PostgreSQL support later.

### 3.2 Database Schema

```sql
-- requests table
CREATE TABLE recorded_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp DATETIME NOT NULL,
    trace_id TEXT,
    protocol TEXT NOT NULL,  -- HTTP, gRPC, WebSocket, GraphQL
    method TEXT,
    path TEXT NOT NULL,
    headers TEXT,  -- JSON
    body TEXT,     -- JSON or raw
    query_params TEXT,  -- JSON

    -- Response data
    response_status INTEGER,
    response_headers TEXT,  -- JSON
    response_body TEXT,
    response_time_ms INTEGER,

    -- Metadata
    mock_matched TEXT,
    plugin_executed TEXT,  -- JSON array
    scenario_mode TEXT,
    tags TEXT,  -- JSON array for filtering

    -- Indexes
    INDEX idx_timestamp (timestamp),
    INDEX idx_path (path),
    INDEX idx_trace_id (trace_id),
    INDEX idx_protocol (protocol)
);

-- behavior_patterns table (for analysis)
CREATE TABLE behavior_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern_type TEXT NOT NULL,  -- sequence, frequency, error_pattern
    description TEXT,
    first_seen DATETIME,
    last_seen DATETIME,
    occurrence_count INTEGER,
    related_requests TEXT,  -- JSON array of request IDs
    metadata TEXT  -- JSON
);
```

### 3.3 Recorder Implementation

```rust
// src/recorder/mod.rs
use sqlx::{SqlitePool, Row};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordedRequest {
    pub timestamp: DateTime<Utc>,
    pub trace_id: Option<String>,
    pub protocol: Protocol,
    pub method: Option<String>,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub response: RecordedResponse,
    pub metadata: RequestMetadata,
}

pub struct FlightRecorder {
    pool: SqlitePool,
    buffer: Arc<RwLock<Vec<RecordedRequest>>>,
    config: RecorderConfig,
}

impl FlightRecorder {
    pub async fn record(&self, request: RecordedRequest) -> Result<()> {
        // Buffer writes for performance
        let mut buffer = self.buffer.write().await;
        buffer.push(request);

        if buffer.len() >= self.config.buffer_size {
            self.flush_buffer(&mut buffer).await?;
        }
        Ok(())
    }

    pub async fn query(&self, filter: RecordingFilter) -> Result<Vec<RecordedRequest>> {
        // Build dynamic SQL query based on filter
        // Support filtering by time range, path pattern, status codes, etc.
    }
}
```

### 3.4 Recording Middleware

```rust
// Integration with mockforge-http
pub async fn recording_middleware(
    req: Request<Body>,
    next: Next<Body>,
) -> Response {
    let start = Instant::now();
    let req_data = extract_request_data(&req).await;

    let response = next.run(req).await;
    let duration = start.elapsed();

    let recorded = RecordedRequest {
        timestamp: Utc::now(),
        // ... populate fields
        response: extract_response_data(&response),
    };

    if let Some(recorder) = get_global_recorder() {
        recorder.record(recorded).await.ok();
    }

    response
}
```

### 3.5 Replay Analysis Engine

```rust
// src/recorder/analysis.rs
pub struct BehaviorAnalyzer {
    recorder: Arc<FlightRecorder>,
}

impl BehaviorAnalyzer {
    pub async fn detect_patterns(&self, filter: TimeRange) -> Vec<BehaviorPattern> {
        // Analyze sequences (e.g., "always calls /auth before /users")
        // Detect frequency patterns (e.g., "spikes every 5 minutes")
        // Identify error patterns (e.g., "fails after 3 retries")
    }

    pub async fn generate_sequence_diagram(&self, trace_id: &str) -> SequenceDiagram {
        // Generate Mermaid or PlantUML diagrams from recorded requests
    }
}
```

### 3.6 API Endpoints

```rust
// Add to management API
Router::new()
    .route("/recordings/query", post(query_recordings))
    .route("/recordings/export", get(export_recordings))
    .route("/recordings/patterns", get(detect_patterns))
    .route("/recordings/replay/:id", post(replay_request))
    .route("/recordings/stats", get(recording_stats))
```

### 3.7 Configuration

```yaml
observability:
  recorder:
    enabled: true
    storage:
      type: sqlite  # or postgres
      path: "./recordings.db"
      retention_days: 30
    buffer_size: 1000
    exclude_paths:
      - /health
      - /metrics
    max_body_size_kb: 1024
```

---

## Phase 4: Scenario Control Center (10-12 hours)

### 4.1 Scenario Configuration Model

```rust
// src/scenarios/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScenarioMode {
    Healthy,
    Degraded { latency_multiplier: f64 },
    Error { error_rate: f64, status_codes: Vec<u16> },
    Chaos { config: ChaosConfig },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosConfig {
    pub error_rate: f64,
    pub timeout_rate: f64,
    pub latency_chaos: LatencyChaos,
    pub random_errors: bool,
    pub random_slowdowns: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyChaos {
    pub min_ms: u64,
    pub max_ms: u64,
    pub spike_probability: f64,
}
```

### 4.2 Scenario Manager

```rust
// src/scenarios/manager.rs
pub struct ScenarioManager {
    current_mode: Arc<RwLock<ScenarioMode>>,
    latency_config: Arc<RwLock<LatencyConfig>>,
    chaos_engine: Arc<ChaosEngine>,
}

impl ScenarioManager {
    pub async fn switch_mode(&self, mode: ScenarioMode) -> Result<()> {
        info!("Switching scenario mode to: {:?}", mode);
        *self.current_mode.write().await = mode;
        Ok(())
    }

    pub async fn adjust_latency(&self, min_ms: u64, max_ms: u64) {
        let mut config = self.latency_config.write().await;
        config.min = min_ms;
        config.max = max_ms;
    }

    pub async fn get_current_mode(&self) -> ScenarioMode {
        self.current_mode.read().await.clone()
    }
}
```

### 4.3 Chaos Engine

```rust
// src/scenarios/chaos.rs
pub struct ChaosEngine {
    config: Arc<RwLock<ChaosConfig>>,
    rng: Arc<Mutex<rand::rngs::StdRng>>,
}

impl ChaosEngine {
    pub fn should_inject_error(&self) -> bool {
        let config = self.config.blocking_read();
        let mut rng = self.rng.blocking_lock();
        rng.gen_bool(config.error_rate)
    }

    pub fn generate_chaos_latency(&self) -> Duration {
        let config = self.config.blocking_read();
        let mut rng = self.rng.blocking_lock();

        // Add random spike
        if rng.gen_bool(config.latency_chaos.spike_probability) {
            Duration::from_millis(rng.gen_range(
                config.latency_chaos.min_ms..config.latency_chaos.max_ms
            ))
        } else {
            Duration::from_millis(rng.gen_range(0..50))
        }
    }

    pub fn generate_error_response(&self) -> (StatusCode, String) {
        let status_codes = vec![500, 502, 503, 504, 429];
        let mut rng = self.rng.blocking_lock();
        let status = status_codes[rng.gen_range(0..status_codes.len())];

        (
            StatusCode::from_u16(status).unwrap(),
            json!({ "error": "Chaos mode triggered", "code": status }).to_string()
        )
    }
}
```

### 4.4 Scenario Middleware

```rust
// Integration with mockforge-http
pub async fn scenario_middleware(
    State(scenario_mgr): State<Arc<ScenarioManager>>,
    req: Request<Body>,
    next: Next<Body>,
) -> Response {
    let mode = scenario_mgr.get_current_mode().await;

    match mode {
        ScenarioMode::Healthy => next.run(req).await,

        ScenarioMode::Degraded { latency_multiplier } => {
            let delay = Duration::from_millis(
                (50.0 * latency_multiplier) as u64
            );
            tokio::time::sleep(delay).await;
            next.run(req).await
        }

        ScenarioMode::Error { error_rate, status_codes } => {
            if rand::random::<f64>() < error_rate {
                let status = status_codes[rand::random::<usize>() % status_codes.len()];
                return Response::builder()
                    .status(status)
                    .body(Body::from("Simulated error"))
                    .unwrap();
            }
            next.run(req).await
        }

        ScenarioMode::Chaos { config } => {
            if scenario_mgr.chaos_engine.should_inject_error() {
                let (status, body) = scenario_mgr.chaos_engine.generate_error_response();
                return Response::builder()
                    .status(status)
                    .body(Body::from(body))
                    .unwrap();
            }

            let chaos_latency = scenario_mgr.chaos_engine.generate_chaos_latency();
            tokio::time::sleep(chaos_latency).await;

            next.run(req).await
        }
    }
}
```

### 4.5 Control API

```rust
// Add to management API
Router::new()
    .route("/scenarios/mode", get(get_current_mode))
    .route("/scenarios/mode", put(set_scenario_mode))
    .route("/scenarios/latency", put(adjust_latency))
    .route("/scenarios/chaos/config", get(get_chaos_config))
    .route("/scenarios/chaos/config", put(update_chaos_config))
    .route("/scenarios/presets", get(list_presets))
    .route("/scenarios/presets/:name", post(apply_preset))
```

### 4.6 Configuration Presets

```yaml
# Example preset configs
scenarios:
  presets:
    production:
      mode: healthy

    staging:
      mode: degraded
      latency_multiplier: 1.5

    high-load:
      mode: degraded
      latency_multiplier: 3.0
      error_rate: 0.05

    network-issues:
      mode: error
      error_rate: 0.15
      status_codes: [502, 503, 504]

    chaos:
      mode: chaos
      error_rate: 0.1
      timeout_rate: 0.05
      latency_min_ms: 0
      latency_max_ms: 5000
      spike_probability: 0.2
```

---

## Phase 5: Admin UI Extensions (8-10 hours)

### 5.1 Scenario Control Center Component

```typescript
// ui/src/components/scenarios/ScenarioControlCenter.tsx
export function ScenarioControlCenter() {
  const [currentMode, setCurrentMode] = useState<ScenarioMode>('healthy');
  const [latencyConfig, setLatencyConfig] = useState({ min: 0, max: 100 });
  const [chaosConfig, setChaosConfig] = useState<ChaosConfig>({});

  return (
    <div className="space-y-6">
      {/* Mode Selector */}
      <div className="grid grid-cols-4 gap-4">
        <ModeCard
          mode="healthy"
          active={currentMode === 'healthy'}
          onClick={() => switchMode('healthy')}
        />
        <ModeCard
          mode="degraded"
          active={currentMode === 'degraded'}
          onClick={() => switchMode('degraded')}
        />
        <ModeCard
          mode="error"
          active={currentMode === 'error'}
          onClick={() => switchMode('error')}
        />
        <ModeCard
          mode="chaos"
          active={currentMode === 'chaos'}
          onClick={() => switchMode('chaos')}
        />
      </div>

      {/* Latency Control Knobs */}
      <LatencyControls
        config={latencyConfig}
        onChange={setLatencyConfig}
      />

      {/* Chaos Configuration */}
      {currentMode === 'chaos' && (
        <ChaosConfigPanel
          config={chaosConfig}
          onChange={setChaosConfig}
        />
      )}

      {/* Preset Selector */}
      <PresetSelector onApply={applyPreset} />

      {/* Live Impact Metrics */}
      <ImpactMetrics mode={currentMode} />
    </div>
  );
}
```

### 5.2 Live Metrics Dashboard Enhancements

```typescript
// ui/src/components/metrics/LiveMetricsDashboard.tsx
export function LiveMetricsDashboard() {
  const { metrics, isLive } = useLiveMetrics();

  return (
    <div className="grid grid-cols-2 gap-6">
      {/* Real-time request volume chart */}
      <RequestVolumeChart data={metrics.volume} />

      {/* Latency distribution histogram */}
      <LatencyHistogram data={metrics.latency} />

      {/* Plugin execution stats */}
      <PluginMetrics data={metrics.plugins} />

      {/* Error rate gauge */}
      <ErrorRateGauge rate={metrics.errorRate} />

      {/* Active scenario indicator */}
      <ScenarioIndicator mode={metrics.scenario} />
    </div>
  );
}
```

### 5.3 Recording Viewer Component

```typescript
// ui/src/components/recordings/RecordingViewer.tsx
export function RecordingViewer() {
  const [recordings, setRecordings] = useState([]);
  const [selectedRecording, setSelectedRecording] = useState(null);

  return (
    <div className="grid grid-cols-3 gap-4">
      {/* Recording list with filters */}
      <RecordingList
        recordings={recordings}
        onSelect={setSelectedRecording}
      />

      {/* Recording detail view */}
      <RecordingDetail recording={selectedRecording} />

      {/* Behavior analysis */}
      <BehaviorAnalysis patterns={detectPatterns(recordings)} />
    </div>
  );
}
```

### 5.4 WebSocket Integration for Live Updates

```typescript
// ui/src/hooks/useLiveMetrics.ts
export function useLiveMetrics() {
  const [metrics, setMetrics] = useState({});
  const ws = useWebSocket('/ws/metrics');

  useEffect(() => {
    ws.on('metrics_update', (data) => {
      setMetrics(data);
    });
  }, [ws]);

  return { metrics, isLive: ws.connected };
}
```

---

## Phase 6: Testing & Documentation (6-8 hours)

### 6.1 Unit Tests

- Test each component in isolation
- Mock external dependencies
- Test edge cases and error handling

### 6.2 Integration Tests

```rust
#[tokio::test]
async fn test_scenario_mode_switching() {
    let mgr = ScenarioManager::new();

    // Test mode switching
    mgr.switch_mode(ScenarioMode::Degraded { latency_multiplier: 2.0 }).await.unwrap();
    assert!(matches!(mgr.get_current_mode().await, ScenarioMode::Degraded { .. }));

    // Test middleware applies latency
    let start = Instant::now();
    // ... make request
    assert!(start.elapsed() >= Duration::from_millis(100));
}
```

### 6.3 Documentation

**Create:**
- `docs/observability/PROMETHEUS.md` - Prometheus setup and metrics reference
- `docs/observability/OPENTELEMETRY.md` - OpenTelemetry configuration guide
- `docs/observability/RECORDING.md` - Flight recorder usage and query API
- `docs/observability/SCENARIOS.md` - Scenario control guide with examples
- `docs/observability/QUICKSTART.md` - End-to-end setup guide

---

## Configuration Example

```yaml
# mockforge.yaml
observability:
  # Prometheus metrics
  prometheus:
    enabled: true
    port: 9090
    path: /metrics

  # OpenTelemetry tracing
  opentelemetry:
    enabled: true
    endpoint: "http://localhost:4317"
    sampling_rate: 1.0

  # Flight recorder
  recorder:
    enabled: true
    storage:
      type: sqlite
      path: "./recordings.db"
      retention_days: 30
    buffer_size: 1000

  # Scenario control
  scenarios:
    enabled: true
    default_mode: healthy
    presets:
      production: { mode: healthy }
      staging: { mode: degraded, latency_multiplier: 1.5 }
      chaos:
        mode: chaos
        error_rate: 0.1
        latency_min_ms: 0
        latency_max_ms: 5000
```

---

## Success Metrics

After implementation, MockForge will provide:

1. **Comprehensive Observability**
   - ✅ Prometheus metrics exportable to Grafana
   - ✅ OpenTelemetry traces viewable in Jaeger/Tempo
   - ✅ Complete request/response recording and replay
   - ✅ Behavior pattern analysis

2. **Live Control**
   - ✅ Real-time scenario mode switching
   - ✅ Dynamic latency adjustment
   - ✅ Chaos engineering capabilities
   - ✅ One-click preset application

3. **Developer Experience**
   - ✅ Visual control center in Admin UI
   - ✅ Live metrics dashboard
   - ✅ Recording browser with search/filter
   - ✅ Behavior analysis tools

---

## Competitive Advantage

**No other mock platform offers:**
- Real-time scenario switching from UI
- Comprehensive API flight recorder
- Built-in chaos engineering
- Multi-protocol observability (HTTP + gRPC + WebSocket + GraphQL)
- Behavior pattern analysis

**This positions MockForge as a simulation lab, not just a mocker.**

---

## Next Steps

1. **Review & Approve** this plan
2. **Set up development branches** for each phase
3. **Start with Phase 1** (Prometheus) as foundation
4. **Iterate through phases** with testing at each step
5. **Release incrementally** to get early feedback

---

**Ready to transform MockForge into an observability powerhouse!** 🚀📊
