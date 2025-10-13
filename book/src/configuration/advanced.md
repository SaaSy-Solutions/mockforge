# Advanced Options

MockForge provides extensive advanced configuration options for enterprise-grade API mocking, testing, and chaos engineering scenarios. This guide covers sophisticated features like traffic shaping, time travel, ML-based anomaly detection, multi-tenancy, and advanced orchestration.

## Traffic Shaping and Bandwidth Control

MockForge supports advanced traffic shaping beyond simple latency simulation, including bandwidth throttling and burst control.

### Bandwidth Throttling

Configure bandwidth limits to simulate network constraints:

```yaml
# mockforge.yaml
traffic_shaping:
  bandwidth:
    enabled: true
    max_bytes_per_sec: 1024000  # 1MB/s
    burst_capacity_bytes: 1048576  # 1MB burst allowance

    # Tag-based overrides for specific routes
    tag_overrides:
      premium: 5242880  # 5MB/s for premium routes
      admin: 0  # Unlimited for admin routes
```

### Packet Loss Simulation

Simulate network unreliability with configurable packet loss:

```yaml
traffic_shaping:
  packet_loss:
    enabled: true
    loss_rate: 0.05  # 5% packet loss
    burst_loss_probability: 0.1  # 10% chance of burst loss
    burst_length: 5  # 5 consecutive packets lost in burst

    # Route-specific overrides
    route_overrides:
      "/api/health": 0.0  # No loss for health checks
      "/api/slow/*": 0.2  # 20% loss for slow endpoints
```

### Environment Variables

```bash
# Bandwidth throttling
MOCKFORGE_TRAFFIC_SHAPING_BANDWIDTH_ENABLED=true
MOCKFORGE_TRAFFIC_SHAPING_BANDWIDTH_MAX_BYTES_PER_SEC=1024000
MOCKFORGE_TRAFFIC_SHAPING_BANDWIDTH_BURST_CAPACITY=1048576

# Packet loss
MOCKFORGE_TRAFFIC_SHAPING_PACKET_LOSS_ENABLED=true
MOCKFORGE_TRAFFIC_SHAPING_PACKET_LOSS_RATE=0.05
```

## Time Travel and Temporal Testing

MockForge's time travel capabilities allow testing time-dependent behavior without waiting for real time to pass.

### Virtual Clock Configuration

```yaml
# mockforge.yaml
time_travel:
  enabled: true
  initial_time: "2024-01-01T00:00:00Z"
  scale_factor: 1.0  # 1.0 = real time, 2.0 = 2x speed

  # Scheduled time jumps
  schedule:
    - at: "2024-01-01T01:00:00Z"
      jump_to: "2024-01-01T06:00:00Z"
    - at: "2024-01-01T12:00:00Z"
      advance_by: "1d"
```

### Time Travel API

Control time programmatically through the Admin UI or REST API:

```bash
# Set virtual time
curl -X POST http://localhost:9080/api/v2/time/set \
  -H "Content-Type: application/json" \
  -d '{"time": "2024-01-01T12:00:00Z"}'

# Advance time
curl -X POST http://localhost:9080/api/v2/time/advance \
  -H "Content-Type: application/json" \
  -d '{"duration": "1h"}'

# Enable/disable time travel
curl -X POST http://localhost:9080/api/v2/time/enable \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

### Testing Time-Dependent Logic

```yaml
# Example: Testing token expiry
routes:
  - path: /api/auth/validate
    method: GET
    response:
      status: 200
      condition: "time_travel.now < time_travel.parse('2024-01-01T02:00:00Z')"
      body: |
        {
          "valid": true,
          "expires_at": "2024-01-01T02:00:00Z"
        }

  - path: /api/auth/validate
    method: GET
    response:
      status: 401
      condition: "time_travel.now >= time_travel.parse('2024-01-01T02:00:00Z')"
      body: |
        {
          "error": "Token expired",
          "expired_at": "2024-01-01T02:00:00Z"
        }
