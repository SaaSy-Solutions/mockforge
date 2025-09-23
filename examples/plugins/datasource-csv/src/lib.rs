//! # CSV Data Source Plugin for MockForge
//!
//! This plugin provides access to CSV files as mock data sources.
//! It allows loading CSV data and querying it for mock responses.
//!
//! ## Features
//!
//! - CSV file parsing and loading
//! - Data querying with filtering and pagination
//! - Multiple CSV datasets support
//! - Caching for performance
//! - Type inference for CSV columns

use mockforge_plugin_core::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// CSV file configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvFileConfig {
    /// Dataset name for this CSV file
    pub name: String,
    /// Path to the CSV file
    pub path: String,
    /// Whether the CSV file has headers
    pub has_headers: bool,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvDataSourceConfig {
    /// List of CSV files to load
    pub csv_files: Vec<CsvFileConfig>,
    /// Enable data caching
    pub cache_enabled: bool,
    /// Maximum rows to return per query
    pub max_rows_per_query: usize,
}

impl Default for CsvDataSourceConfig {
    fn default() -> Self {
        Self {
            csv_files: vec![],
            cache_enabled: true,
            max_rows_per_query: 1000,
        }
    }
}

/// Loaded CSV dataset
#[derive(Debug, Clone)]
pub struct CsvDataset {
    /// Dataset name
    pub name: String,
    /// Column headers (if available)
    pub headers: Vec<String>,
    /// Data rows
    pub rows: Vec<HashMap<String, String>>,
    /// Column types (inferred)
    pub column_types: HashMap<String, ColumnType>,
}

/// Inferred column type
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnType {
    String,
    Integer,
    Float,
    Boolean,
}

/// CSV Data Source Plugin
pub struct CsvDataSourcePlugin {
    config: CsvDataSourceConfig,
    datasets: HashMap<String, CsvDataset>,
    cache: HashMap<String, Vec<HashMap<String, String>>>,
}

impl CsvDataSourcePlugin {
    /// Create a new CSV data source plugin
    pub fn new(config: CsvDataSourceConfig) -> Self {
        let mut plugin = Self {
            config,
            datasets: HashMap::new(),
            cache: HashMap::new(),
        };

        // Load all configured CSV files
        for csv_config in &plugin.config.csv_files {
            if let Err(e) = plugin.load_csv_dataset(csv_config) {
                eprintln!("Failed to load CSV dataset {}: {}", csv_config.name, e);
            }
        }

        plugin
    }

    /// Load a CSV dataset
    fn load_csv_dataset(&mut self, csv_config: &CsvFileConfig) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(&csv_config.path);

        // Read CSV file
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(csv_config.has_headers)
            .from_path(path)?;

        let mut headers = Vec::new();
        let mut rows = Vec::new();
        let mut column_types = HashMap::new();

        // Get headers
        if csv_config.has_headers {
            headers = reader.headers()?.iter().map(|s| s.to_string()).collect();
        } else {
            // Generate column names for headerless CSV
            if let Some(first_record) = reader.records().next() {
                let record = first_record?;
                headers = (0..record.len()).map(|i| format!("col{}", i + 1)).collect();

                // Create a row from the first record
                let mut row_data = HashMap::new();
                for (i, field) in record.iter().enumerate() {
                    let col_name = &headers[i];
                    row_data.insert(col_name.clone(), field.to_string());
                    self.infer_column_type(&mut column_types, col_name, field);
                }
                rows.push(row_data);
            }
        }

        // Read all records
        for result in reader.records() {
            let record = result?;
            let mut row_data = HashMap::new();

            for (i, field) in record.iter().enumerate() {
                let col_name = if i < headers.len() {
                    &headers[i]
                } else {
                    continue; // Skip extra fields
                };

                row_data.insert(col_name.clone(), field.to_string());
                self.infer_column_type(&mut column_types, col_name, field);
            }

            rows.push(row_data);
        }

        let dataset = CsvDataset {
            name: csv_config.name.clone(),
            headers,
            rows,
            column_types,
        };

