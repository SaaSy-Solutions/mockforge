//! Data models for the admin UI

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Server status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    /// Server type (HTTP, WebSocket, gRPC)
    pub server_type: String,
    /// Server address
    pub address: Option<String>,
    /// Whether server is running
    pub running: bool,
    /// Start time
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Uptime in seconds
    pub uptime_seconds: Option<u64>,
    /// Number of active connections
    pub active_connections: u64,
    /// Total requests served
    pub total_requests: u64,
}

/// Route information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    /// HTTP method
    pub method: Option<String>,
    /// Route path
    pub path: String,
    /// Route priority
    pub priority: i32,
    /// Whether route has fixtures
    pub has_fixtures: bool,
    /// Latency profile
    pub latency_ms: Option<u64>,
    /// Request count
    pub request_count: u64,
    /// Last request time
    pub last_request: Option<chrono::DateTime<chrono::Utc>>,
    /// Error count
    pub error_count: u64,
}

/// Request log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    /// Request ID
    pub id: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Response status code
    pub status_code: u16,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request headers (filtered)
    pub headers: HashMap<String, String>,
    /// Response size in bytes
    pub response_size_bytes: u64,
    /// Error message (if any)
    pub error_message: Option<String>,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// MockForge version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Memory usage in MB
    pub memory_usage_mb: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Number of active threads
    pub active_threads: usize,
    /// Total routes configured
    pub total_routes: usize,
    /// Total fixtures available
    pub total_fixtures: usize,
}

/// Latency profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyProfile {
    /// Profile name
    pub name: String,
    /// Base latency in milliseconds
    pub base_ms: u64,
    /// Jitter range in milliseconds
    pub jitter_ms: u64,
    /// Tag-based overrides
    pub tag_overrides: HashMap<String, u64>,
}

/// Fault injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultConfig {
    /// Whether fault injection is enabled
    pub enabled: bool,
    /// Failure rate (0.0 to 1.0)
    pub failure_rate: f64,
    /// HTTP status codes for failures
    pub status_codes: Vec<u16>,
    /// Current active failures
    pub active_failures: u64,
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Whether proxy is enabled
    pub enabled: bool,
    /// Upstream URL
    pub upstream_url: Option<String>,
    /// Request timeout seconds
    pub timeout_seconds: u64,
    /// Total requests proxied
    pub requests_proxied: u64,
}

/// Bandwidth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthConfig {
    /// Whether bandwidth throttling is enabled
    pub enabled: bool,
    /// Maximum bandwidth in bytes per second
    pub max_bytes_per_sec: u64,
    /// Burst capacity in bytes
    pub burst_capacity_bytes: u64,
    /// Tag-based overrides
    pub tag_overrides: HashMap<String, u64>,
}

/// Burst loss configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurstLossConfig {
    /// Whether burst loss is enabled
    pub enabled: bool,
    /// Probability of entering burst (0.0 to 1.0)
    pub burst_probability: f64,
    /// Duration of burst in milliseconds
    pub burst_duration_ms: u64,
    /// Loss rate during burst (0.0 to 1.0)
    pub loss_rate_during_burst: f64,
    /// Recovery time between bursts in milliseconds
    pub recovery_time_ms: u64,
    /// Tag-based overrides
    pub tag_overrides: HashMap<String, BurstLossOverride>,
}

/// Burst loss override for specific tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurstLossOverride {
    pub burst_probability: f64,
    pub burst_duration_ms: u64,
    pub loss_rate_during_burst: f64,
    pub recovery_time_ms: u64,
}

/// Traffic shaping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficShapingConfig {
    /// Whether traffic shaping is enabled
    pub enabled: bool,
    /// Bandwidth configuration
    pub bandwidth: BandwidthConfig,
    /// Burst loss configuration
    pub burst_loss: BurstLossConfig,
}