```

## ML-Based Anomaly Detection

MockForge integrates machine learning for intelligent anomaly detection in system behavior.

### Anomaly Detection Configuration

```yaml
# mockforge.yaml
anomaly_detection:
  enabled: true

  # Detection parameters
  config:
    std_dev_threshold: 3.0  # Standard deviations for anomaly
    min_baseline_samples: 30  # Minimum samples for baseline
    moving_average_window: 10  # Smoothing window
    enable_seasonal: true  # Account for seasonal patterns
    seasonal_period: 24  # Hours in daily cycle
    sensitivity: 0.7  # Detection sensitivity (0.0-1.0)

  # Metrics to monitor
  monitored_metrics:
    - name: response_time_ms
      baseline_samples: 100
      alert_on_anomaly: true
      severity_threshold: high

    - name: error_rate
      baseline_samples: 50
      alert_on_anomaly: true
      severity_threshold: medium

    - name: request_throughput
      baseline_samples: 100
      alert_on_anomaly: false
      severity_threshold: high

  # Collective anomaly detection
  collective_detection:
    enabled: true
    metric_groups:
      - name: api_health
        metrics:
          - response_time_ms
          - error_rate
          - request_throughput
        min_affected_metrics: 2
```

### Anomaly Response Actions

Configure automatic responses to detected anomalies:

```yaml
anomaly_detection:
  response_actions:
    - trigger: high_severity_anomaly
      action: circuit_breaker
      duration: 5m
      routes: ["/api/*"]

    - trigger: collective_anomaly
      action: failover
      target: backup_service
      routes: ["/api/critical/*"]

    - trigger: performance_degradation
      action: scale_up
      threshold: 2.0  # 2x normal response time
```

## Chaos Mesh Integration

Integrate with Chaos Mesh for Kubernetes-native chaos engineering.

### Chaos Mesh Configuration

```yaml
# mockforge.yaml
chaos_mesh:
  enabled: true
  api_url: https://kubernetes.default.svc
  namespace: chaos-testing

  # Default experiment settings
  defaults:
    mode: one  # one, all, fixed, fixed-percent, random-max-percent
    duration: 5m

  # Pre-configured experiments
  experiments:
    - name: pod-kill-test
      type: PodChaos
      action: pod-kill
      selector:
        namespaces:
          - production
        label_selectors:
          app: api-gateway
          tier: backend
      mode: one
      duration: 30s
      schedule: "*/5 * * * *"  # Every 5 minutes

    - name: network-latency-test
      type: NetworkChaos
      action: delay
      selector:
        namespaces:
          - production
        label_selectors:
          app: database
      delay:
        latency: 100ms
        jitter: 10ms
        correlation: "50"
      duration: 3m

    - name: cpu-stress-test
      type: StressChaos
      selector:
        namespaces:
          - staging
        label_selectors:
          app: worker-service
      stressors:
        cpu_workers: 4
        cpu_load: 80
      duration: 10m
```

### Chaos Experiment Orchestration

```yaml
# Orchestrate chaos experiments with MockForge scenarios
orchestration:
  name: chaos-testing-workflow
  description: Comprehensive chaos testing with monitoring

  steps:
    - name: baseline_measurement
      type: metrics_collection
      duration: 5m

    - name: pod_failure_injection
      type: chaos_mesh
      experiment: pod-kill-test
      wait_for_completion: true

    - name: anomaly_detection
      type: ml_detection
      metrics: [response_time_ms, error_rate]
      alert_threshold: high

    - name: network_chaos
      type: chaos_mesh
      experiment: network-latency-test

    - name: recovery_verification
      type: health_check
      endpoints: ["/api/health", "/api/status"]
      timeout: 30s
```

## Multi-Tenancy Configuration

MockForge supports multi-tenant deployments with configurable plans and quotas.

### Tenant Plans Configuration

```yaml
# mockforge.yaml
multi_tenancy:
  enabled: true

  # Define tenant plans
  plans:
    free:
      quotas:
        max_scenarios: 5
        max_concurrent_executions: 1
        max_orchestrations: 3
        max_templates: 5
        max_requests_per_minute: 50
        max_storage_mb: 50
        max_users: 1
        max_experiment_duration_secs: 600

      permissions:
        can_create_scenarios: true
        can_execute_scenarios: true
        can_view_observability: false
        can_manage_resilience: false
        can_use_advanced_features: false
        can_integrate_external: false
        can_use_ml_features: false
        can_manage_users: false

    professional:
      quotas:
        max_scenarios: 100
        max_concurrent_executions: 20
        max_orchestrations: 50
        max_templates: 100
        max_requests_per_minute: 1000
        max_storage_mb: 5000
        max_users: 25
        max_experiment_duration_secs: 14400

      permissions:
        can_create_scenarios: true
        can_execute_scenarios: true
        can_view_observability: true
        can_manage_resilience: true
        can_use_advanced_features: true
        can_integrate_external: true
        can_use_ml_features: true
        can_manage_users: true

  # Default tenants
  tenants:
    - name: acme-corp
      plan: professional
      enabled: true
      metadata:
        organization: Acme Corporation
        contact: admin@acme.com
        environment: production
