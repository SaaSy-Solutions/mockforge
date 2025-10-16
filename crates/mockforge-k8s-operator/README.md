# MockForge Kubernetes Operator

Kubernetes operator for managing chaos engineering orchestrations as Custom Resource Definitions (CRDs).

This crate provides a Kubernetes-native way to define, schedule, and execute chaos engineering experiments across your cluster. It integrates with the MockForge chaos engineering framework to provide GitOps-style chaos testing with full Kubernetes integration.

## Features

- **Kubernetes CRDs**: Define chaos orchestrations as Kubernetes resources
- **GitOps Integration**: Version control your chaos experiments
- **Scheduled Executions**: Cron-based automatic chaos testing
- **Multi-Service Targeting**: Apply chaos to specific Kubernetes services
- **Lifecycle Hooks**: Pre/post chaos execution hooks
- **Assertions & Validation**: Built-in success criteria validation
- **Metrics & Monitoring**: Prometheus metrics for operator health
- **Admission Webhooks**: Validate chaos orchestrations before execution
- **RBAC Integration**: Kubernetes-native access control

## Installation

### Using Helm

```bash
# Add MockForge Helm repository
helm repo add mockforge https://charts.mockforge.io
helm repo update

# Install the operator
helm install mockforge-operator mockforge/mockforge-operator

# Install with custom values
helm install mockforge-operator mockforge/mockforge-operator \
  --set image.tag=v0.1.0 \
  --set resources.limits.memory=512Mi
```

### Manual Installation

```bash
# Apply CRDs
kubectl apply -f https://raw.githubusercontent.com/SaaSy-Solutions/mockforge/main/k8s/crd/chaosorchestration.yaml

# Deploy operator
kubectl apply -f https://raw.githubusercontent.com/SaaSy-Solutions/mockforge/main/k8s/operator/deployment.yaml

# Verify installation
kubectl get pods -l app=mockforge-operator
```

## Custom Resource Definition

### ChaosOrchestration

Define chaos experiments as Kubernetes resources:

```yaml
apiVersion: mockforge.io/v1
kind: ChaosOrchestration
metadata:
  name: api-failure-test
  namespace: chaos-testing
spec:
  name: "API Failure Resilience Test"
  description: "Test API resilience against service failures"

  # Optional: Schedule automatic execution
  schedule: "0 2 * * *"  # Daily at 2 AM

  # Global variables
  variables:
    api_service: "api-server"
    test_duration: 300

  # Target services
  targetServices:
    - name: api-server
      namespace: production
      selector:
        app: api-server
    - name: database
      namespace: production
      selector:
        app: postgres

  # Orchestration steps
  steps:
    - name: "baseline-measurement"
      scenario: "baseline"
      durationSeconds: 60

    - name: "inject-failures"
      scenario: "service_failure"
      durationSeconds: 180
      parameters:
        failureRate: 0.1
        services: ["{{api_service}}"]

    - name: "network-chaos"
      scenario: "network_partition"
      durationSeconds: 120
      parameters:
        partitionType: "partial"
        affectedServices: ["api-server"]

    - name: "recovery-test"
      scenario: "recovery_validation"
      durationSeconds: 60

  # Lifecycle hooks
  hooks:
    - name: "pre-chaos"
      type: "pre"
      actions:
        - type: "http"
          url: "https://slack.com/api/chat.postMessage"
          method: "POST"
          body: '{"text": "Starting chaos experiment: {{.Name}}"}'

    - name: "post-chaos"
      type: "post"
      actions:
        - type: "metric"
          name: "chaos_experiment_completed"
          value: 1

  # Success criteria
  assertions:
    - name: "api-availability"
      type: "metric"
      metric: "http_request_duration_seconds"
      operator: "lt"
      value: 5.0
      description: "API response time should stay under 5 seconds"

    - name: "error-rate"
      type: "metric"
      metric: "http_requests_total"
      labels: 'code=~"5.."'
      operator: "lt"
      value: 0.05
      description: "Error rate should stay below 5%"

status:
  phase: "Pending"
  progress: "0/4 steps completed"
  startTime: "2024-01-01T00:00:00Z"
  conditions:
    - type: "Ready"
      status: "True"
      lastTransitionTime: "2024-01-01T00:00:00Z"
```

