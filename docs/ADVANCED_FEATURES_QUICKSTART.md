# Advanced Features Quick Start Guide

This guide provides examples and usage instructions for the newly implemented advanced features in MockForge.

## Table of Contents
1. [Real-time Orchestration Visualization](#real-time-orchestration-visualization)
2. [Collaborative Editing](#collaborative-editing)
3. [Version Control](#version-control)
4. [Template Marketplace](#template-marketplace)
5. [ML-based Assertion Generation](#ml-based-assertion-generation)

---

## Real-time Orchestration Visualization

### Overview
Monitor orchestration execution in real-time with live metrics, step progress, and control capabilities.

### Usage

#### In React/TypeScript UI:
```tsx
import { OrchestrationExecutionView } from './pages/OrchestrationExecutionView';

function MyComponent() {
  return <OrchestrationExecutionView orchestrationId="my-orchestration-id" />;
}
```

#### Features Available:
- **Live Progress Tracking**: See which step is currently executing
- **Real-time Metrics**: View request counts, error rates, and latency
- **Execution Control**: Start, pause, resume, stop, or skip steps
- **Visual Indicators**: Color-coded status for each step
- **Failure Alerts**: Immediate notification of failed steps

#### WebSocket Connection:
The component automatically connects to:
```
ws://localhost:8080/api/chaos/orchestration/{id}/ws
```

#### Message Format:
```json
{
  "type": "status_update",
  "data": {
    "status": "running",
    "currentStep": 2,
    "totalSteps": 5,
    "progress": 0.4
  }
}
```

---

## Collaborative Editing

### Overview
Enable multiple users to edit orchestrations simultaneously with real-time synchronization and conflict resolution.

### Usage

#### Wrap Your Editor Component:
```tsx
import { CollaborativeEditor } from './components/collaboration/CollaborativeEditor';

function OrchestrationEditor() {
  const [orchestration, setOrchestration] = useState(initialValue);

  return (
    <CollaborativeEditor
      orchestrationId="orch-123"
      value={orchestration}
      onChange={setOrchestration}
    >
      <YourEditorComponent value={orchestration} />
    </CollaborativeEditor>
  );
}
```

#### Backend Setup (Rust):
```rust
use mockforge_chaos::{CollaborationManager, CollaborationUser};

let manager = CollaborationManager::new();
let session = manager.get_or_create_session("orch-123")?;

let user = CollaborationUser {
    id: "user-1".to_string(),
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
    color: "#FF6B6B".to_string(),
    cursor: None,
    active_field: None,
    joined_at: Utc::now(),
};

session.add_user(user)?;
```

#### Features:
- **Presence Awareness**: See who else is editing
- **Cursor Tracking**: View other users' cursor positions
- **Conflict Resolution**: Automatic merging of concurrent changes
- **Change History**: Track all modifications
- **Notifications**: Join/leave alerts

---

## Version Control

### Overview
Git-like version control for orchestration configurations with branching, commits, and diffs.

### Usage

#### Initialize Repository (Rust):
```rust
use mockforge_chaos::{VersionControlRepository, Commit};

let mut repo = VersionControlRepository::new(
    "orchestration-123".to_string(),
    "/path/to/storage".to_string(),
)?;

// Create a commit
let content = serde_json::json!({
    "name": "My Orchestration",
    "steps": [
        {"name": "step1", "scenario": "network_degradation"}
    ]
});

let commit = repo.commit(
    "Alice".to_string(),
    "alice@example.com".to_string(),
    "Add network degradation step".to_string(),
    &content,
)?;
```

#### Branching:
```rust
// Create a new branch
repo.create_branch("feature/new-scenario".to_string(), None)?;

// Switch to branch
repo.checkout("feature/new-scenario".to_string())?;

// Make changes and commit
let new_content = /* ... */;
repo.commit(
    "Alice".to_string(),
    "alice@example.com".to_string(),
    "Implement new scenario".to_string(),
    &new_content,
)?;
```

#### View Diff:
```rust
let diff = repo.diff(
    commit1_id,
    commit2_id,
)?;

println!("Changes: {} additions, {} deletions, {} modifications",
    diff.stats.additions,
    diff.stats.deletions,
    diff.stats.modifications
);

for change in diff.changes {
    println!("{}: {} {:?} -> {:?}",
        change.change_type,
        change.path,
        change.old_value,
        change.new_value
    );
}
```

#### View History:
```rust
let commits = repo.history(Some(10))?; // Last 10 commits

for commit in commits {
    println!("{} - {} by {} at {}",
        &commit.id[..7],
        commit.message,
        commit.author,
        commit.timestamp
    );
}
```

#### UI Component:
```tsx
import { VersionControlPanel } from './components/version-control/VersionControlPanel';

<VersionControlPanel orchestrationId="orch-123" />
```

---

## Template Marketplace

### Overview
Browse, search, and share chaos orchestration templates with the community.

### Usage

#### Browse Templates (UI):
```tsx
import { TemplateMarketplacePage } from './pages/TemplateMarketplacePage';

function App() {
  return <TemplateMarketplacePage />;
}
```

#### Publish a Template (Rust):
```rust
use mockforge_chaos::{TemplateMarketplace, OrchestrationTemplate, TemplateCategory};

let mut marketplace = TemplateMarketplace::new();

let template = OrchestrationTemplate {
    id: "network-chaos-template".to_string(),
    name: "Network Degradation Test".to_string(),
    description: "Simulates network latency and packet loss".to_string(),
    author: "Alice".to_string(),
    author_email: "alice@example.com".to_string(),
    version: "1.0.0".to_string(),
    category: TemplateCategory::NetworkChaos,
    tags: vec!["network".to_string(), "latency".to_string()],
    content: serde_json::json!({
        "steps": [
            {
                "name": "Introduce Latency",
                "scenario": "network_degradation",
                "duration_seconds": 60
            }
        ]
    }),
    readme: "# Network Degradation Template\n\n...".to_string(),
    // ... other fields
    stats: TemplateStats {
        downloads: 0,
        stars: 0,
        forks: 0,
        rating: 0.0,
        rating_count: 0,
    },
    created_at: Utc::now(),
    updated_at: Utc::now(),
    published: true,
};

marketplace.publish_template(template)?;
```

#### Search Templates:
```rust
use mockforge_chaos::{TemplateSearchFilters, TemplateSortBy};

let filters = TemplateSearchFilters {
    category: Some(TemplateCategory::NetworkChaos),
    tags: vec!["latency".to_string()],
    min_rating: Some(4.0),
    author: None,
    query: Some("network".to_string()),
    sort_by: TemplateSortBy::Popular,
    limit: 20,
    offset: 0,
};

let templates = marketplace.search_templates(filters);

for template in templates {
    println!("{} (v{}) - {} stars, {} downloads",
        template.name,
        template.version,
        template.stats.stars,
        template.stats.downloads
    );
}
```

#### Download and Use Template:
```rust
let template = marketplace.download_template("network-chaos-template")?;

// Use template content to create orchestration
let orchestration = OrchestratedScenario::from_json(
    &serde_json::to_string(&template.content)?
)?;
```

#### Add a Review:
```rust
use mockforge_chaos::TemplateReview;

let review = TemplateReview {
    id: "review-1".to_string(),
    template_id: "network-chaos-template".to_string(),
    user_id: "user-1".to_string(),
    user_name: "Bob".to_string(),
    rating: 5,
    comment: "Excellent template! Very useful.".to_string(),
    created_at: Utc::now(),
    helpful_count: 0,
};

marketplace.add_review(review)?;
```

---

## ML-based Assertion Generation

### Overview
Automatically generate test assertions by analyzing historical execution data using machine learning and statistical analysis.

### Usage

#### Collect Historical Data:
```rust
use mockforge_chaos::{AssertionGenerator, AssertionGeneratorConfig, ExecutionDataPoint};
use std::collections::HashMap;

let config = AssertionGeneratorConfig {
    min_samples: 20,
    min_confidence: 0.75,
    std_dev_multiplier: 2.0,
    use_percentiles: true,
    upper_percentile: 95.0,
    lower_percentile: 5.0,
};

let mut generator = AssertionGenerator::new(config);

// Add historical execution data
for execution in historical_executions {
    let mut metrics = HashMap::new();
    metrics.insert("latency_ms".to_string(), execution.latency);
    metrics.insert("error_rate".to_string(), execution.error_rate);
    metrics.insert("throughput".to_string(), execution.throughput);

    let data_point = ExecutionDataPoint {
        timestamp: execution.timestamp,
        orchestration_id: "orch-123".to_string(),
        step_id: "step-1".to_string(),
        metrics,
        success: execution.success,
        duration_ms: execution.duration_ms,
        error_message: execution.error,
    };

    generator.add_data(data_point);
}
```

#### Generate Assertions:
```rust
let assertions = generator.generate_assertions()?;

for assertion in assertions {
    println!("Generated Assertion:");
    println!("  Type: {:?}", assertion.assertion_type);
    println!("  Path: {}", assertion.path);
    println!("  Operator: {:?}", assertion.operator);
    println!("  Value: {:.2}", assertion.value);
    println!("  Confidence: {:.2}%", assertion.confidence * 100.0);
    println!("  Rationale: {}", assertion.rationale);
    println!("  Based on {} samples\n", assertion.based_on_samples);
}
```

#### Example Output:
```
Generated Assertion:
  Type: Duration
  Path: orch-123.step-1.duration
  Operator: LessThanOrEqual
  Value: 156.50
  Confidence: 95.00%
  Rationale: Based on P95 of historical data: 156.50ms (mean: 102.34ms, std: 23.45ms)
  Based on 50 samples

Generated Assertion:
  Type: SuccessRate
  Path: orch-123.step-1.success_rate
  Operator: GreaterThanOrEqual
  Value: 0.95
  Confidence: 98.00%
  Rationale: Based on historical success rate: 98.00% (49/50 successful executions)
  Based on 50 samples
```

#### Use Generated Assertions:
```rust
// Convert to orchestration assertions
for generated in assertions {
    let assertion = Assertion {
        assertion_type: match generated.assertion_type {
            AssertionType::Duration => "step_duration",
            AssertionType::SuccessRate => "step_succeeded",
            AssertionType::MetricThreshold => "metric_in_range",
            _ => continue,
        }.to_string(),
        // Map fields appropriately
        expected_value: Some(generated.value),
        operator: format!("{:?}", generated.operator),
        confidence: Some(generated.confidence),
    };

    orchestration.add_assertion(assertion);
}
```

#### Advanced Configuration:
```rust
// Use standard deviation approach instead of percentiles
let config = AssertionGeneratorConfig {
    min_samples: 30,
    min_confidence: 0.8,
    std_dev_multiplier: 3.0,  // 3-sigma rule
    use_percentiles: false,
    ..Default::default()
};

let generator = AssertionGenerator::new(config);
```

#### Continuous Learning:
```rust
// Update generator with new data as it becomes available
for new_execution in new_executions.iter() {
    generator.add_data(create_data_point(new_execution));
}

// Regenerate assertions periodically
let updated_assertions = generator.generate_assertions()?;
```

---

## API Examples

### Version Control API

#### Create Commit:
```bash
curl -X POST http://localhost:8080/api/chaos/orchestration/orch-123/commit \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Add new resilience step",
    "author": "Alice",
    "email": "alice@example.com"
  }'
```

#### Get History:
```bash
curl http://localhost:8080/api/chaos/orchestration/orch-123/history?limit=10
```

#### Create Branch:
```bash
curl -X POST http://localhost:8080/api/chaos/orchestration/orch-123/branches \
  -H "Content-Type: application/json" \
  -d '{
    "name": "feature/new-test",
    "fromCommit": "abc123"
  }'
```

#### Get Diff:
```bash
curl "http://localhost:8080/api/chaos/orchestration/orch-123/diff?from=abc123&to=def456"
```

### Template Marketplace API

#### Search Templates:
```bash
curl -X POST http://localhost:8080/api/chaos/templates/search \
  -H "Content-Type: application/json" \
  -d '{
    "category": "network-chaos",
    "min_rating": 4.0,
    "sort_by": "popular",
    "limit": 20
  }'
```

#### Download Template:
```bash
curl -X POST http://localhost:8080/api/chaos/templates/network-chaos-template/download
```

#### Star Template:
```bash
curl -X POST http://localhost:8080/api/chaos/templates/network-chaos-template/star
```

### ML Assertion Generation API

#### Generate Assertions:
```bash
curl -X POST http://localhost:8080/api/ml/assertions/generate \
  -H "Content-Type: application/json" \
  -d '{
    "orchestration_id": "orch-123",
    "step_id": "step-1",
    "config": {
      "min_samples": 20,
      "min_confidence": 0.75,
      "use_percentiles": true,
      "upper_percentile": 95
    }
  }'
```

---

## Integration Examples

### With Existing Orchestration Builder:

```tsx
import { OrchestrationBuilder } from './pages/OrchestrationBuilder';
import { CollaborativeEditor } from './components/collaboration/CollaborativeEditor';
import { VersionControlPanel } from './components/version-control/VersionControlPanel';

function EnhancedOrchestrationBuilder() {
  const [orchestration, setOrchestration] = useState(initialOrchestration);
  const [showVersionControl, setShowVersionControl] = useState(false);

  return (
    <Box>
      <Tabs>
        <Tab label="Builder" />
        <Tab label="Version Control" />
        <Tab label="Templates" />
      </Tabs>

      <TabPanel value={0}>
        <CollaborativeEditor
          orchestrationId={orchestration.id}
          value={orchestration}
          onChange={setOrchestration}
        >
          <OrchestrationBuilder
            orchestration={orchestration}
            onChange={setOrchestration}
          />
        </CollaborativeEditor>
      </TabPanel>

      <TabPanel value={1}>
        <VersionControlPanel orchestrationId={orchestration.id} />
      </TabPanel>

      <TabPanel value={2}>
        <TemplateMarketplacePage />
      </TabPanel>
    </Box>
  );
}
```

---

## Best Practices

### Version Control
1. **Commit Often**: Create commits for logical changes
2. **Use Branches**: Isolate experimental changes
3. **Write Clear Messages**: Describe what and why
4. **Review Diffs**: Before merging, check differences carefully

### Collaborative Editing
1. **Communicate**: Use chat alongside editing
2. **Avoid Conflicts**: Coordinate who edits which sections
3. **Save Frequently**: Don't rely solely on auto-sync
4. **Review Changes**: Check the change log regularly

### Template Marketplace
1. **Document Well**: Include comprehensive README
2. **Version Properly**: Use semantic versioning
3. **Test Templates**: Verify before publishing
4. **Respond to Reviews**: Engage with the community

### ML Assertions
1. **Collect Sufficient Data**: Minimum 20-30 samples recommended
2. **Review Generated Assertions**: Always validate before using
3. **Adjust Confidence**: Lower for exploratory testing, higher for production
4. **Update Regularly**: Regenerate as new data becomes available

---

## Troubleshooting

### WebSocket Connection Issues
```typescript
// Check if WebSocket is supported
if (!window.WebSocket) {
  console.error('WebSocket not supported');
}

// Add connection error handling
const ws = new WebSocket(url);
ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};
```

### Version Control Conflicts
```rust
// Handle merge conflicts
match repo.merge("feature-branch", "main") {
    Ok(_) => println!("Merged successfully"),
    Err(e) if e.contains("conflict") => {
        // Resolve conflicts manually
        let diff = repo.diff(/*...*/)?;
        // Review and apply changes
    }
    Err(e) => return Err(e),
}
```

### ML Assertion Generation
```rust
// Handle insufficient data
match generator.generate_assertions() {
    Err(e) if e.contains("Insufficient data") => {
        println!("Need more data. Current: {}, Required: {}",
            generator.data_count(),
            config.min_samples
        );
    }
    Ok(assertions) => {
        // Use assertions
    }
    Err(e) => return Err(e),
}
```

---

## Next Steps

1. **Explore the Full API**: See `ADVANCED_FEATURES_IMPLEMENTATION.md` for complete API reference
2. **Run Examples**: Check `examples/` directory for full working examples
3. **Read Architecture Docs**: Understand the implementation details
4. **Join Community**: Share templates and best practices
5. **Contribute**: Help implement remaining features

## Support

- **Documentation**: `/docs` directory
- **Examples**: `/examples` directory
- **Issues**: GitHub Issues
- **Discussions**: GitHub Discussions
- **API Reference**: OpenAPI spec at `/api/docs`