```

### Tenant Isolation

Configure tenant-specific resources and isolation:

```yaml
multi_tenancy:
  isolation:
    # Database isolation
    database:
      separate_schemas: true
      schema_prefix: "tenant_"

    # File system isolation
    filesystem:
      tenant_directories: true
      shared_resources: ["global-templates"]

    # Network isolation
    network:
      tenant_subdomains: true
      shared_ports: [80, 443]
```

## Plugin System Configuration

Advanced plugin configuration for extending MockForge functionality.

### Plugin Registry Configuration

```yaml
# mockforge.yaml
plugins:
  enabled: true

  # Plugin registry settings
  registry:
    auto_discover: true
    plugin_dirs:
      - /etc/mockforge/plugins
      - ~/.mockforge/plugins
      - ./custom-plugins

  # Built-in plugins
  builtin:
    - id: custom-fault-injector
      enabled: true
      config:
        fault_probability: 0.1
        default_timeout_ms: 5000

    - id: metrics-collector
      enabled: true
      config:
        export_interval_secs: 60
        buffer_size: 1000

  # Custom plugins
  custom:
    - id: database-fault-injector
      enabled: true
      path: /etc/mockforge/plugins/database_fault.so
      config:
        connection_timeout_ms: 5000
        query_timeout_ms: 30000
        fault_types:
          - connection_timeout
          - query_error
          - slow_query
          - deadlock

    - id: prometheus-exporter
      enabled: true
      path: /etc/mockforge/plugins/prometheus.so
      config:
        export_port: 9090
        metrics_path: /metrics
        include_labels:
          - tenant_id
          - scenario_id
          - experiment_type

  # Plugin hooks
  hooks:
    - type: logging
      enabled: true
      config:
        log_level: info
        include_context: true

    - type: metrics
      enabled: true
      config:
        track_execution_time: true
        track_success_rate: true

    - type: rate_limiting
      enabled: true
      config:
        max_executions_per_minute: 100
        burst_size: 20
```

### Plugin Security

Configure plugin execution security:

```yaml
plugins:
  security:
    # Sandbox configuration
    sandbox:
      enabled: true
      memory_limit_mb: 100
      cpu_limit_percent: 50
      network_access: deny
      filesystem_access: restricted

    # Plugin signing
    signing:
      enabled: true
      trusted_keys:
        - "mockforge-official"
        - "enterprise-customer-key"

    # Resource limits
    limits:
      max_plugins_per_tenant: 10
      max_plugin_memory_mb: 50
      max_plugin_timeout_secs: 30
```

## Advanced Orchestration

Complex scenario orchestration with conditional logic and dependencies.

### Orchestration Configuration

```yaml
# mockforge.yaml
orchestration:
  name: advanced-chaos-scenario
  description: Comprehensive chaos test with ML detection and multi-tenancy

  # Tenant context
  tenant_id: production-tenant

  # Enable advanced features
  features:
    anomaly_detection: true
    chaos_mesh_integration: true
    plugin_execution: true
    time_travel: true

  # Complex step orchestration
  steps:
    # Step 1: Baseline measurement
    - name: collect_baseline
      type: custom
      plugin: metrics-collector
      config:
        duration: 5m
        metrics:
          - response_time_ms
          - error_rate
          - request_throughput

    # Step 2: Time travel setup
    - name: setup_time_travel
      type: time_travel
      config:
        enabled: true
        initial_time: "2024-01-01T00:00:00Z"

    # Step 3: Chaos Mesh pod kill
    - name: pod_chaos
      type: chaos_mesh
      experiment: pod-kill-test
      wait_for_completion: true
      depends_on: ["collect_baseline"]

    # Step 4: Monitor for anomalies
    - name: detect_anomalies
      type: ml_detection
      metrics:
        - response_time_ms
        - error_rate
      alert_threshold: high
      depends_on: ["pod_chaos"]

    # Step 5: Custom fault injection
    - name: database_fault
      type: plugin
      plugin: database-fault-injector
      config:
        fault_type: slow_query
        latency_ms: 1000
        duration: 2m
      depends_on: ["detect_anomalies"]

    # Step 6: Network chaos
    - name: network_latency
      type: chaos_mesh
      experiment: network-latency-test
      depends_on: ["database_fault"]

    # Step 7: Final analysis
    - name: analyze_results
      type: custom
      plugin: prometheus-exporter
      config:
        export_metrics: true
        generate_report: true
      depends_on: ["network_latency"]

  # Conditional execution
  conditions:
    - name: high_load_detected
      expression: "metrics.request_throughput > 1000"
      actions:
        - skip_step: "network_latency"
        - enable_step: "load_shedding"

    - name: anomaly_critical
      expression: "anomaly.severity == 'critical'"
      actions:
        - abort_orchestration: true
        - send_alert: "critical_anomaly"

  # Assertions and validations
  assertions:
    - metric: response_time_ms
      operator: less_than
      value: 1000
      severity: high

    - metric: error_rate
      operator: less_than
      value: 0.05
      severity: critical

    - metric: anomaly_count
      operator: equals
      value: 0
      severity: medium

  # Cleanup configuration
  cleanup:
    - delete_chaos_mesh_experiments: true
    - export_metrics: true
    - send_notifications: true
    - reset_time_travel: true
