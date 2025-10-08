# Advanced Observability Features

MockForge provides advanced observability capabilities including flamegraph trace analysis, custom dashboard layouts, scenario comparison tools, and comprehensive report generation in multiple formats.

## Table of Contents

1. [Flamegraph Trace Analysis](#flamegraph-trace-analysis)
2. [Custom Dashboard Layouts](#custom-dashboard-layouts)
3. [Scenario Comparison Tools](#scenario-comparison-tools)
4. [Report Generation (PDF/CSV)](#report-generation)
5. [API Reference](#api-reference)

---

## Flamegraph Trace Analysis

Flamegraphs provide a powerful visualization of distributed traces, helping you identify performance bottlenecks and understand call hierarchies.

### Features

- **Hierarchical Visualization**: See the complete call stack hierarchy
- **Performance Hotspot Detection**: Automatically identify the slowest execution paths
- **Interactive SVG Output**: Explore traces with interactive flamegraphs
- **Statistics Generation**: Get detailed metrics about trace depth and duration

### Using the Flamegraph Generator

```rust
use mockforge_reporting::{FlamegraphGenerator, TraceData, TraceSpan};
use std::collections::HashMap;

// Create trace data
let trace = TraceData {
    trace_id: "trace-123".to_string(),
    spans: vec![
        TraceSpan {
            span_id: "span-1".to_string(),
            parent_span_id: None,
            operation_name: "api_request".to_string(),
            service_name: "api-gateway".to_string(),
            start_time: 0,
            duration_us: 10000,
            tags: HashMap::new(),
        },
        TraceSpan {
            span_id: "span-2".to_string(),
            parent_span_id: Some("span-1".to_string()),
            operation_name: "database_query".to_string(),
            service_name: "postgres".to_string(),
            start_time: 1000,
            duration_us: 5000,
            tags: HashMap::new(),
        },
    ],
};

// Generate flamegraph
let generator = FlamegraphGenerator::new();
generator.generate(&trace, "output.svg")?;

// Get statistics
let stats = generator.generate_stats(&trace)?;
println!("Total spans: {}", stats.total_spans);
println!("Max depth: {}", stats.max_depth);
println!("Hottest path: {:?}", stats.hottest_path);
```

### API Endpoint

```bash
# Get flamegraph for a trace
GET /api/observability/traces/{trace_id}/flamegraph

# Response
{
  "success": true,
  "data": {
    "trace_id": "trace-123",
    "svg_url": "/flamegraphs/trace-123.svg",
    "stats": {
      "total_spans": 15,
      "max_depth": 4,
      "total_duration_us": 125000,
      "hottest_path": [
        "api-gateway::request",
        "user-service::getUser",
        "database::query"
      ]
    }
  }
}
```

---

## Custom Dashboard Layouts

Create, save, and share custom dashboard configurations with different widget arrangements and data sources.

### Features

- **Flexible Grid System**: Responsive 12-column grid layout
- **Multiple Widget Types**: Charts, gauges, tables, heatmaps, and more
- **Data Source Integration**: Connect to Prometheus, OpenTelemetry, and custom metrics
- **Template Marketplace**: Pre-built templates for common use cases
- **Import/Export**: Share layouts as JSON

### Widget Types

- `LineChart` - Time series line charts
- `BarChart` - Bar charts for comparisons
- `PieChart` - Distribution visualization
- `Gauge` - Single metric gauges with thresholds
- `Counter` - Large number displays
- `Table` - Tabular data displays
- `Heatmap` - Density visualizations
- `Flamegraph` - Trace flamegraphs
- `Timeline` - Event timelines
- `AlertList` - Active alerts display
- `MetricComparison` - Side-by-side comparisons
- `ScenarioStatus` - Chaos scenario status
- `ServiceMap` - Service dependency maps
- `LogStream` - Real-time log streaming

### Creating a Custom Dashboard

```rust
use mockforge_reporting::{
    DashboardLayoutBuilder, Widget, WidgetType, WidgetPosition,
    DataSource, DataSourceType, TimeRange, TimeRangeType,
    AggregationType,
};

let layout = DashboardLayoutBuilder::new("My Dashboard", "john@example.com")
    .description("Custom performance dashboard")
    .tag("performance")
    .tag("custom")
    .public(false)
    .add_widget(Widget {
        id: "error-rate".to_string(),
        widget_type: WidgetType::LineChart,
        title: "Error Rate".to_string(),
        position: WidgetPosition {
            x: 0,
            y: 0,
            width: 6,
            height: 4,
        },
        data_source: DataSource {
            source_type: DataSourceType::Prometheus,
            query: "rate(http_errors_total[5m])".to_string(),
            aggregation: Some(AggregationType::Sum),
            time_range: TimeRange {
                range_type: TimeRangeType::Last1Hour,
                value: None,
            },
        },
        refresh_interval_seconds: Some(10),
        config: serde_json::json!({
            "yAxisLabel": "Errors/sec",
            "color": "red"
        }),
    })
    .build();

// Save the layout
let manager = DashboardLayoutManager::new("./dashboards")?;
manager.save_layout(&layout)?;
```

### Using Pre-built Templates

```rust
use mockforge_reporting::DashboardTemplates;

// Chaos Engineering Overview
let chaos_dashboard = DashboardTemplates::chaos_overview();

// Service Performance Dashboard
let perf_dashboard = DashboardTemplates::service_performance();

// Resilience Testing Dashboard
let resilience_dashboard = DashboardTemplates::resilience_testing();
```

### API Endpoints

```bash
# List all dashboard layouts
GET /api/dashboard/layouts

# Get a specific layout
GET /api/dashboard/layouts/{id}

# Create a new layout
POST /api/dashboard/layouts
Content-Type: application/json

{
  "name": "My Dashboard",
  "description": "Custom dashboard",
  "layout_data": { /* layout configuration */ }
}

# Update a layout
POST /api/dashboard/layouts/{id}

# Delete a layout
DELETE /api/dashboard/layouts/{id}

# Get pre-built templates
GET /api/dashboard/templates
```

---

## Scenario Comparison Tools

Compare chaos scenario executions to identify performance regressions and improvements.

### Features

- **Multi-scenario Comparison**: Compare multiple runs against a baseline
- **Automatic Regression Detection**: Identify performance degradations
- **Statistical Significance**: Determine if changes are statistically significant
- **Improvement Tracking**: Highlight performance improvements
- **Confidence Scores**: Get confidence levels for comparisons

### Comparison Example

```rust
use mockforge_reporting::{ComparisonReportGenerator, ExecutionReport};

let mut generator = ComparisonReportGenerator::new();

// Set baseline
generator.set_baseline(baseline_report);

// Compare against other runs
let comparison = generator.compare(vec![run1, run2, run3])?;

// Analyze results
println!("Verdict: {:?}", comparison.overall_assessment.verdict);
println!("Regressions: {}", comparison.regressions.len());
println!("Improvements: {}", comparison.improvements.len());

for regression in &comparison.regressions {
    println!(
        "❌ {}: {:.2}% degradation ({})",
        regression.metric_name,
        regression.impact_percentage,
        regression.severity
    );
}

for improvement in &comparison.improvements {
    println!(
        "✅ {}: {:.2}% improvement",
        improvement.metric_name,
        improvement.improvement_percentage
    );
}
```

### Comparison Metrics

- Error rate
- Average latency
- P95/P99 latency
- Request throughput
- Failed requests
- Duration
- Step failures

### Significance Levels

- **Not Significant**: < 5% change
- **Low**: 5-15% change
- **Medium**: 15-30% change
- **High**: > 30% change

### API Endpoint

```bash
POST /api/reports/compare
Content-Type: application/json

{
  "baseline_scenario": "baseline-run-123",
  "comparison_scenarios": ["run-124", "run-125"]
}

# Response
{
  "success": true,
  "data": {
    "baseline": "baseline-run-123",
    "comparisons": ["run-124", "run-125"],
    "regressions_count": 2,
    "improvements_count": 5,
    "verdict": "better"
  }
}
```

---

## Report Generation

Export execution reports in multiple formats for documentation and analysis.

### PDF Reports

```rust
use mockforge_reporting::{PdfReportGenerator, PdfConfig};

let config = PdfConfig {
    title: "Chaos Engineering Report".to_string(),
    author: "MockForge".to_string(),
    include_charts: true,
    include_metrics: true,
    include_recommendations: true,
};

let generator = PdfReportGenerator::new(config);
generator.generate(&execution_report, "report.pdf")?;
```

### CSV Exports

```rust
use mockforge_reporting::{CsvExporter, CsvExportConfig};

let config = CsvExportConfig {
    delimiter: ',',
    include_headers: true,
    quote_strings: true,
};

let exporter = CsvExporter::new(config);

// Single report
exporter.export_execution_report(&report, "report.csv")?;

// Multiple reports
exporter.export_execution_reports(&reports, "all_reports.csv")?;

// Comparison report
exporter.export_comparison_report(&comparison, "comparison.csv")?;

// Time series metrics
exporter.export_metrics_time_series(&metrics, "timeseries.csv")?;
```

### Batch Export

```rust
use mockforge_reporting::CsvBatchExporter;

let exporter = CsvBatchExporter::new(config);
exporter.export_all(
    &execution_reports,
    Some(&comparison_report),
    "./exports"
)?;

// Generates:
// - ./exports/execution_reports.csv
// - ./exports/comparison.csv
// - ./exports/regressions.csv
// - ./exports/improvements.csv
```

### API Endpoints

```bash
# Generate PDF report
POST /api/reports/pdf
Content-Type: application/json

{
  "scenario_name": "test-scenario",
  "include_charts": true
}

# Generate CSV report
POST /api/reports/csv
Content-Type: application/json

{
  "scenario_names": ["scenario-1", "scenario-2"],
  "include_comparison": true
}
```

---

## API Reference

### Flamegraph Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/observability/traces/{trace_id}/flamegraph` | Get flamegraph for trace |

### Dashboard Layout Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/dashboard/layouts` | List all layouts |
| POST | `/api/dashboard/layouts` | Create new layout |
| GET | `/api/dashboard/layouts/{id}` | Get specific layout |
| POST | `/api/dashboard/layouts/{id}` | Update layout |
| DELETE | `/api/dashboard/layouts/{id}` | Delete layout |
| GET | `/api/dashboard/templates` | Get pre-built templates |

### Report Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/reports/pdf` | Generate PDF report |
| POST | `/api/reports/csv` | Generate CSV report |
| POST | `/api/reports/compare` | Compare scenarios |

---

## Best Practices

### Flamegraphs

1. **Set Appropriate Thresholds**: Use `with_threshold()` to collapse very short spans
2. **Focus on Critical Paths**: Examine the hottest_path to find bottlenecks
3. **Regular Analysis**: Generate flamegraphs regularly to catch regressions early

### Custom Dashboards

1. **Organize by Purpose**: Create separate dashboards for different audiences
2. **Use Filters**: Add dashboard filters for dynamic data exploration
3. **Set Refresh Intervals**: Balance freshness with performance
4. **Start with Templates**: Use pre-built templates and customize

### Scenario Comparison

1. **Establish Baselines**: Always compare against a stable baseline
2. **Multiple Runs**: Compare multiple runs to account for variance
3. **Track Trends**: Use time-series comparisons to identify trends
4. **Document Changes**: Export comparisons for historical records

### Report Generation

1. **Automate Exports**: Schedule regular CSV exports for analysis
2. **Include Context**: Add descriptions and recommendations to PDFs
3. **Archive Reports**: Keep historical reports for trend analysis
4. **Share Insights**: Use PDF reports for stakeholder communication

---

## Integration Examples

### With CI/CD

```bash
#!/bin/bash
# Run chaos scenario and generate reports

# Execute scenario
mockforge chaos run my-scenario

# Generate flamegraph
curl -X GET http://localhost:8080/api/observability/traces/latest/flamegraph \
  -o flamegraph.svg

# Generate PDF report
curl -X POST http://localhost:8080/api/reports/pdf \
  -H "Content-Type: application/json" \
  -d '{"scenario_name":"my-scenario","include_charts":true}' \
  -o report.pdf

# Compare with baseline
curl -X POST http://localhost:8080/api/reports/compare \
  -H "Content-Type: application/json" \
  -d '{"baseline_scenario":"baseline","comparison_scenarios":["my-scenario"]}' \
  | jq '.data.verdict'
```

### With Monitoring Systems

```yaml
# Prometheus AlertManager integration
routes:
  - receiver: 'mockforge-reports'
    matchers:
      - alertname =~ "ChaosScenario.*"

receivers:
  - name: 'mockforge-reports'
    webhook_configs:
      - url: 'http://mockforge:8080/api/reports/pdf'
        send_resolved: true
```

### Scheduled Reporting

```rust
use tokio::time::{interval, Duration};

async fn scheduled_reporting() {
    let mut interval = interval(Duration::from_secs(3600)); // Every hour

    loop {
        interval.tick().await;

        // Generate hourly CSV export
        let exporter = CsvBatchExporter::default();
        let reports = fetch_last_hour_reports().await;
        exporter.export_all(&reports, None, "./hourly-reports").unwrap();
    }
}
```

---

## Troubleshooting

### Flamegraph Generation Issues

**Problem**: Flamegraph shows no data
**Solution**: Ensure traces have parent-child relationships properly set

**Problem**: SVG is too large
**Solution**: Increase collapse threshold or filter by service

### Dashboard Layout Issues

**Problem**: Widgets overlap
**Solution**: Check grid positions don't conflict (x + width)

**Problem**: Data not refreshing
**Solution**: Verify data source query and check refresh_interval_seconds

### Comparison Issues

**Problem**: All changes marked as "not significant"
**Solution**: Changes might be too small; check baseline stability

**Problem**: Unexpected regressions
**Solution**: Verify baseline is from same environment/configuration

### Export Issues

**Problem**: PDF generation fails
**Solution**: Check printpdf dependency and font availability

**Problem**: CSV encoding issues
**Solution**: Ensure quote_strings is enabled for special characters

---

## Additional Resources

- [Chaos Engineering Guide](./CHAOS_ENGINEERING.md)
- [OpenTelemetry Integration](./OPENTELEMETRY.md)
- [Resilience Patterns](./RESILIENCE_PATTERNS.md)
- [Admin UI Guide](./ADMIN_UI_QUICKSTART.md)

---

**Next Steps**: Explore the [Test Generation](./TEST_GENERATION.md) features to automatically create test scenarios from your reports.
