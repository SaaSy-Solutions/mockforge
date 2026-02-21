//! Data source plugin interface
//!
//! This module defines the DataSourcePlugin trait and related types for implementing
//! external data source integrations in MockForge. Data source plugins enable
//! connecting to databases, APIs, files, and other data sources for enhanced mocking.

use crate::{PluginCapabilities, PluginContext, PluginResult, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Data source plugin trait
///
/// Implement this trait to create custom data source integrations.
/// Data source plugins enable MockForge to connect to external data sources
/// like databases, REST APIs, files, and other systems for dynamic mocking.
#[async_trait::async_trait]
pub trait DataSourcePlugin: Send + Sync {
    /// Get plugin capabilities (permissions and limits)
    fn capabilities(&self) -> PluginCapabilities;

    /// Initialize the plugin with configuration
    async fn initialize(&self, config: &DataSourcePluginConfig) -> Result<()>;

    /// Connect to the data source
    ///
    /// This method establishes a connection to the data source using
    /// the provided configuration.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `config` - Data source configuration
    ///
    /// # Returns
    /// Connection handle
    async fn connect(
        &self,
        context: &PluginContext,
        config: &DataSourcePluginConfig,
    ) -> Result<PluginResult<DataConnection>>;

    /// Execute a query against the data source
    ///
    /// This method executes a query using the provided connection.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `connection` - Active connection
    /// * `query` - Query to execute
    /// * `config` - Data source configuration
    ///
    /// # Returns
    /// Query results
    async fn query(
        &self,
        context: &PluginContext,
        connection: &DataConnection,
        query: &DataQuery,
        config: &DataSourcePluginConfig,
    ) -> Result<PluginResult<DataResult>>;

    /// Get data source schema information
    ///
    /// This method retrieves schema information about the data source,
    /// such as available tables, columns, and relationships.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `connection` - Active connection
    /// * `config` - Data source configuration
    ///
    /// # Returns
    /// Schema information
    async fn get_schema(
        &self,
        context: &PluginContext,
        connection: &DataConnection,
        config: &DataSourcePluginConfig,
    ) -> Result<PluginResult<Schema>>;

    /// Test the data source connection
    ///
    /// This method tests whether the data source is accessible and
    /// the configuration is correct.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `config` - Data source configuration
    ///
    /// # Returns
    /// Connection test result
    async fn test_connection(
        &self,
        context: &PluginContext,
        config: &DataSourcePluginConfig,
    ) -> Result<PluginResult<ConnectionTestResult>>;

    /// Validate plugin configuration
    fn validate_config(&self, config: &DataSourcePluginConfig) -> Result<()>;

    /// Get supported data source types
    fn supported_types(&self) -> Vec<String>;

    /// Cleanup plugin resources
    async fn cleanup(&self) -> Result<()>;
}

/// Data source plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourcePluginConfig {
    /// Plugin-specific configuration
    pub config: HashMap<String, Value>,
    /// Enable/disable the plugin
    pub enabled: bool,
    /// Data source type (e.g., "postgresql", "mysql", "api", "file")
    pub data_source_type: String,
    /// Connection string or endpoint URL
    pub connection_string: Option<String>,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Query timeout in seconds
    pub query_timeout_secs: u64,
    /// Maximum connections
    pub max_connections: u32,
    /// Authentication credentials
    pub credentials: Option<DataSourceCredentials>,
    /// SSL/TLS configuration
    pub ssl_config: Option<SslConfig>,
    /// Custom settings
    pub settings: HashMap<String, Value>,
}

impl Default for DataSourcePluginConfig {
    fn default() -> Self {
        Self {
            config: HashMap::new(),
            enabled: true,
            data_source_type: "unknown".to_string(),
            connection_string: None,
            connection_timeout_secs: 30,
            query_timeout_secs: 30,
            max_connections: 10,
            credentials: None,
            ssl_config: None,
            settings: HashMap::new(),
        }
    }
}

/// Data source credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceCredentials {
    /// Username
    pub username: Option<String>,
    /// Password
    pub password: Option<String>,
    /// API key
    pub api_key: Option<String>,
    /// Bearer token
    pub bearer_token: Option<String>,
    /// Custom authentication fields
    pub custom: HashMap<String, String>,
}

impl DataSourceCredentials {
    /// Create with username/password
    pub fn user_pass<S: Into<String>>(username: S, password: S) -> Self {
        Self {
            username: Some(username.into()),
            password: Some(password.into()),
            api_key: None,
            bearer_token: None,
            custom: HashMap::new(),
        }
    }

    /// Create with API key
    pub fn api_key<S: Into<String>>(api_key: S) -> Self {
        Self {
            username: None,
            password: None,
            api_key: Some(api_key.into()),
            bearer_token: None,
            custom: HashMap::new(),
        }
    }

    /// Create with bearer token
    pub fn bearer_token<S: Into<String>>(token: S) -> Self {
        Self {
            username: None,
            password: None,
            api_key: None,
            bearer_token: Some(token.into()),
            custom: HashMap::new(),
        }
    }
}