/// Simple metrics data for admin dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMetricsData {
    /// Total requests served
    pub total_requests: u64,
    /// Active requests currently being processed
    pub active_requests: u64,
    /// Average response time in milliseconds
    pub average_response_time: f64,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
}

/// Dashboard system information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSystemInfo {
    /// Operating system
    pub os: String,
    /// Architecture
    pub arch: String,
    /// Uptime in seconds
    pub uptime: u64,
    /// Memory usage in bytes
    pub memory_usage: u64,
}

/// Dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// Server information
    pub server_info: ServerInfo,
    /// System information
    pub system_info: DashboardSystemInfo,
    /// Metrics data
    pub metrics: SimpleMetricsData,
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Whether request was successful
    pub success: bool,
    /// Response data
    pub data: Option<T>,
    /// Error message (if any)
    pub error: Option<String>,
    /// Response timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create an error response
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Configuration update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdate {
    /// Configuration type
    pub config_type: String,
    /// Configuration data
    pub data: serde_json::Value,
}

/// Route management request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteUpdate {
    /// Route path
    pub path: String,
    /// HTTP method (optional)
    pub method: Option<String>,
    /// Update operation
    pub operation: String,
    /// Update data
    pub data: Option<serde_json::Value>,
}

/// Log filter options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    /// Filter by HTTP method
    pub method: Option<String>,
    /// Filter by path pattern
    pub path_pattern: Option<String>,
    /// Filter by status code
    pub status_code: Option<u16>,
    /// Filter by time range (hours ago)
    pub hours_ago: Option<u64>,
    /// Maximum number of results
    pub limit: Option<usize>,
}

impl Default for LogFilter {
    fn default() -> Self {
        Self {
            method: None,
            path_pattern: None,
            status_code: None,
            hours_ago: Some(24),
            limit: Some(100),
        }
    }
}

/// Metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsData {
    /// Request count by endpoint
    pub requests_by_endpoint: HashMap<String, u64>,
    /// Response time percentiles
    pub response_time_percentiles: HashMap<String, u64>,
    /// Error rate by endpoint
    pub error_rate_by_endpoint: HashMap<String, f64>,
    /// Memory usage over time
    pub memory_usage_over_time: Vec<(chrono::DateTime<chrono::Utc>, u64)>,
    /// CPU usage over time
    pub cpu_usage_over_time: Vec<(chrono::DateTime<chrono::Utc>, f64)>,
}

/// Validation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSettings {
    /// Validation mode: "enforce", "warn", or "off"
    pub mode: String,
    /// Whether to aggregate errors
    pub aggregate_errors: bool,
    /// Whether to validate responses
    pub validate_responses: bool,
    /// Per-route validation overrides
    pub overrides: HashMap<String, String>,
}

/// Validation update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationUpdate {
    /// Validation mode
    pub mode: String,
    /// Whether to aggregate errors
    pub aggregate_errors: bool,
    /// Whether to validate responses
    pub validate_responses: bool,
    /// Per-route validation overrides
    pub overrides: Option<HashMap<String, String>>,
}

/// Log entry for admin UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// HTTP status code
    pub status: u16,
    /// HTTP method
    pub method: String,
    /// Request URL/path
    pub url: String,
    /// Response time in milliseconds
    pub response_time: u64,
    /// Response size in bytes
    pub size: u64,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Overall health status
    pub status: String,
    /// Individual service health
    pub services: HashMap<String, String>,
    /// Last health check time
    pub last_check: chrono::DateTime<chrono::Utc>,
    /// Any health issues
    pub issues: Vec<String>,
}

impl HealthCheck {
    /// Create a healthy status
    pub fn healthy() -> Self {
        Self {
            status: "healthy".to_string(),
            services: HashMap::new(),
            last_check: chrono::Utc::now(),
            issues: Vec::new(),
        }
    }

    /// Create an unhealthy status
    pub fn unhealthy(issues: Vec<String>) -> Self {
        Self {
            status: "unhealthy".to_string(),
            services: HashMap::new(),
            last_check: chrono::Utc::now(),
            issues,
        }
    }

