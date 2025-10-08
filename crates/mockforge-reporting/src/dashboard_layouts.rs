//! Custom dashboard layouts for observability and chaos engineering
//!
//! Allows users to create, save, and share custom dashboard configurations
//! with different widget arrangements and data sources.

use crate::{Result, ReportingError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Dashboard layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayout {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub author: String,
    pub tags: Vec<String>,
    pub is_public: bool,
    pub grid_config: GridConfig,
    pub widgets: Vec<Widget>,
    pub filters: Vec<DashboardFilter>,
}

/// Grid configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    pub columns: u32,
    pub row_height: u32,
    pub gap: u32,
    pub responsive_breakpoints: HashMap<String, u32>,
}

impl Default for GridConfig {
    fn default() -> Self {
        let mut breakpoints = HashMap::new();
        breakpoints.insert("mobile".to_string(), 1);
        breakpoints.insert("tablet".to_string(), 2);
        breakpoints.insert("desktop".to_string(), 4);
        breakpoints.insert("wide".to_string(), 6);

        Self {
            columns: 12,
            row_height: 60,
            gap: 16,
            responsive_breakpoints: breakpoints,
        }
    }
}

/// Widget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    pub widget_type: WidgetType,
    pub title: String,
    pub position: WidgetPosition,
    pub data_source: DataSource,
    pub refresh_interval_seconds: Option<u32>,
    pub config: serde_json::Value,
}

/// Widget position in grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetPosition {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Widget type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    LineChart,
    BarChart,
    PieChart,
    Gauge,
    Counter,
    Table,
    Heatmap,
    Flamegraph,
    Timeline,
    AlertList,
    MetricComparison,
    ScenarioStatus,
    ServiceMap,
    LogStream,
    Custom,
}

/// Data source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub source_type: DataSourceType,
    pub query: String,
    pub aggregation: Option<AggregationType>,
    pub time_range: TimeRange,
}

/// Data source type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceType {
    Prometheus,
    OpenTelemetry,
    ChaosMetrics,
    ScenarioExecutions,
    AlertHistory,
    CustomMetric,
}

/// Aggregation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AggregationType {
    Sum,
    Average,
    Min,
    Max,
    Count,
    P50,
    P95,
    P99,
}

/// Time range for data queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub range_type: TimeRangeType,
    pub value: Option<u64>,
}

/// Time range type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TimeRangeType {
    Last15Minutes,
    Last1Hour,
    Last6Hours,
    Last24Hours,
    Last7Days,
    Last30Days,
    Custom,
}

/// Dashboard filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardFilter {
    pub id: String,
    pub name: String,
    pub filter_type: FilterType,
    pub options: Vec<String>,
    pub default_value: Option<String>,
}

/// Filter type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FilterType {
    ServiceName,
    Environment,
    ScenarioType,
    TimeRange,
    Custom,
}

/// Dashboard layout manager
pub struct DashboardLayoutManager {
    layouts_dir: String,
}

impl DashboardLayoutManager {
    /// Create a new layout manager
    pub fn new(layouts_dir: String) -> Result<Self> {
        // Ensure directory exists
        fs::create_dir_all(&layouts_dir)?;

        Ok(Self { layouts_dir })
    }

    /// Save a dashboard layout
    pub fn save_layout(&self, layout: &DashboardLayout) -> Result<()> {
        let file_path = self.get_layout_path(&layout.id);
        let json = serde_json::to_string_pretty(layout)?;
        fs::write(file_path, json)?;
        Ok(())
    }

    /// Load a dashboard layout
    pub fn load_layout(&self, layout_id: &str) -> Result<DashboardLayout> {
        let file_path = self.get_layout_path(layout_id);

        if !Path::new(&file_path).exists() {
            return Err(ReportingError::Analysis(
                format!("Layout not found: {}", layout_id)
            ));
        }

        let json = fs::read_to_string(file_path)?;
        let layout: DashboardLayout = serde_json::from_str(&json)?;
        Ok(layout)
    }

    /// List all dashboard layouts
    pub fn list_layouts(&self) -> Result<Vec<DashboardLayoutInfo>> {
        let mut layouts = Vec::new();

        for entry in fs::read_dir(&self.layouts_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(json) = fs::read_to_string(&path) {
                    if let Ok(layout) = serde_json::from_str::<DashboardLayout>(&json) {
                        layouts.push(DashboardLayoutInfo {
                            id: layout.id,
                            name: layout.name,
                            description: layout.description,
                            author: layout.author,
                            tags: layout.tags,
                            created_at: layout.created_at,
                            updated_at: layout.updated_at,
                            widget_count: layout.widgets.len(),
                        });
                    }
                }
            }
        }