/// SSL/TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SslConfig {
    /// Enable SSL/TLS
    pub enabled: bool,
    /// CA certificate path
    pub ca_cert_path: Option<String>,
    /// Client certificate path
    pub client_cert_path: Option<String>,
    /// Client key path
    pub client_key_path: Option<String>,
    /// Skip certificate verification (for development)
    pub skip_verify: bool,
    /// Custom SSL settings
    pub custom: HashMap<String, Value>,
}

/// Data connection handle
#[derive(Debug, Clone)]
pub struct DataConnection {
    /// Connection ID
    pub id: String,
    /// Connection type
    pub connection_type: String,
    /// Connection metadata
    pub metadata: HashMap<String, Value>,
    /// Connection creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last used time
    pub last_used: chrono::DateTime<chrono::Utc>,
    /// Internal connection handle (plugin-specific)
    pub handle: Value,
}

impl DataConnection {
    /// Create a new connection
    pub fn new<S: Into<String>>(connection_type: S, handle: Value) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            connection_type: connection_type.into(),
            metadata: HashMap::new(),
            created_at: now,
            last_used: now,
            handle,
        }
    }

    /// Update last used timestamp
    pub fn mark_used(&mut self) {
        self.last_used = chrono::Utc::now();
    }

    /// Add metadata
    pub fn with_metadata<S: Into<String>>(mut self, key: S, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get metadata value
    pub fn metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// Check if connection is stale (older than specified duration)
    pub fn is_stale(&self, max_age: chrono::Duration) -> bool {
        chrono::Utc::now().signed_duration_since(self.last_used) > max_age
    }
}

/// Data query specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQuery {
    /// Query type
    pub query_type: QueryType,
    /// Query string or specification
    pub query: String,
    /// Query parameters
    pub parameters: HashMap<String, Value>,
    /// Result limit
    pub limit: Option<usize>,
    /// Result offset
    pub offset: Option<usize>,
    /// Sort specification
    pub sort: Option<Vec<SortField>>,
    /// Filter conditions
    pub filters: Vec<QueryFilter>,
    /// Custom query options
    pub options: HashMap<String, Value>,
}

impl DataQuery {
    /// Create a simple SELECT query
    pub fn select<S: Into<String>>(query: S) -> Self {
        Self {
            query_type: QueryType::Select,
            query: query.into(),
            parameters: HashMap::new(),
            limit: None,
            offset: None,
            sort: None,
            filters: Vec::new(),
            options: HashMap::new(),
        }
    }

    /// Create an INSERT query
    pub fn insert<S: Into<String>>(query: S) -> Self {
        Self {
            query_type: QueryType::Insert,
            query: query.into(),
            parameters: HashMap::new(),
            limit: None,
            offset: None,
            sort: None,
            filters: Vec::new(),
            options: HashMap::new(),
        }
    }

    /// Create an UPDATE query
    pub fn update<S: Into<String>>(query: S) -> Self {
        Self {
            query_type: QueryType::Update,
            query: query.into(),
            parameters: HashMap::new(),
            limit: None,
            offset: None,
            sort: None,
            filters: Vec::new(),
            options: HashMap::new(),
        }
    }

    /// Create a DELETE query
    pub fn delete<S: Into<String>>(query: S) -> Self {
        Self {
            query_type: QueryType::Delete,
            query: query.into(),
            parameters: HashMap::new(),
            limit: None,
            offset: None,
            sort: None,
            filters: Vec::new(),
            options: HashMap::new(),
        }
    }

    /// Add a parameter
    pub fn with_parameter<S: Into<String>>(mut self, key: S, value: Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }

    /// Set limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set offset
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Add sort field
    pub fn with_sort(mut self, field: SortField) -> Self {
        self.sort.get_or_insert_with(Vec::new).push(field);
        self
    }

    /// Add filter
    pub fn with_filter(mut self, filter: QueryFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Add option
    pub fn with_option<S: Into<String>>(mut self, key: S, value: Value) -> Self {
        self.options.insert(key.into(), value);
        self
    }
}

/// Query type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    /// SELECT query
    Select,
    /// INSERT query
    Insert,
    /// UPDATE query
    Update,
    /// DELETE query
    Delete,
    /// Custom query type
    Custom(String),
}

impl fmt::Display for QueryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryType::Select => write!(f, "SELECT"),
            QueryType::Insert => write!(f, "INSERT"),
            QueryType::Update => write!(f, "UPDATE"),
            QueryType::Delete => write!(f, "DELETE"),
            QueryType::Custom(custom) => write!(f, "{}", custom),
        }
    }
}

use std::fmt;

/// Sort field specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortField {
    /// Field name
    pub field: String,
    /// Sort direction
    pub direction: SortDirection,
}

impl SortField {
    /// Create ascending sort
    pub fn asc<S: Into<String>>(field: S) -> Self {
        Self {
            field: field.into(),
            direction: SortDirection::Ascending,
        }
    }

    /// Create descending sort
    pub fn desc<S: Into<String>>(field: S) -> Self {
        Self {
            field: field.into(),
            direction: SortDirection::Descending,
        }
    }
}

/// Sort direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    /// Ascending order
    Ascending,
    /// Descending order
    Descending,
}

/// Query filter specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    /// Field name
    pub field: String,
    /// Filter operator
    pub operator: FilterOperator,
    /// Filter value
    pub value: Value,
    /// Logical AND/OR with next filter
    pub logical_op: Option<LogicalOperator>,
}