    /// Add service status
    pub fn with_service(mut self, name: String, status: String) -> Self {
        self.services.insert(name, status);
        self
    }
}

/// Workspace summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    /// Workspace ID
    pub id: String,
    /// Workspace name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Whether this is the active workspace
    pub active: bool,
    /// Number of folders
    pub folder_count: usize,
    /// Number of requests
    pub request_count: usize,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Folder summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderSummary {
    /// Folder ID
    pub id: String,
    /// Folder name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Parent folder ID (None if root)
    pub parent_id: Option<String>,
    /// Number of subfolders
    pub subfolder_count: usize,
    /// Number of requests
    pub request_count: usize,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Request summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSummary {
    /// Request ID
    pub id: String,
    /// Request name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Response status code
    pub status_code: u16,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Workspace detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDetail {
    /// Workspace summary
    pub summary: WorkspaceSummary,
    /// Root folders
    pub folders: Vec<FolderSummary>,
    /// Root requests
    pub requests: Vec<RequestSummary>,
}

/// Folder detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderDetail {
    /// Folder summary
    pub summary: FolderSummary,
    /// Subfolders
    pub subfolders: Vec<FolderSummary>,
    /// Requests in this folder
    pub requests: Vec<RequestSummary>,
}

/// Create workspace request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    /// Workspace name
    pub name: String,
    /// Description (optional)
    pub description: Option<String>,
}

/// Create folder request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFolderRequest {
    /// Folder name
    pub name: String,
    /// Description (optional)
    pub description: Option<String>,
    /// Parent folder ID (optional)
    pub parent_id: Option<String>,
}

/// Create request request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRequestRequest {
    /// Request name
    pub name: String,
    /// Description (optional)
    pub description: Option<String>,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Response status code
    pub status_code: Option<u16>,
    /// Response body
    pub response_body: Option<String>,
    /// Folder ID (optional)
    pub folder_id: Option<String>,
}

/// Import to workspace request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportToWorkspaceRequest {
    /// Import format (postman, insomnia, curl)
    pub format: String,
    /// Import data (file content or URL)
    pub data: String,
    /// Folder ID to import into (optional)
    pub folder_id: Option<String>,
    /// Whether to create folders from import structure
    pub create_folders: Option<bool>,
    /// Indices of routes to import (for selective import)
    pub selected_routes: Option<Vec<usize>>,
}

/// Export workspaces request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportWorkspacesRequest {
    /// Workspace IDs to export
    pub workspace_ids: Vec<String>,
}

/// Workspace export data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceExportData {
    /// Exported workspaces
    pub workspaces: Vec<mockforge_core::Workspace>,
    /// Export version
    pub version: String,
    /// Export timestamp
    pub exported_at: chrono::DateTime<chrono::Utc>,
    /// Exporter version
    pub exporter_version: String,
}

/// Environment color information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentColor {
    /// Hex color code (e.g., "#FF5733")
    pub hex: String,
    /// Optional color name for accessibility
    pub name: Option<String>,
}

/// Environment summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSummary {
    /// Environment ID
    pub id: String,
    /// Environment name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Color for visual distinction
    pub color: Option<EnvironmentColor>,
    /// Number of variables
    pub variable_count: usize,
    /// Whether this is the active environment
    pub active: bool,
    /// Whether this is the global environment
    pub is_global: bool,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Environment variable information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    /// Variable name
    pub name: String,
    /// Variable value
    pub value: String,
    /// Whether this variable is from the global environment
    pub from_global: bool,
}

/// Create environment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEnvironmentRequest {
    /// Environment name
    pub name: String,
    /// Description
    pub description: Option<String>,
}

/// Update environment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEnvironmentRequest {
    /// Environment name
    pub name: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Color
    pub color: Option<EnvironmentColor>,
}

/// Set variable request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetVariableRequest {
    /// Variable name
    pub name: String,
    /// Variable value
    pub value: String,
}