## Usage Examples

### Basic Chaos Orchestration

```yaml
apiVersion: mockforge.io/v1
kind: ChaosOrchestration
metadata:
  name: simple-failure-test
spec:
  name: "Simple Service Failure Test"
  steps:
    - name: "inject-pod-failure"
      scenario: "pod_kill"
      durationSeconds: 30
      parameters:
        targetPods: "app=api-server"
        killCount: 2
```

### Network Chaos

```yaml
apiVersion: mockforge.io/v1
kind: ChaosOrchestration
metadata:
  name: network-chaos
spec:
  name: "Network Partition Test"
  steps:
    - name: "network-partition"
      scenario: "network_chaos"
      durationSeconds: 120
      parameters:
        action: "partition"
        sourceSelector: "app=api-server"
        targetSelector: "app=database"
        direction: "both"
```

### Multi-Scenario Orchestration

```yaml
apiVersion: mockforge.io/v1
kind: ChaosOrchestration
metadata:
  name: comprehensive-chaos
spec:
  name: "Comprehensive Chaos Test"
  targetServices:
    - name: web-frontend
      selector: "app=web"
    - name: api-backend
      selector: "app=api"
    - name: database
      selector: "app=db"

  steps:
    - name: "cpu-stress"
      scenario: "resource_stress"
      durationSeconds: 60
      parameters:
        resource: "cpu"
        stressLevel: 80
        targets: ["web-frontend"]

    - name: "memory-pressure"
      scenario: "resource_stress"
      durationSeconds: 60
      parameters:
        resource: "memory"
        stressLevel: 90
        targets: ["api-backend"]

    - name: "network-latency"
      scenario: "network_chaos"
      durationSeconds: 120
      parameters:
        action: "delay"
        delayMs: 500
        jitterMs: 100
        sourceSelector: "app=api"
        targetSelector: "app=db"

    - name: "pod-failures"
      scenario: "pod_chaos"
      durationSeconds: 180
      parameters:
        action: "kill"
        killCount: 1
        intervalSeconds: 30
        targets: ["api-backend"]
```

## Available Scenarios

### Pod Chaos
- **pod_kill**: Randomly kill pods matching selectors
- **pod_restart**: Restart pods without killing them
- **pod_scale**: Scale deployments up/down

### Network Chaos
- **network_delay**: Add latency to network traffic
- **network_loss**: Drop packets randomly
- **network_partition**: Create network partitions
- **network_corruption**: Corrupt network packets

### Resource Chaos
- **cpu_stress**: Consume CPU resources
- **memory_stress**: Consume memory resources
- **disk_stress**: Fill disk space
- **io_stress**: Stress I/O operations

### Application Chaos
- **http_errors**: Inject HTTP error responses
- **service_failure**: Make services unresponsive
- **dependency_failure**: Break service dependencies

## Monitoring & Observability

### Prometheus Metrics

The operator exposes comprehensive metrics:

```prometheus
# Operator health
mockforge_operator_up 1

# Orchestration status
mockforge_orchestration_total{phase="Running"} 3
mockforge_orchestration_total{phase="Completed"} 12
mockforge_orchestration_total{phase="Failed"} 1

# Step execution times
mockforge_step_duration_seconds{name="inject-failures", orchestration="api-test"} 180

# Assertion results
mockforge_assertion_passed_total{assertion="api-availability"} 15
mockforge_assertion_failed_total{assertion="api-availability"} 2
```

### Viewing Orchestration Status

```bash
# List all orchestrations
kubectl get chaosorchestrations

# Get detailed status
kubectl describe chaosorchestration api-failure-test

# View logs
kubectl logs -l app=mockforge-operator

# Check metrics
kubectl port-forward svc/mockforge-operator-metrics 9090:9090
curl http://localhost:9090/metrics
```

## RBAC Configuration