impl QueryFilter {
    /// Create equality filter
    pub fn equals<S: Into<String>>(field: S, value: Value) -> Self {
        Self {
            field: field.into(),
            operator: FilterOperator::Equals,
            value,
            logical_op: None,
        }
    }

    /// Create greater than filter
    pub fn greater_than<S: Into<String>>(field: S, value: Value) -> Self {
        Self {
            field: field.into(),
            operator: FilterOperator::GreaterThan,
            value,
            logical_op: None,
        }
    }

    /// Create less than filter
    pub fn less_than<S: Into<String>>(field: S, value: Value) -> Self {
        Self {
            field: field.into(),
            operator: FilterOperator::LessThan,
            value,
            logical_op: None,
        }
    }

    /// Create contains filter
    pub fn contains<S: Into<String>>(field: S, value: Value) -> Self {
        Self {
            field: field.into(),
            operator: FilterOperator::Contains,
            value,
            logical_op: None,
        }
    }

    /// Add logical AND
    pub fn and(mut self) -> Self {
        self.logical_op = Some(LogicalOperator::And);
        self
    }

    /// Add logical OR
    pub fn or(mut self) -> Self {
        self.logical_op = Some(LogicalOperator::Or);
        self
    }
}

/// Filter operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    /// Equal to
    Equals,
    /// Not equal to
    NotEquals,
    /// Greater than
    GreaterThan,
    /// Greater than or equal
    GreaterThanOrEqual,
    /// Less than
    LessThan,
    /// Less than or equal
    LessThanOrEqual,
    /// Contains (for strings/arrays)
    Contains,
    /// Starts with (for strings)
    StartsWith,
    /// Ends with (for strings)
    EndsWith,
    /// In array
    In,
    /// Not in array
    NotIn,
    /// Is null
    IsNull,
    /// Is not null
    IsNotNull,
}

/// Logical operator for combining filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    /// Logical AND
    And,
    /// Logical OR
    Or,
}

/// Query result data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataResult {
    /// Result rows
    pub rows: Vec<DataRow>,
    /// Column metadata
    pub columns: Vec<ColumnInfo>,
    /// Total number of rows (for pagination)
    pub total_count: Option<usize>,
    /// Query execution time
    pub execution_time_ms: u64,
    /// Custom result metadata
    pub metadata: HashMap<String, Value>,
}

impl DataResult {
    /// Create empty result
    pub fn empty() -> Self {
        Self {
            rows: Vec::new(),
            columns: Vec::new(),
            total_count: Some(0),
            execution_time_ms: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create result with rows
    pub fn with_rows(rows: Vec<DataRow>, columns: Vec<ColumnInfo>) -> Self {
        let row_count = rows.len();
        Self {
            rows,
            columns,
            total_count: Some(row_count),
            execution_time_ms: 0,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata<S: Into<String>>(mut self, key: S, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set execution time
    pub fn with_execution_time(mut self, time_ms: u64) -> Self {
        self.execution_time_ms = time_ms;
        self
    }

    /// Get row count
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get column count
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Convert to JSON array
    pub fn to_json_array(&self) -> Result<Value> {
        let mut json_rows = Vec::new();
        for row in &self.rows {
            json_rows.push(row.to_json(&self.columns)?);
        }
        Ok(Value::Array(json_rows))
    }
}

/// Data row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRow {
    /// Row values (indexed by column position)
    pub values: Vec<Value>,
    /// Row metadata
    pub metadata: HashMap<String, Value>,
}

impl DataRow {
    /// Create new row
    pub fn new(values: Vec<Value>) -> Self {
        Self {
            values,
            metadata: HashMap::new(),
        }
    }

    /// Get value by column index
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    /// Get value by column name
    pub fn get_by_name(&self, name: &str, columns: &[ColumnInfo]) -> Option<&Value> {
        columns
            .iter()
            .position(|col| col.name == name)
            .and_then(|index| self.get(index))
    }

    /// Convert row to JSON object
    pub fn to_json(&self, columns: &[ColumnInfo]) -> Result<Value> {
        let mut obj = serde_json::Map::new();
        for (i, value) in self.values.iter().enumerate() {
            if let Some(column) = columns.get(i) {
                obj.insert(column.name.clone(), value.clone());
            }
        }
        Ok(Value::Object(obj))
    }
}

/// Column information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// Column name
    pub name: String,
    /// Column data type
    pub data_type: DataType,
    /// Whether column is nullable
    pub nullable: bool,
    /// Column description
    pub description: Option<String>,
    /// Column metadata
    pub metadata: HashMap<String, Value>,
}

impl ColumnInfo {
    /// Create new column info
    pub fn new<S: Into<String>>(name: S, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: true,
            description: None,
            metadata: HashMap::new(),
        }
    }

    /// Set nullable
    pub fn nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    /// Set description
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add metadata
    pub fn with_metadata<S: Into<String>>(mut self, key: S, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Data type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    /// Text/string data
    Text,
    /// Integer number
    Integer,
    /// Floating point number
    Float,
    /// Boolean value
    Boolean,
    /// Date/time value
    DateTime,
    /// Binary data
    Binary,
    /// JSON data
    Json,
    /// UUID
    Uuid,
    /// Custom data type
    Custom(String),
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::Text => write!(f, "TEXT"),
            DataType::Integer => write!(f, "INTEGER"),
            DataType::Float => write!(f, "FLOAT"),
            DataType::Boolean => write!(f, "BOOLEAN"),
            DataType::DateTime => write!(f, "DATETIME"),
            DataType::Binary => write!(f, "BINARY"),
            DataType::Json => write!(f, "JSON"),
            DataType::Uuid => write!(f, "UUID"),
            DataType::Custom(custom) => write!(f, "{}", custom),
        }
    }
}

/// Data source schema information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Schema name
    pub name: Option<String>,
    /// Tables in the schema
    pub tables: Vec<TableInfo>,
    /// Custom schema metadata
    pub metadata: HashMap<String, Value>,
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

impl Schema {
    /// Create new schema
    pub fn new() -> Self {
        Self {
            name: None,
            tables: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add table
    pub fn with_table(mut self, table: TableInfo) -> Self {
        self.tables.push(table);
        self
    }

    /// Get table by name
    pub fn get_table(&self, name: &str) -> Option<&TableInfo> {
        self.tables.iter().find(|t| t.name == name)
    }

    /// Get all table names
    pub fn table_names(&self) -> Vec<&str> {
        self.tables.iter().map(|t| t.name.as_str()).collect()
    }
}

/// Table information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Table name
    pub name: String,
    /// Columns in the table
    pub columns: Vec<ColumnInfo>,
    /// Primary key columns
    pub primary_keys: Vec<String>,
    /// Foreign key relationships
    pub foreign_keys: Vec<ForeignKey>,
    /// Table description
    pub description: Option<String>,
    /// Row count (if available)
    pub row_count: Option<usize>,
    /// Custom table metadata
    pub metadata: HashMap<String, Value>,
}

impl TableInfo {
    /// Create new table info
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            primary_keys: Vec::new(),
            foreign_keys: Vec::new(),
            description: None,
            row_count: None,
            metadata: HashMap::new(),
        }
    }