        Ok(layouts)
    }

    /// Delete a dashboard layout
    pub fn delete_layout(&self, layout_id: &str) -> Result<()> {
        let file_path = self.get_layout_path(layout_id);

        if Path::new(&file_path).exists() {
            fs::remove_file(file_path)?;
            Ok(())
        } else {
            Err(ReportingError::Analysis(
                format!("Layout not found: {}", layout_id)
            ))
        }
    }

    /// Clone a dashboard layout with a new ID
    pub fn clone_layout(&self, source_id: &str, new_name: &str, new_author: &str) -> Result<DashboardLayout> {
        let mut layout = self.load_layout(source_id)?;

        // Update with new information
        layout.id = uuid::Uuid::new_v4().to_string();
        layout.name = new_name.to_string();
        layout.author = new_author.to_string();
        layout.created_at = chrono::Utc::now();
        layout.updated_at = chrono::Utc::now();

        self.save_layout(&layout)?;
        Ok(layout)
    }

    /// Export layout to JSON string
    pub fn export_layout(&self, layout_id: &str) -> Result<String> {
        let layout = self.load_layout(layout_id)?;
        let json = serde_json::to_string_pretty(&layout)?;
        Ok(json)
    }

    /// Import layout from JSON string
    pub fn import_layout(&self, json: &str) -> Result<DashboardLayout> {
        let layout: DashboardLayout = serde_json::from_str(json)?;
        self.save_layout(&layout)?;
        Ok(layout)
    }

    fn get_layout_path(&self, layout_id: &str) -> String {
        format!("{}/{}.json", self.layouts_dir, layout_id)
    }
}

/// Dashboard layout info (summary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayoutInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: String,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub widget_count: usize,
}

/// Dashboard layout builder
pub struct DashboardLayoutBuilder {
    layout: DashboardLayout,
}

impl DashboardLayoutBuilder {
    /// Create a new layout builder
    pub fn new(name: &str, author: &str) -> Self {
        let now = chrono::Utc::now();

        Self {
            layout: DashboardLayout {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                description: None,
                created_at: now,
                updated_at: now,
                author: author.to_string(),
                tags: Vec::new(),
                is_public: false,
                grid_config: GridConfig::default(),
                widgets: Vec::new(),
                filters: Vec::new(),
            },
        }
    }

    /// Set description
    pub fn description(mut self, description: &str) -> Self {
        self.layout.description = Some(description.to_string());
        self
    }

    /// Add tag
    pub fn tag(mut self, tag: &str) -> Self {
        self.layout.tags.push(tag.to_string());
        self
    }

    /// Set public visibility
    pub fn public(mut self, is_public: bool) -> Self {
        self.layout.is_public = is_public;
        self
    }

    /// Set grid configuration
    pub fn grid_config(mut self, config: GridConfig) -> Self {
        self.layout.grid_config = config;
        self
    }

    /// Add widget
    pub fn add_widget(mut self, widget: Widget) -> Self {
        self.layout.widgets.push(widget);
        self
    }

    /// Add filter
    pub fn add_filter(mut self, filter: DashboardFilter) -> Self {
        self.layout.filters.push(filter);
        self
    }

    /// Build the layout
    pub fn build(self) -> DashboardLayout {
        self.layout
    }
}

/// Pre-built dashboard templates
pub struct DashboardTemplates;

impl DashboardTemplates {
    /// Create a chaos engineering overview dashboard
    pub fn chaos_overview() -> DashboardLayout {
        DashboardLayoutBuilder::new("Chaos Engineering Overview", "MockForge")
            .description("Real-time overview of chaos engineering activities")
            .tag("chaos")
            .tag("overview")
            .public(true)
            .add_widget(Widget {
                id: "active-scenarios".to_string(),
                widget_type: WidgetType::Counter,
                title: "Active Scenarios".to_string(),
                position: WidgetPosition { x: 0, y: 0, width: 3, height: 2 },
                data_source: DataSource {
                    source_type: DataSourceType::ScenarioExecutions,
                    query: "count(active_scenarios)".to_string(),
                    aggregation: Some(AggregationType::Count),
                    time_range: TimeRange {
                        range_type: TimeRangeType::Last15Minutes,
                        value: None,
                    },
                },
                refresh_interval_seconds: Some(5),
                config: serde_json::json!({"color": "blue"}),
            })
            .add_widget(Widget {
                id: "error-rate".to_string(),
                widget_type: WidgetType::LineChart,
                title: "Error Rate".to_string(),
                position: WidgetPosition { x: 3, y: 0, width: 6, height: 4 },
                data_source: DataSource {
                    source_type: DataSourceType::ChaosMetrics,
                    query: "error_rate".to_string(),
                    aggregation: Some(AggregationType::Average),
                    time_range: TimeRange {
                        range_type: TimeRangeType::Last1Hour,
                        value: None,
                    },
                },
                refresh_interval_seconds: Some(10),
                config: serde_json::json!({"yAxisLabel": "Error %"}),
            })
            .add_widget(Widget {
                id: "latency-heatmap".to_string(),
                widget_type: WidgetType::Heatmap,
                title: "Latency Distribution".to_string(),
                position: WidgetPosition { x: 0, y: 4, width: 12, height: 4 },
                data_source: DataSource {
                    source_type: DataSourceType::OpenTelemetry,
                    query: "histogram_quantile(0.95, latency)".to_string(),
                    aggregation: Some(AggregationType::P95),
                    time_range: TimeRange {
                        range_type: TimeRangeType::Last6Hours,
                        value: None,
                    },
                },
                refresh_interval_seconds: Some(30),
                config: serde_json::json!({"colorScheme": "RdYlGn"}),
            })
            .build()
    }