/// Directory sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Enable directory syncing for this workspace
    pub enabled: bool,
    /// Target directory for sync (relative or absolute path)
    pub target_directory: Option<String>,
    /// Directory structure to use (flat, nested, grouped)
    pub directory_structure: SyncDirectoryStructure,
    /// Auto-sync direction (one-way workspace→directory, bidirectional, or manual)
    pub sync_direction: SyncDirection,
    /// Whether to include metadata files
    pub include_metadata: bool,
    /// Filesystem monitoring enabled for real-time sync
    pub realtime_monitoring: bool,
    /// Custom filename pattern for exported files
    pub filename_pattern: String,
    /// Regular expression for excluding workspaces/requests
    pub exclude_pattern: Option<String>,
    /// Force overwrite existing files during sync
    pub force_overwrite: bool,
}

/// Directory structure options for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncDirectoryStructure {
    /// All workspaces in flat structure: workspace-name.yaml
    Flat,
    /// Nested by workspace: workspaces/{name}/workspace.yaml + requests/
    Nested,
    /// Grouped by type: requests/, responses/, metadata/
    Grouped,
}

/// Sync direction options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncDirection {
    /// Manual sync only (one-off operations)
    Manual,
    /// One-way: workspace changes sync silently to directory
    WorkspaceToDirectory,
    /// Bidirectional: changes in either direction trigger sync
    Bidirectional,
}

impl From<mockforge_core::workspace::SyncDirection> for SyncDirection {
    fn from(core: mockforge_core::workspace::SyncDirection) -> Self {
        match core {
            mockforge_core::workspace::SyncDirection::Manual => Self::Manual,
            mockforge_core::workspace::SyncDirection::WorkspaceToDirectory => Self::WorkspaceToDirectory,
            mockforge_core::workspace::SyncDirection::Bidirectional => Self::Bidirectional,
        }
    }
}

/// Sync status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Workspace ID
    pub workspace_id: String,
    /// Whether sync is enabled
    pub enabled: bool,
    /// Target directory
    pub target_directory: Option<String>,
    /// Current sync direction
    pub sync_direction: SyncDirection,
    /// Whether real-time monitoring is active
    pub realtime_monitoring: bool,
    /// Last sync timestamp
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    /// Sync status message
    pub status: String,
}

/// Sync change information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncChange {
    /// Change type
    pub change_type: String,
    /// File path
    pub path: String,
    /// Change description
    pub description: String,
    /// Whether this change requires confirmation
    pub requires_confirmation: bool,
}

/// Configure sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigureSyncRequest {
    /// Target directory
    pub target_directory: String,
    /// Sync direction
    pub sync_direction: SyncDirection,
    /// Enable real-time monitoring
    pub realtime_monitoring: bool,
    /// Directory structure
    pub directory_structure: Option<SyncDirectoryStructure>,
    /// Filename pattern
    pub filename_pattern: Option<String>,
}

/// Confirm sync changes request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmSyncChangesRequest {
    /// Workspace ID
    pub workspace_id: String,
    /// Changes to confirm
    pub changes: Vec<SyncChange>,
    /// Whether to apply all changes
    pub apply_all: bool,
}

/// Autocomplete suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteSuggestion {
    /// Suggestion text
    pub text: String,
    /// Suggestion type (e.g., "variable", "template")
    pub kind: String,
    /// Optional description
    pub description: Option<String>,
}

/// Autocomplete request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteRequest {
    /// Current input text
    pub input: String,
    /// Cursor position in the text
    pub cursor_position: usize,
    /// Context type (e.g., "header", "body", "url")
    pub context: Option<String>,
}

/// Autocomplete response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteResponse {
    /// List of suggestions
    pub suggestions: Vec<AutocompleteSuggestion>,
    /// Start position of the token being completed
    pub start_position: usize,
    /// End position of the token being completed
    pub end_position: usize,
}