    /// Add column
    pub fn with_column(mut self, column: ColumnInfo) -> Self {
        self.columns.push(column);
        self
    }

    /// Add primary key
    pub fn with_primary_key<S: Into<String>>(mut self, column: S) -> Self {
        self.primary_keys.push(column.into());
        self
    }

    /// Add foreign key
    pub fn with_foreign_key(mut self, fk: ForeignKey) -> Self {
        self.foreign_keys.push(fk);
        self
    }

    /// Set description
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set row count
    pub fn row_count(mut self, count: usize) -> Self {
        self.row_count = Some(count);
        self
    }

    /// Get column by name
    pub fn get_column(&self, name: &str) -> Option<&ColumnInfo> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Check if column is primary key
    pub fn is_primary_key(&self, column: &str) -> bool {
        self.primary_keys.contains(&column.to_string())
    }
}

/// Foreign key relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    /// Local column name
    pub column: String,
    /// Referenced table name
    pub referenced_table: String,
    /// Referenced column name
    pub referenced_column: String,
    /// Relationship name
    pub name: Option<String>,
}

/// Connection test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    /// Test successful
    pub success: bool,
    /// Test message
    pub message: String,
    /// Connection latency in milliseconds
    pub latency_ms: Option<u64>,
    /// Test metadata
    pub metadata: HashMap<String, Value>,
}