```

## Observability Integration

Advanced observability with Prometheus, OpenTelemetry, and alerting.

### Prometheus Integration

```yaml
# mockforge.yaml
observability:
  prometheus:
    enabled: true
    port: 9090
    path: /metrics

    # Custom metrics
    custom_metrics:
      - name: mockforge_scenario_duration
        type: histogram
        description: "Time spent executing scenarios"
        labels: ["scenario_name", "tenant_id"]

      - name: mockforge_anomaly_detected
        type: counter
        description: "Number of anomalies detected"
        labels: ["severity", "metric_name"]

  opentelemetry:
    enabled: true
    endpoint: http://otel-collector:4317

    # Tracing configuration
    tracing:
      service_name: mockforge
      service_version: "1.0.0"
      sample_rate: 0.1

    # Metrics configuration
    metrics:
      export_interval: 30s
      resource_attributes:
        service.name: mockforge
        service.version: "1.0.0"
```

### Alerting Configuration

```yaml
observability:
  alerts:
    - name: anomaly_detected
      condition: "anomaly.severity >= 'high'"
      channels:
        - slack
        - email
        - webhook
      cooldown: 5m

    - name: quota_exceeded
      condition: "tenant.usage >= tenant.quota * 0.9"
      channels:
        - email
      cooldown: 1h

    - name: service_degradation
      condition: "metrics.response_time_p95 > 2000"
      channels:
        - slack
        - pager_duty
      cooldown: 10m

  # Alert channels configuration
  channels:
    slack:
      webhook_url: "${SLACK_WEBHOOK_URL}"
      channel: "#alerts"
      username: "MockForge Alert"

    email:
      smtp_server: "smtp.company.com"
      smtp_port: 587
      username: "${SMTP_USERNAME}"
      password: "${SMTP_PASSWORD}"
      from: "alerts@mockforge.company.com"
      to: ["devops@company.com", "engineering@company.com"]

    webhook:
      url: "https://alert-manager.company.com/webhook"
      headers:
        Authorization: "Bearer ${WEBHOOK_TOKEN}"
      method: POST
```

## Security and Encryption

Advanced security features for enterprise deployments.

### Encryption Configuration

```yaml
# mockforge.yaml
security:
  encryption:
    enabled: true

    # Key management
    keys:
      default:
        algorithm: AES-256-GCM
        key_rotation_days: 30

      sensitive:
        algorithm: AES-256-GCM
        hsm_integration: true

    # Data encryption
    data_encryption:
      fixtures: true
      logs: true
      configuration: false

    # TLS configuration
    tls:
      enabled: true
      certificate_file: /etc/ssl/mockforge.crt
      private_key_file: /etc/ssl/mockforge.key
      client_auth: optional
```

### Authentication and Authorization

```yaml
security:
  auth:
    # JWT configuration
    jwt:
      enabled: true
      secret: "${JWT_SECRET}"
      issuer: "mockforge"
      audience: "mockforge-users"
      algorithms: ["HS256", "RS256"]

    # OAuth2 integration
    oauth2:
      enabled: true
      provider: keycloak
      client_id: "${OAUTH2_CLIENT_ID}"
      client_secret: "${OAUTH2_CLIENT_SECRET}"
      token_url: "https://auth.company.com/token"
      userinfo_url: "https://auth.company.com/userinfo"

    # Role-based access control
    rbac:
      enabled: true
      roles:
        admin:
          permissions:
            - "scenarios:*"
            - "tenants:*"
            - "system:*"

        developer:
          permissions:
            - "scenarios:read"
            - "scenarios:execute"
            - "fixtures:*"

        viewer:
          permissions:
            - "scenarios:read"
            - "fixtures:read"
            - "metrics:read"