    /// Create a service performance dashboard
    pub fn service_performance() -> DashboardLayout {
        DashboardLayoutBuilder::new("Service Performance", "MockForge")
            .description("Detailed service performance metrics")
            .tag("performance")
            .tag("services")
            .public(true)
            .add_widget(Widget {
                id: "request-rate".to_string(),
                widget_type: WidgetType::LineChart,
                title: "Request Rate".to_string(),
                position: WidgetPosition { x: 0, y: 0, width: 6, height: 4 },
                data_source: DataSource {
                    source_type: DataSourceType::Prometheus,
                    query: "rate(http_requests_total[5m])".to_string(),
                    aggregation: None,
                    time_range: TimeRange {
                        range_type: TimeRangeType::Last1Hour,
                        value: None,
                    },
                },
                refresh_interval_seconds: Some(10),
                config: serde_json::json!({}),
            })
            .add_widget(Widget {
                id: "p95-latency".to_string(),
                widget_type: WidgetType::Gauge,
                title: "P95 Latency".to_string(),
                position: WidgetPosition { x: 6, y: 0, width: 3, height: 4 },
                data_source: DataSource {
                    source_type: DataSourceType::Prometheus,
                    query: "histogram_quantile(0.95, latency_seconds)".to_string(),
                    aggregation: Some(AggregationType::P95),
                    time_range: TimeRange {
                        range_type: TimeRangeType::Last15Minutes,
                        value: None,
                    },
                },
                refresh_interval_seconds: Some(5),
                config: serde_json::json!({"max": 1000, "thresholds": [{"value": 500, "color": "yellow"}, {"value": 800, "color": "red"}]}),
            })
            .build()
    }

    /// Create a resilience testing dashboard
    pub fn resilience_testing() -> DashboardLayout {
        DashboardLayoutBuilder::new("Resilience Testing", "MockForge")
            .description("Monitor resilience patterns and circuit breaker status")
            .tag("resilience")
            .tag("testing")
            .public(true)
            .add_widget(Widget {
                id: "circuit-breaker-status".to_string(),
                widget_type: WidgetType::Table,
                title: "Circuit Breaker Status".to_string(),
                position: WidgetPosition { x: 0, y: 0, width: 6, height: 4 },
                data_source: DataSource {
                    source_type: DataSourceType::ChaosMetrics,
                    query: "circuit_breaker_status".to_string(),
                    aggregation: None,
                    time_range: TimeRange {
                        range_type: TimeRangeType::Last15Minutes,
                        value: None,
                    },
                },
                refresh_interval_seconds: Some(5),
                config: serde_json::json!({"columns": ["service", "status", "failures", "last_failure"]}),
            })
            .add_widget(Widget {
                id: "retry-success-rate".to_string(),
                widget_type: WidgetType::BarChart,
                title: "Retry Success Rate".to_string(),
                position: WidgetPosition { x: 6, y: 0, width: 6, height: 4 },
                data_source: DataSource {
                    source_type: DataSourceType::ChaosMetrics,
                    query: "retry_success_rate".to_string(),
                    aggregation: Some(AggregationType::Average),
                    time_range: TimeRange {
                        range_type: TimeRangeType::Last1Hour,
                        value: None,
                    },
                },
                refresh_interval_seconds: Some(15),
                config: serde_json::json!({}),
            })
            .build()
    }
}

// Add uuid dependency
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_dashboard_layout_builder() {
        let layout = DashboardLayoutBuilder::new("Test Dashboard", "test-user")
            .description("Test description")
            .tag("test")
            .public(true)
            .build();

        assert_eq!(layout.name, "Test Dashboard");
        assert_eq!(layout.author, "test-user");
        assert!(layout.is_public);
        assert_eq!(layout.tags, vec!["test"]);
    }

    #[test]
    fn test_layout_manager_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let manager = DashboardLayoutManager::new(temp_dir.path().to_str().unwrap().to_string()).unwrap();

        let layout = DashboardLayoutBuilder::new("Test", "author")
            .build();

        // Save
        manager.save_layout(&layout).unwrap();

        // Load
        let loaded = manager.load_layout(&layout.id).unwrap();
        assert_eq!(loaded.id, layout.id);
        assert_eq!(loaded.name, layout.name);
    }

    #[test]
    fn test_layout_templates() {
        let chaos_layout = DashboardTemplates::chaos_overview();
        assert_eq!(chaos_layout.name, "Chaos Engineering Overview");
        assert!(!chaos_layout.widgets.is_empty());

        let perf_layout = DashboardTemplates::service_performance();
        assert_eq!(perf_layout.name, "Service Performance");
    }
}