        self.datasets.insert(csv_config.name.clone(), dataset);
        Ok(())
    }

    /// Infer column type from field value
    fn infer_column_type(&self, types: &mut HashMap<String, ColumnType>, column: &str, value: &str) {
        if types.contains_key(column) {
            return; // Type already inferred
        }

        let trimmed = value.trim();

        // Try to parse as boolean
        if trimmed.eq_ignore_ascii_case("true") || trimmed.eq_ignore_ascii_case("false") {
            types.insert(column.to_string(), ColumnType::Boolean);
            return;
        }

        // Try to parse as integer
        if trimmed.parse::<i64>().is_ok() {
            types.insert(column.to_string(), ColumnType::Integer);
            return;
        }

        // Try to parse as float
        if trimmed.parse::<f64>().is_ok() {
            types.insert(column.to_string(), ColumnType::Float);
            return;
        }

        // Default to string
        types.insert(column.to_string(), ColumnType::String);
    }

    /// Query a dataset
    fn query_dataset(
        &self,
        dataset_name: &str,
        query: &DataSourceQuery,
    ) -> Result<DataSet, DataSourceError> {
        let dataset = self.datasets.get(dataset_name)
            .ok_or_else(|| DataSourceError::NotFound(format!("Dataset '{}' not found", dataset_name)))?;

        let mut filtered_rows = dataset.rows.clone();

        // Apply filters
        for filter in &query.filters {
            filtered_rows.retain(|row| self.matches_filter(row, filter));
        }

        // Apply sorting
        if let Some(sort) = &query.sort {
            filtered_rows.sort_by(|a, b| {
                let a_val = a.get(&sort.field).unwrap_or(&"".to_string());
                let b_val = b.get(&sort.field).unwrap_or(&"".to_string());

                match sort.direction {
                    SortDirection::Asc => a_val.cmp(b_val),
                    SortDirection::Desc => b_val.cmp(a_val),
                }
            });
        }

        // Apply pagination
        let start_idx = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(self.config.max_rows_per_query).min(self.config.max_rows_per_query);
        let end_idx = (start_idx + limit).min(filtered_rows.len());

        let paginated_rows = if start_idx < filtered_rows.len() {
            filtered_rows[start_idx..end_idx].to_vec()
        } else {
            vec![]
        };

        // Convert to DataSet format
        let columns: Vec<ColumnInfo> = dataset.headers.iter().map(|header| {
            let col_type = dataset.column_types.get(header)
                .cloned()
                .unwrap_or(ColumnType::String);

            let data_type = match col_type {
                ColumnType::String => DataType::String,
                ColumnType::Integer => DataType::Integer,
                ColumnType::Float => DataType::Float,
                ColumnType::Boolean => DataType::Boolean,
            };

            ColumnInfo {
                name: header.clone(),
                data_type,
                nullable: true, // Assume all columns can be null
            }
        }).collect();

        let rows: Vec<DataRow> = paginated_rows.into_iter().map(|row_map| {
            let values: Vec<serde_json::Value> = dataset.headers.iter().map(|header| {
                row_map.get(header)
                    .map(|v| serde_json::Value::String(v.clone()))
                    .unwrap_or(serde_json::Value::Null)
            }).collect();

            DataRow { values }
        }).collect();

        Ok(DataSet {
            columns,
            rows,
            total_count: filtered_rows.len() as u64,
        })
    }

    /// Check if a row matches a filter
    fn matches_filter(&self, row: &HashMap<String, String>, filter: &DataFilter) -> bool {
        let field_value = match row.get(&filter.field) {
            Some(value) => value,
            None => return false,
        };

        match &filter.operator {
            FilterOperator::Eq => field_value == &filter.value.to_string(),
            FilterOperator::Ne => field_value != &filter.value.to_string(),
            FilterOperator::Gt => {
                // Try numeric comparison
                if let (Ok(a), Ok(b)) = (field_value.parse::<f64>(), filter.value.parse::<f64>()) {
                    a > b
                } else {
                    field_value > &filter.value.to_string()
                }
            }
            FilterOperator::Lt => {
                if let (Ok(a), Ok(b)) = (field_value.parse::<f64>(), filter.value.parse::<f64>()) {
                    a < b
                } else {
                    field_value < &filter.value.to_string()
                }
            }
            FilterOperator::Gte => {
                if let (Ok(a), Ok(b)) = (field_value.parse::<f64>(), filter.value.parse::<f64>()) {
                    a >= b
                } else {
                    field_value >= &filter.value.to_string()
                }
            }
            FilterOperator::Lte => {
                if let (Ok(a), Ok(b)) = (field_value.parse::<f64>(), filter.value.parse::<f64>()) {
                    a <= b
                } else {
                    field_value <= &filter.value.to_string()
                }
            }
            FilterOperator::Contains => field_value.contains(&filter.value.to_string()),
            FilterOperator::StartsWith => field_value.starts_with(&filter.value.to_string()),
            FilterOperator::EndsWith => field_value.ends_with(&filter.value.to_string()),
        }
    }

    /// Get dataset metadata
    fn get_dataset_metadata(&self, dataset_name: &str) -> Option<DatasetMetadata> {
        self.datasets.get(dataset_name).map(|dataset| {
            DatasetMetadata {
                name: dataset.name.clone(),
                description: format!("CSV dataset loaded from {}", dataset_name),
                row_count: dataset.rows.len() as u64,
                column_count: dataset.headers.len() as u32,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }
        })
    }
}

impl DataSourcePlugin for CsvDataSourcePlugin {
    fn query(
        &self,
        dataset_name: &str,
        query: &DataSourceQuery,
        _config: &DataSourcePluginConfig,
    ) -> PluginResult<DataSet> {
        match self.query_dataset(dataset_name, query) {
            Ok(dataset) => PluginResult::success(dataset, 0),
            Err(e) => PluginResult::failure(format!("Query failed: {}", e), 0),
        }
    }