```

## Performance Tuning

Advanced performance configuration for high-throughput scenarios.

### Resource Limits

```yaml
# mockforge.yaml
performance:
  # Thread pool configuration
  thread_pool:
    http_workers: 16
    background_workers: 4
    max_blocking_threads: 512

  # Memory management
  memory:
    max_heap_size_mb: 2048
    gc_threshold_mb: 1024
    cache_size_mb: 512

  # Connection pooling
  connections:
    max_http_connections: 1000
    connection_timeout_secs: 30
    keep_alive_secs: 300

  # Request processing
  requests:
    max_concurrent_requests: 10000
    request_timeout_secs: 60
    buffer_size_kb: 64
```

### Caching Configuration

```yaml
performance:
  caching:
    # Response caching
    responses:
      enabled: true
      max_size_mb: 100
      ttl_secs: 300
      compression: true

    # Template caching
    templates:
      enabled: true
      max_entries: 1000
      ttl_secs: 3600

    # Plugin caching
    plugins:
      enabled: true
      max_instances: 10
      preload: ["metrics-collector", "template-renderer"]
```

### Monitoring and Profiling

```yaml
performance:
  monitoring:
    # Performance metrics
    metrics:
      enabled: true
      interval_secs: 30
      export_format: prometheus

    # Profiling
    profiling:
      enabled: true
      sample_rate: 1000  # 1000 Hz
      max_stack_depth: 64

    # Health checks
    health_checks:
      enabled: true
      interval_secs: 60
      failure_threshold: 3
```

## Environment Variables

Advanced configuration through environment variables:

```bash
# Traffic shaping
MOCKFORGE_TRAFFIC_SHAPING_ENABLED=true
MOCKFORGE_BANDWIDTH_MAX_BYTES_PER_SEC=1024000
MOCKFORGE_PACKET_LOSS_RATE=0.05

# Time travel
MOCKFORGE_TIME_TRAVEL_ENABLED=true
MOCKFORGE_VIRTUAL_TIME_SCALE=1.0

# Anomaly detection
MOCKFORGE_ANOMALY_DETECTION_ENABLED=true
MOCKFORGE_ANOMALY_SENSITIVITY=0.7

# Chaos Mesh
MOCKFORGE_CHAOS_MESH_ENABLED=true
MOCKFORGE_CHAOS_MESH_NAMESPACE=chaos-testing

# Multi-tenancy
MOCKFORGE_MULTI_TENANCY_ENABLED=true
MOCKFORGE_DEFAULT_TENANT_PLAN=professional

# Plugins
MOCKFORGE_PLUGINS_ENABLED=true
MOCKFORGE_PLUGIN_AUTO_DISCOVER=true

# Observability
MOCKFORGE_PROMETHEUS_ENABLED=true
MOCKFORGE_OPENTELEMETRY_ENABLED=true

# Security
MOCKFORGE_ENCRYPTION_ENABLED=true
MOCKFORGE_JWT_ENABLED=true
MOCKFORGE_TLS_ENABLED=true

# Performance
MOCKFORGE_MAX_CONCURRENT_REQUESTS=10000
MOCKFORGE_CACHE_ENABLED=true
MOCKFORGE_PROFILING_ENABLED=true
```

## Best Practices

### Configuration Management

1. **Version Control**: Keep all configuration files in version control
2. **Environment Separation**: Use different configurations for dev/staging/prod
3. **Secrets Management**: Never commit secrets to version control
4. **Validation**: Always validate configurations before deployment

### Security

1. **Principle of Least Privilege**: Grant minimal required permissions
2. **Network Security**: Use firewalls and network policies
3. **Audit Logging**: Enable comprehensive audit logging
4. **Regular Updates**: Keep MockForge and dependencies updated

### Performance

1. **Resource Monitoring**: Monitor resource usage continuously
2. **Load Testing**: Test configurations under load
3. **Caching Strategy**: Configure appropriate caching for your use case
4. **Scalability Planning**: Plan for growth and scale accordingly

### Troubleshooting

1. **Debug Logging**: Enable debug logging for troubleshooting
2. **Metrics Collection**: Use observability tools for monitoring
3. **Configuration Validation**: Validate configurations regularly
4. **Incremental Changes**: Make configuration changes incrementally

This comprehensive guide covers MockForge's advanced configuration options for enterprise-grade API mocking and chaos engineering scenarios.