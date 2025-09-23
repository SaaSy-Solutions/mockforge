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
    pub config: HashMap<String, serde_json::Value>,
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
    pub settings: HashMap<String, serde_json::Value>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for SslConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            skip_verify: false,
            custom: HashMap::new(),
        }
    }
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
        columns.iter().position(|col| col.name == name)
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