impl ConnectionTestResult {
    /// Create successful test result
    pub fn success<S: Into<String>>(message: S) -> Self {
        Self {
            success: true,
            message: message.into(),
            latency_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Create failed test result
    pub fn failure<S: Into<String>>(message: S) -> Self {
        Self {
            success: false,
            message: message.into(),
            latency_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Set latency
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    /// Add metadata
    pub fn with_metadata<S: Into<String>>(mut self, key: S, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Data source plugin registry entry
pub struct DataSourcePluginEntry {
    /// Plugin ID
    pub plugin_id: crate::PluginId,
    /// Plugin instance
    pub plugin: std::sync::Arc<dyn DataSourcePlugin>,
    /// Plugin configuration
    pub config: DataSourcePluginConfig,
    /// Plugin capabilities
    pub capabilities: PluginCapabilities,
}

impl DataSourcePluginEntry {
    /// Create new plugin entry
    pub fn new(
        plugin_id: crate::PluginId,
        plugin: std::sync::Arc<dyn DataSourcePlugin>,
        config: DataSourcePluginConfig,
    ) -> Self {
        let capabilities = plugin.capabilities();
        Self {
            plugin_id,
            plugin,
            config,
            capabilities,
        }
    }

    /// Check if plugin is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if plugin supports a data source type
    pub fn supports_type(&self, data_type: &str) -> bool {
        self.config.data_source_type == data_type
    }
}

/// Helper trait for creating data source plugins
pub trait DataSourcePluginFactory: Send + Sync {
    /// Create a new data source plugin instance
    fn create_plugin(&self) -> Result<Box<dyn DataSourcePlugin>>;
}

/// Built-in data source helpers
pub mod helpers {
    use super::*;

    /// Create a simple in-memory data source for testing
    pub fn create_memory_data_source() -> Vec<DataRow> {
        vec![
            DataRow::new(vec![
                Value::String("John".to_string()),
                Value::String("Doe".to_string()),
                Value::Number(30.into()),
            ]),
            DataRow::new(vec![
                Value::String("Jane".to_string()),
                Value::String("Smith".to_string()),
                Value::Number(25.into()),
            ]),
        ]
    }

    /// Create sample column info for testing
    pub fn create_sample_columns() -> Vec<ColumnInfo> {
        vec![
            ColumnInfo::new("first_name", DataType::Text).nullable(false),
            ColumnInfo::new("last_name", DataType::Text).nullable(false),
            ColumnInfo::new("age", DataType::Integer).nullable(false),
        ]
    }

    /// Create sample schema for testing
    pub fn create_sample_schema() -> Schema {
        let table = TableInfo::new("users")
            .with_column(ColumnInfo::new("id", DataType::Integer).nullable(false))
            .with_column(ColumnInfo::new("first_name", DataType::Text).nullable(false))
            .with_column(ColumnInfo::new("last_name", DataType::Text).nullable(false))
            .with_column(ColumnInfo::new("email", DataType::Text).nullable(false))
            .with_primary_key("id");

        Schema::new().with_table(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== DataSourcePluginConfig Tests ====================

    #[test]
    fn test_data_source_plugin_config_default() {
        let config = DataSourcePluginConfig::default();

        assert!(config.config.is_empty());
        assert!(config.enabled);
        assert_eq!(config.data_source_type, "unknown");
        assert!(config.connection_string.is_none());
        assert_eq!(config.connection_timeout_secs, 30);
        assert_eq!(config.query_timeout_secs, 30);
        assert_eq!(config.max_connections, 10);
        assert!(config.credentials.is_none());
        assert!(config.ssl_config.is_none());
        assert!(config.settings.is_empty());
    }

    #[test]
    fn test_data_source_plugin_config_custom() {
        let config = DataSourcePluginConfig {
            config: HashMap::from([("key".to_string(), Value::String("value".to_string()))]),
            enabled: false,
            data_source_type: "postgresql".to_string(),
            connection_string: Some("postgres://localhost:5432".to_string()),
            connection_timeout_secs: 60,
            query_timeout_secs: 120,
            max_connections: 20,
            credentials: Some(DataSourceCredentials::user_pass("user", "pass")),
            ssl_config: Some(SslConfig::default()),
            settings: HashMap::new(),
        };

        assert!(!config.config.is_empty());
        assert!(!config.enabled);
        assert_eq!(config.data_source_type, "postgresql");
        assert!(config.connection_string.is_some());
    }

    #[test]
    fn test_data_source_plugin_config_clone() {
        let config = DataSourcePluginConfig::default();
        let cloned = config.clone();

        assert_eq!(cloned.data_source_type, config.data_source_type);
        assert_eq!(cloned.enabled, config.enabled);
    }

    #[test]
    fn test_data_source_plugin_config_serialization() {
        let config = DataSourcePluginConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DataSourcePluginConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.data_source_type, config.data_source_type);
    }

    // ==================== DataSourceCredentials Tests ====================

    #[test]
    fn test_credentials_user_pass() {
        let creds = DataSourceCredentials::user_pass("admin", "secret123");

        assert_eq!(creds.username.as_deref(), Some("admin"));
        assert_eq!(creds.password.as_deref(), Some("secret123"));
        assert!(creds.api_key.is_none());
        assert!(creds.bearer_token.is_none());
    }

    #[test]
    fn test_credentials_api_key() {
        let creds = DataSourceCredentials::api_key("my-api-key-12345");

        assert!(creds.username.is_none());
        assert!(creds.password.is_none());
        assert_eq!(creds.api_key.as_deref(), Some("my-api-key-12345"));
        assert!(creds.bearer_token.is_none());
    }

    #[test]
    fn test_credentials_bearer_token() {
        let creds = DataSourceCredentials::bearer_token("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");

        assert!(creds.username.is_none());
        assert!(creds.password.is_none());
        assert!(creds.api_key.is_none());
        assert_eq!(creds.bearer_token.as_deref(), Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }

    #[test]
    fn test_credentials_clone() {
        let creds = DataSourceCredentials::api_key("test");
        let cloned = creds.clone();

        assert_eq!(cloned.api_key, creds.api_key);
    }

    // ==================== SslConfig Tests ====================

    #[test]
    fn test_ssl_config_default() {
        let config = SslConfig::default();

        assert!(!config.enabled);
        assert!(config.ca_cert_path.is_none());
        assert!(config.client_cert_path.is_none());
        assert!(config.client_key_path.is_none());
        assert!(!config.skip_verify);
        assert!(config.custom.is_empty());
    }

    #[test]
    fn test_ssl_config_custom() {
        let config = SslConfig {
            enabled: true,
            ca_cert_path: Some("/certs/ca.pem".to_string()),
            client_cert_path: Some("/certs/client.pem".to_string()),
            client_key_path: Some("/certs/client.key".to_string()),
            skip_verify: false,
            custom: HashMap::new(),
        };

        assert!(config.enabled);
        assert_eq!(config.ca_cert_path.as_deref(), Some("/certs/ca.pem"));
    }

    // ==================== DataConnection Tests ====================

    #[test]
    fn test_data_connection_new() {
        let conn = DataConnection::new("postgresql", Value::Null);

        assert!(!conn.id.is_empty());
        assert_eq!(conn.connection_type, "postgresql");
        assert!(conn.metadata.is_empty());
    }

    #[test]
    fn test_data_connection_mark_used() {
        let mut conn = DataConnection::new("mysql", Value::Null);
        let original_last_used = conn.last_used;

        std::thread::sleep(std::time::Duration::from_millis(10));
        conn.mark_used();

        assert!(conn.last_used >= original_last_used);
    }

    #[test]
    fn test_data_connection_with_metadata() {
        let conn = DataConnection::new("api", Value::Null)
            .with_metadata("version", Value::String("v1".to_string()));

        assert!(conn.metadata("version").is_some());
        assert!(conn.metadata("nonexistent").is_none());
    }

    #[test]
    fn test_data_connection_is_stale() {
        let conn = DataConnection::new("test", Value::Null);

        // Should not be stale with a 1 hour max age
        assert!(!conn.is_stale(chrono::Duration::hours(1)));

        // Should be stale with 0 duration
        assert!(conn.is_stale(chrono::Duration::zero()));
    }

    #[test]
    fn test_data_connection_clone() {
        let conn = DataConnection::new("sqlite", Value::Null);
        let cloned = conn.clone();

        assert_eq!(cloned.id, conn.id);
        assert_eq!(cloned.connection_type, conn.connection_type);
    }

    // ==================== DataQuery Tests ====================

    #[test]
    fn test_data_query_select() {
        let query = DataQuery::select("SELECT * FROM users");

        assert!(matches!(query.query_type, QueryType::Select));
        assert_eq!(query.query, "SELECT * FROM users");
    }

    #[test]
    fn test_data_query_insert() {
        let query = DataQuery::insert("INSERT INTO users VALUES (?)");

        assert!(matches!(query.query_type, QueryType::Insert));
    }

    #[test]
    fn test_data_query_update() {
        let query = DataQuery::update("UPDATE users SET name = ?");

        assert!(matches!(query.query_type, QueryType::Update));
    }

    #[test]
    fn test_data_query_delete() {
        let query = DataQuery::delete("DELETE FROM users WHERE id = ?");

        assert!(matches!(query.query_type, QueryType::Delete));
    }

    #[test]
    fn test_data_query_with_parameter() {
        let query = DataQuery::select("SELECT * FROM users WHERE id = :id")
            .with_parameter("id", Value::Number(42.into()));

        assert!(query.parameters.contains_key("id"));
    }

    #[test]
    fn test_data_query_with_limit() {
        let query = DataQuery::select("SELECT * FROM users").with_limit(10);

        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn test_data_query_with_offset() {
        let query = DataQuery::select("SELECT * FROM users").with_offset(20);

        assert_eq!(query.offset, Some(20));
    }

    #[test]
    fn test_data_query_with_sort() {
        let query = DataQuery::select("SELECT * FROM users").with_sort(SortField::asc("name"));

        assert!(query.sort.is_some());
        assert_eq!(query.sort.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_data_query_with_filter() {
        let query = DataQuery::select("SELECT * FROM users")
            .with_filter(QueryFilter::equals("status", Value::String("active".to_string())));

        assert_eq!(query.filters.len(), 1);
    }

    #[test]
    fn test_data_query_with_option() {
        let query = DataQuery::select("SELECT * FROM users")
            .with_option("timeout", Value::Number(30.into()));

        assert!(query.options.contains_key("timeout"));
    }

    #[test]
    fn test_data_query_chained() {
        let query = DataQuery::select("SELECT * FROM users")
            .with_parameter("status", Value::String("active".to_string()))
            .with_limit(50)
            .with_offset(0)
            .with_sort(SortField::desc("created_at"));

        assert!(!query.parameters.is_empty());
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(0));
        assert!(query.sort.is_some());
    }

    // ==================== QueryType Tests ====================

    #[test]
    fn test_query_type_display_select() {
        assert_eq!(format!("{}", QueryType::Select), "SELECT");
    }

    #[test]
    fn test_query_type_display_insert() {
        assert_eq!(format!("{}", QueryType::Insert), "INSERT");
    }

    #[test]
    fn test_query_type_display_update() {
        assert_eq!(format!("{}", QueryType::Update), "UPDATE");
    }

    #[test]
    fn test_query_type_display_delete() {
        assert_eq!(format!("{}", QueryType::Delete), "DELETE");
    }

    #[test]
    fn test_query_type_display_custom() {
        assert_eq!(format!("{}", QueryType::Custom("MERGE".to_string())), "MERGE");
    }

    // ==================== SortField Tests ====================

    #[test]
    fn test_sort_field_asc() {
        let sort = SortField::asc("name");

        assert_eq!(sort.field, "name");
        assert!(matches!(sort.direction, SortDirection::Ascending));
    }

    #[test]
    fn test_sort_field_desc() {
        let sort = SortField::desc("created_at");

        assert_eq!(sort.field, "created_at");
        assert!(matches!(sort.direction, SortDirection::Descending));
    }

    // ==================== QueryFilter Tests ====================

    #[test]
    fn test_query_filter_equals() {
        let filter = QueryFilter::equals("status", Value::String("active".to_string()));

        assert_eq!(filter.field, "status");
        assert!(matches!(filter.operator, FilterOperator::Equals));
    }

    #[test]
    fn test_query_filter_greater_than() {
        let filter = QueryFilter::greater_than("age", Value::Number(18.into()));

        assert!(matches!(filter.operator, FilterOperator::GreaterThan));
    }

    #[test]
    fn test_query_filter_less_than() {
        let filter = QueryFilter::less_than("price", Value::Number(100.into()));

        assert!(matches!(filter.operator, FilterOperator::LessThan));
    }

    #[test]
    fn test_query_filter_contains() {
        let filter = QueryFilter::contains("name", Value::String("test".to_string()));

        assert!(matches!(filter.operator, FilterOperator::Contains));
    }

    #[test]
    fn test_query_filter_and() {
        let filter = QueryFilter::equals("status", Value::String("active".to_string())).and();

        assert!(matches!(filter.logical_op, Some(LogicalOperator::And)));
    }

    #[test]
    fn test_query_filter_or() {
        let filter = QueryFilter::equals("status", Value::String("pending".to_string())).or();

        assert!(matches!(filter.logical_op, Some(LogicalOperator::Or)));
    }

    // ==================== DataResult Tests ====================

    #[test]
    fn test_data_result_empty() {
        let result = DataResult::empty();

        assert!(result.rows.is_empty());
        assert!(result.columns.is_empty());
        assert_eq!(result.total_count, Some(0));
        assert_eq!(result.execution_time_ms, 0);
    }

    #[test]
    fn test_data_result_with_rows() {
        let rows = vec![
            DataRow::new(vec![Value::String("test".to_string())]),
            DataRow::new(vec![Value::String("test2".to_string())]),
        ];
        let columns = vec![ColumnInfo::new("name", DataType::Text)];

        let result = DataResult::with_rows(rows, columns);

        assert_eq!(result.row_count(), 2);
        assert_eq!(result.column_count(), 1);
    }

    #[test]
    fn test_data_result_with_metadata() {
        let result = DataResult::empty().with_metadata("source", Value::String("test".to_string()));

        assert!(result.metadata.contains_key("source"));
    }

    #[test]
    fn test_data_result_with_execution_time() {
        let result = DataResult::empty().with_execution_time(150);

        assert_eq!(result.execution_time_ms, 150);
    }

    #[test]
    fn test_data_result_to_json_array() {
        let rows = vec![DataRow::new(vec![Value::String("John".to_string())])];
        let columns = vec![ColumnInfo::new("name", DataType::Text)];
        let result = DataResult::with_rows(rows, columns);

        let json = result.to_json_array().unwrap();
        assert!(json.is_array());
    }

    // ==================== DataRow Tests ====================

    #[test]
    fn test_data_row_new() {
        let row = DataRow::new(vec![Value::Number(1.into()), Value::String("test".to_string())]);

        assert_eq!(row.values.len(), 2);
        assert!(row.metadata.is_empty());
    }

    #[test]
    fn test_data_row_get() {
        let row = DataRow::new(vec![Value::Number(42.into())]);

        assert!(row.get(0).is_some());
        assert!(row.get(1).is_none());
    }

    #[test]
    fn test_data_row_get_by_name() {
        let row = DataRow::new(vec![Value::String("John".to_string())]);
        let columns = vec![ColumnInfo::new("name", DataType::Text)];

        assert!(row.get_by_name("name", &columns).is_some());
        assert!(row.get_by_name("nonexistent", &columns).is_none());
    }

    #[test]
    fn test_data_row_to_json() {
        let row = DataRow::new(vec![Value::String("John".to_string()), Value::Number(30.into())]);
        let columns = vec![
            ColumnInfo::new("name", DataType::Text),
            ColumnInfo::new("age", DataType::Integer),
        ];

        let json = row.to_json(&columns).unwrap();
        assert!(json.is_object());
    }

    // ==================== ColumnInfo Tests ====================

    #[test]
    fn test_column_info_new() {
        let col = ColumnInfo::new("id", DataType::Integer);

        assert_eq!(col.name, "id");
        assert!(col.nullable);
        assert!(col.description.is_none());
    }

    #[test]
    fn test_column_info_nullable() {
        let col = ColumnInfo::new("id", DataType::Integer).nullable(false);

        assert!(!col.nullable);
    }

    #[test]
    fn test_column_info_description() {
        let col = ColumnInfo::new("id", DataType::Integer).description("Primary key");

        assert_eq!(col.description.as_deref(), Some("Primary key"));
    }

    #[test]
    fn test_column_info_with_metadata() {
        let col =
            ColumnInfo::new("id", DataType::Integer).with_metadata("index", Value::Bool(true));

        assert!(col.metadata.contains_key("index"));
    }

    // ==================== DataType Tests ====================

    #[test]
    fn test_data_type_display_text() {
        assert_eq!(format!("{}", DataType::Text), "TEXT");
    }

    #[test]
    fn test_data_type_display_integer() {
        assert_eq!(format!("{}", DataType::Integer), "INTEGER");
    }

    #[test]
    fn test_data_type_display_float() {
        assert_eq!(format!("{}", DataType::Float), "FLOAT");
    }

    #[test]
    fn test_data_type_display_boolean() {
        assert_eq!(format!("{}", DataType::Boolean), "BOOLEAN");
    }

    #[test]
    fn test_data_type_display_datetime() {
        assert_eq!(format!("{}", DataType::DateTime), "DATETIME");
    }

    #[test]
    fn test_data_type_display_custom() {
        assert_eq!(format!("{}", DataType::Custom("MONEY".to_string())), "MONEY");
    }

    // ==================== Schema Tests ====================

    #[test]
    fn test_schema_default() {
        let schema = Schema::default();

        assert!(schema.name.is_none());
        assert!(schema.tables.is_empty());
    }

    #[test]
    fn test_schema_new() {
        let schema = Schema::new();

        assert!(schema.tables.is_empty());
    }

    #[test]
    fn test_schema_with_table() {
        let schema = Schema::new().with_table(TableInfo::new("users"));

        assert_eq!(schema.tables.len(), 1);
    }

    #[test]
    fn test_schema_get_table() {
        let schema = Schema::new().with_table(TableInfo::new("users"));

        assert!(schema.get_table("users").is_some());
        assert!(schema.get_table("nonexistent").is_none());
    }

    #[test]
    fn test_schema_table_names() {
        let schema = Schema::new()
            .with_table(TableInfo::new("users"))
            .with_table(TableInfo::new("orders"));

        let names = schema.table_names();
        assert!(names.contains(&"users"));
        assert!(names.contains(&"orders"));
    }

    // ==================== TableInfo Tests ====================

    #[test]
    fn test_table_info_new() {
        let table = TableInfo::new("users");

        assert_eq!(table.name, "users");
        assert!(table.columns.is_empty());
        assert!(table.primary_keys.is_empty());
    }

    #[test]
    fn test_table_info_with_column() {
        let table = TableInfo::new("users").with_column(ColumnInfo::new("id", DataType::Integer));

        assert_eq!(table.columns.len(), 1);
    }

    #[test]
    fn test_table_info_with_primary_key() {
        let table = TableInfo::new("users").with_primary_key("id");

        assert!(table.is_primary_key("id"));
        assert!(!table.is_primary_key("name"));
    }

    #[test]
    fn test_table_info_with_foreign_key() {
        let fk = ForeignKey {
            column: "user_id".to_string(),
            referenced_table: "users".to_string(),
            referenced_column: "id".to_string(),
            name: Some("fk_orders_user".to_string()),
        };

        let table = TableInfo::new("orders").with_foreign_key(fk);

        assert_eq!(table.foreign_keys.len(), 1);
    }

    #[test]
    fn test_table_info_description() {
        let table = TableInfo::new("users").description("Stores user information");

        assert_eq!(table.description.as_deref(), Some("Stores user information"));
    }

    #[test]
    fn test_table_info_row_count() {
        let table = TableInfo::new("users").row_count(1000);

        assert_eq!(table.row_count, Some(1000));
    }

    #[test]
    fn test_table_info_get_column() {
        let table = TableInfo::new("users")
            .with_column(ColumnInfo::new("id", DataType::Integer))
            .with_column(ColumnInfo::new("name", DataType::Text));

        assert!(table.get_column("id").is_some());
        assert!(table.get_column("nonexistent").is_none());
    }

    // ==================== ForeignKey Tests ====================

    #[test]
    fn test_foreign_key_creation() {
        let fk = ForeignKey {
            column: "user_id".to_string(),
            referenced_table: "users".to_string(),
            referenced_column: "id".to_string(),
            name: None,
        };

        assert_eq!(fk.column, "user_id");
        assert_eq!(fk.referenced_table, "users");
    }

    // ==================== ConnectionTestResult Tests ====================

    #[test]
    fn test_connection_test_result_success() {
        let result = ConnectionTestResult::success("Connection successful");

        assert!(result.success);
        assert_eq!(result.message, "Connection successful");
    }

    #[test]
    fn test_connection_test_result_failure() {
        let result = ConnectionTestResult::failure("Connection refused");

        assert!(!result.success);
        assert_eq!(result.message, "Connection refused");
    }

    #[test]
    fn test_connection_test_result_with_latency() {
        let result = ConnectionTestResult::success("OK").with_latency(50);

        assert_eq!(result.latency_ms, Some(50));
    }

    #[test]
    fn test_connection_test_result_with_metadata() {
        let result = ConnectionTestResult::success("OK")
            .with_metadata("version", Value::String("5.7".to_string()));

        assert!(result.metadata.contains_key("version"));
    }

    // ==================== Helper Functions Tests ====================

    #[test]
    fn test_helpers_create_memory_data_source() {
        let rows = helpers::create_memory_data_source();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].values.len(), 3);
    }

    #[test]
    fn test_helpers_create_sample_columns() {
        let columns = helpers::create_sample_columns();

        assert_eq!(columns.len(), 3);
        assert_eq!(columns[0].name, "first_name");
    }

    #[test]
    fn test_helpers_create_sample_schema() {
        let schema = helpers::create_sample_schema();

        assert!(schema.get_table("users").is_some());
        let users = schema.get_table("users").unwrap();
        assert!(users.is_primary_key("id"));
    }
}