    fn list_datasets(&self) -> PluginResult<Vec<DatasetMetadata>> {
        let datasets: Vec<DatasetMetadata> = self.datasets.keys()
            .filter_map(|name| self.get_dataset_metadata(name))
            .collect();

        PluginResult::success(datasets, 0)
    }

    fn get_dataset_info(&self, dataset_name: &str) -> PluginResult<DatasetMetadata> {
        match self.get_dataset_metadata(dataset_name) {
            Some(metadata) => PluginResult::success(metadata, 0),
            None => PluginResult::failure(format!("Dataset '{}' not found", dataset_name), 0),
        }
    }

    fn health_check(&self) -> PluginHealth {
        let loaded_datasets = self.datasets.len();
        let total_rows: usize = self.datasets.values().map(|d| d.rows.len()).sum();

        PluginHealth::healthy(
            format!("CSV data source healthy: {} datasets, {} total rows", loaded_datasets, total_rows),
            PluginMetrics::default(),
        )
    }

    fn get_capabilities(&self) -> PluginCapabilities {
        let mut allowed_paths = vec!["*.csv".to_string()];
        // Add configured CSV file paths
        for csv_config in &self.config.csv_files {
            allowed_paths.push(csv_config.path.clone());
        }

        PluginCapabilities {
            network: NetworkCapabilities {
                allow_http_outbound: false,
                allowed_hosts: vec![],
            },
            filesystem: FilesystemCapabilities {
                allow_read: true,
                allow_write: false,
                allowed_paths,
            },
            resources: PluginResources {
                max_memory_bytes: 25 * 1024 * 1024, // 25MB
                max_cpu_time_ms: 250, // 250ms per query
            },
            custom: HashMap::new(),
        }
    }
}

/// Plugin factory function
#[no_mangle]
pub extern "C" fn create_datasource_plugin(config_json: *const u8, config_len: usize) -> *mut CsvDataSourcePlugin {
    let config_bytes = unsafe {
        std::slice::from_raw_parts(config_json, config_len)
    };

    let config_str = match std::str::from_utf8(config_bytes) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let config: CsvDataSourceConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    let plugin = Box::new(CsvDataSourcePlugin::new(config));
    Box::into_raw(plugin)
}

/// Plugin cleanup function
#[no_mangle]
pub extern "C" fn destroy_datasource_plugin(plugin: *mut CsvDataSourcePlugin) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_csv_dataset_loading() {
        // Create a temporary CSV file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "name,age,city").unwrap();
        writeln!(temp_file, "Alice,25,New York").unwrap();
        writeln!(temp_file, "Bob,30,London").unwrap();
        writeln!(temp_file, "Charlie,35,Tokyo").unwrap();

        let csv_config = CsvFileConfig {
            name: "test_users".to_string(),
            path: temp_file.path().to_string_lossy().to_string(),
            has_headers: true,
        };

        let config = CsvDataSourceConfig {
            csv_files: vec![csv_config],
            cache_enabled: false,
            max_rows_per_query: 100,
        };

        let plugin = CsvDataSourcePlugin::new(config);

        // Check that dataset was loaded
        assert!(plugin.datasets.contains_key("test_users"));

        let dataset = plugin.datasets.get("test_users").unwrap();
        assert_eq!(dataset.rows.len(), 3);
        assert_eq!(dataset.headers, vec!["name", "age", "city"]);

        // Test querying
        let query = DataSourceQuery {
            filters: vec![],
            sort: None,
            limit: Some(2),
            offset: Some(0),
        };

        let result = plugin.query_dataset("test_users", &query);
        assert!(result.is_ok());

        let data_set = result.unwrap();
        assert_eq!(data_set.rows.len(), 2);
        assert_eq!(data_set.total_count, 3);
    }

    #[test]
    fn test_data_filtering() {
        let config = CsvDataSourceConfig::default();
        let plugin = CsvDataSourcePlugin::new(config);

        // Create test data
        let mut row = HashMap::new();
        row.insert("age".to_string(), "25".to_string());
        row.insert("city".to_string(), "New York".to_string());

        // Test equality filter
        let filter = DataFilter {
            field: "city".to_string(),
            operator: FilterOperator::Eq,
            value: serde_json::json!("New York"),
        };
        assert!(plugin.matches_filter(&row, &filter));

        // Test inequality filter
        let filter = DataFilter {
            field: "city".to_string(),
            operator: FilterOperator::Eq,
            value: serde_json::json!("London"),
        };
        assert!(!plugin.matches_filter(&row, &filter));

        // Test numeric comparison
        let filter = DataFilter {
            field: "age".to_string(),
            operator: FilterOperator::Gt,
            value: serde_json::json!(20),
        };
        assert!(plugin.matches_filter(&row, &filter));
    }

    #[test]
    fn test_capabilities() {
        let config = CsvDataSourceConfig::default();
        let plugin = CsvDataSourcePlugin::new(config);

        let capabilities = plugin.get_capabilities();
        assert!(capabilities.filesystem.allow_read);
        assert!(!capabilities.filesystem.allow_write);
        assert!(capabilities.filesystem.allowed_paths.contains(&"*.csv".to_string()));
    }
}