### ClusterRole for Operator

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: mockforge-operator
rules:
- apiGroups: ["mockforge.io"]
  resources: ["chaosorchestrations", "chaosorchestrations/status"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["apps"]
  resources: ["deployments", "statefulsets"]
  verbs: ["get", "list", "watch", "update", "patch"]
- apiGroups: [""]
  resources: ["pods", "services", "configmaps", "secrets"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["networking.k8s.io"]
  resources: ["networkpolicies"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
```

### Admission Webhooks

Enable validation webhooks:

```yaml
apiVersion: admissionregistration.k8s.io/v1
kind: ValidatingWebhookConfiguration
metadata:
  name: mockforge-operator-webhook
webhooks:
- name: chaosorchestration.mockforge.io
  rules:
  - operations: ["CREATE", "UPDATE"]
    apiGroups: ["mockforge.io"]
    apiVersions: ["v1"]
    resources: ["chaosorchestrations"]
  clientConfig:
    service:
      name: mockforge-operator-webhook
      namespace: mockforge-system
    caBundle: <CA_BUNDLE>
  admissionReviewVersions: ["v1"]
  sideEffects: None
```

## Development

### Building the Operator

```bash
# Build the operator binary
cargo build --release --bin mockforge-k8s-operator

# Build Docker image
docker build -t mockforge/operator:v0.1.0 .
```

### Running Locally

```bash
# Use kubeconfig for local development
export KUBECONFIG=~/.kube/config

# Run operator locally (requires cluster access)
cargo run --bin mockforge-k8s-operator
```

### Testing

```bash
# Run unit tests
cargo test

# Run integration tests (requires Kubernetes cluster)
cargo test --test integration -- --nocapture
```

## Configuration

### Operator Configuration

```yaml
# config/config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: mockforge-operator-config
  namespace: mockforge-system
data:
  config.yaml: |
    # Operator settings
    leaderElection:
      enabled: true
      leaseDuration: 15s
      renewDeadline: 10s
      retryPeriod: 2s

    # Chaos settings
    chaos:
      defaultDuration: 300s
      maxConcurrentOrchestrations: 5
      enableReporting: true

    # Metrics settings
    metrics:
      enabled: true
      port: 9090
      path: /metrics

    # Webhook settings
    webhook:
      enabled: true
      port: 9443
      certDir: /tmp/k8s-webhook-server/serving-certs
```

## Troubleshooting

### Common Issues

**Operator not starting:**
```bash
# Check operator logs
kubectl logs -l app=mockforge-operator

# Verify RBAC permissions
kubectl auth can-i create chaosorchestrations --as=system:serviceaccount:mockforge-system:mockforge-operator
```

**CRDs not installed:**
```bash
# Install CRDs
kubectl apply -f k8s/crd/

# Verify CRDs
kubectl get crd | grep mockforge
```

**Orchestration stuck:**
```bash
# Check orchestration status
kubectl describe chaosorchestration <name>

# Check operator events
kubectl get events --sort-by=.metadata.creationTimestamp
```

**Webhook validation failures:**
```bash
# Check webhook configuration
kubectl get validatingwebhookconfigurations

# Check webhook logs
kubectl logs -l app=mockforge-operator-webhook
```

## API Reference

### ChaosOrchestration Spec

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Human-readable name for the orchestration |
| `description` | `string?` | Optional description |
| `schedule` | `string?` | Cron schedule for automatic execution |
| `steps` | `OrchestrationStep[]` | Steps to execute |
| `variables` | `object` | Global variables available to all steps |
| `hooks` | `OrchestrationHook[]` | Lifecycle hooks |
| `assertions` | `OrchestrationAssertion[]` | Success criteria |
| `targetServices` | `TargetService[]` | Kubernetes services to target |

### OrchestrationStep

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Step identifier |
| `scenario` | `string` | Chaos scenario to execute |
| `durationSeconds` | `number?` | How long to run the scenario |
| `delayBeforeSeconds` | `number` | Delay before starting (default: 0) |
| `continueOnFailure` | `boolean` | Continue if step fails (default: false) |
| `parameters` | `object` | Scenario-specific parameters |

## Contributing

See the main [MockForge repository](https://github.com/SaaSy-Solutions/mockforge) for contribution guidelines.

## License

Licensed under MIT OR Apache-2.0
