//! Core dataset structures and basic operations
//!
//! This module provides the fundamental data structures for datasets,
//! including dataset definitions, rows, and basic operations.

use crate::{DataConfig, OutputFormat};
use mockforge_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dataset validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetValidationResult {
    /// Whether the dataset is valid
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Total number of rows validated
    pub total_rows_validated: usize,
}

/// Dataset metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    /// Dataset name
    pub name: String,
    /// Dataset description
    pub description: Option<String>,
    /// Schema name used to generate this dataset
    pub schema_name: String,
    /// Number of rows
    pub row_count: usize,
    /// Generation configuration
    pub config: DataConfig,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Generation time in milliseconds
    pub generation_time_ms: u128,
    /// File format
    pub format: OutputFormat,
    /// File size in bytes
    pub file_size_bytes: Option<u64>,
    /// Additional metadata
    pub tags: HashMap<String, String>,
}

impl Default for DatasetMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            schema_name: String::new(),
            row_count: 0,
            config: DataConfig::default(),
            created_at: chrono::Utc::now(),
            generation_time_ms: 0,
            format: OutputFormat::Json,
            file_size_bytes: None,
            tags: HashMap::new(),
        }
    }
}

impl DatasetMetadata {
    /// Create new dataset metadata
    pub fn new(
        name: String,
        schema_name: String,
        config: DataConfig,
        format: OutputFormat,
    ) -> Self {
        Self {
            name,
            schema_name,
            config,
            format,
            created_at: chrono::Utc::now(),
            ..Default::default()
        }
    }

    /// Update generation time
    pub fn set_generation_time(&mut self, time_ms: u128) {
        self.generation_time_ms = time_ms;
    }

    /// Set file size
    pub fn set_file_size(&mut self, size_bytes: u64) {
        self.file_size_bytes = Some(size_bytes);
    }

    /// Add tag
    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }

    /// Get tag value
    pub fn get_tag(&self, key: &str) -> Option<&String> {
        self.tags.get(key)
    }

    /// Remove tag
    pub fn remove_tag(&mut self, key: &str) -> Option<String> {
        self.tags.remove(key)
    }

    /// Get total size in bytes (estimated)
    pub fn estimated_size_bytes(&self) -> u64 {
        self.file_size_bytes.unwrap_or_else(|| {
            // Rough estimate: each row ~1KB
            (self.row_count * 1024) as u64
        })
    }

    /// Check if dataset is empty
    pub fn is_empty(&self) -> bool {
        self.row_count == 0
    }

    /// Get human-readable size
    pub fn human_readable_size(&self) -> String {
        let bytes = self.estimated_size_bytes();
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

/// Single row of data in a dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRow {
    /// Row ID
    pub id: String,
    /// Row data as key-value pairs
    pub data: HashMap<String, serde_json::Value>,
    /// Row metadata
    pub metadata: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl DatasetRow {
    /// Create a new dataset row
    pub fn new(id: String, data: HashMap<String, serde_json::Value>) -> Self {
        Self {
            id,
            data,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Add metadata to the row
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Remove metadata
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        self.metadata.remove(key)
    }

    /// Get field value
    pub fn get_field(&self, field_name: &str) -> Option<&serde_json::Value> {
        self.data.get(field_name)
    }

    /// Set field value
    pub fn set_field(&mut self, field_name: String, value: serde_json::Value) {
        self.data.insert(field_name, value);
    }

    /// Check if row contains a field
    pub fn has_field(&self, field_name: &str) -> bool {
        self.data.contains_key(field_name)
    }

    /// Get all field names
    pub fn field_names(&self) -> Vec<&String> {
        self.data.keys().collect()
    }

    /// Get row as JSON value
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "data": self.data,
            "metadata": self.metadata,
            "created_at": self.created_at,
        })
    }
}

/// Dataset statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    /// Total number of rows
    pub row_count: usize,
    /// Number of columns/fields
    pub column_count: usize,
    /// Total size in bytes
    pub total_size_bytes: u64,
    /// Average row size in bytes
    pub average_row_size_bytes: f64,
    /// Smallest row size in bytes
    pub min_row_size_bytes: u64,
    /// Largest row size in bytes
    pub max_row_size_bytes: u64,
    /// Field name statistics
    pub field_stats: HashMap<String, FieldStats>,
    /// Generation timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Field statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldStats {
    /// Field name
    pub field_name: String,
    /// Field type
    pub field_type: String,
    /// Number of non-null values
    pub non_null_count: usize,
    /// Number of null values
    pub null_count: usize,
    /// Number of unique values
    pub unique_count: usize,
    /// Minimum value (if numeric)
    pub min_value: Option<serde_json::Value>,
    /// Maximum value (if numeric)
    pub max_value: Option<serde_json::Value>,
    /// Average value (if numeric)
    pub average_value: Option<f64>,
    /// Most common values
    pub most_common_values: Vec<(serde_json::Value, usize)>,
}

/// Dataset represents a collection of generated data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    /// Dataset metadata
    pub metadata: DatasetMetadata,
    /// Dataset rows
    pub rows: Vec<DatasetRow>,
    /// Dataset statistics
    pub stats: Option<DatasetStats>,
}

impl Dataset {
    /// Create a new empty dataset
    pub fn new(
        name: String,
        schema_name: String,
        config: DataConfig,
        format: OutputFormat,
    ) -> Self {
        Self {
            metadata: DatasetMetadata::new(name, schema_name, config, format),
            rows: Vec::new(),
            stats: None,
        }
    }

    /// Create a dataset with pre-existing rows
    pub fn with_rows(
        name: String,
        schema_name: String,
        config: DataConfig,
        format: OutputFormat,
        rows: Vec<DatasetRow>,
    ) -> Self {
        let mut dataset = Self::new(name, schema_name, config, format);
        dataset.rows = rows;
        dataset.metadata.row_count = dataset.rows.len();
        dataset
    }

    /// Add a row to the dataset
    pub fn add_row(&mut self, row: DatasetRow) {
        self.rows.push(row);
        self.metadata.row_count = self.rows.len();
    }

    /// Add multiple rows to the dataset
    pub fn add_rows(&mut self, rows: Vec<DatasetRow>) {
        self.rows.extend(rows);
        self.metadata.row_count = self.rows.len();
    }

    /// Get row by ID
    pub fn get_row(&self, id: &str) -> Option<&DatasetRow> {
        self.rows.iter().find(|row| row.id == id)
    }

    /// Get row by ID (mutable)
    pub fn get_row_mut(&mut self, id: &str) -> Option<&mut DatasetRow> {
        self.rows.iter_mut().find(|row| row.id == id)
    }

    /// Remove row by ID
    pub fn remove_row(&mut self, id: &str) -> Option<DatasetRow> {
        if let Some(pos) = self.rows.iter().position(|row| row.id == id) {
            let row = self.rows.remove(pos);
            self.metadata.row_count = self.rows.len();
            Some(row)
        } else {
            None
        }
    }

    /// Get rows by metadata key-value
    pub fn get_rows_by_metadata(&self, key: &str, value: &str) -> Vec<&DatasetRow> {
        self.rows
            .iter()
            .filter(|row| row.get_metadata(key).map(|v| v == value).unwrap_or(false))
            .collect()
    }

    /// Get all row IDs
    pub fn row_ids(&self) -> Vec<&String> {
        self.rows.iter().map(|row| &row.id).collect()
    }

    /// Check if dataset is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Get dataset size
    pub fn size(&self) -> usize {
        self.rows.len()
    }

    /// Get field names from the first row (if available)
    pub fn field_names(&self) -> Vec<&String> {
        if let Some(first_row) = self.rows.first() {
            first_row.field_names()
        } else {
            Vec::new()
        }
    }

    /// Calculate dataset statistics
    pub fn calculate_stats(&mut self) -> Result<()> {
        if self.rows.is_empty() {
            self.stats = Some(DatasetStats {
                row_count: 0,
                column_count: 0,
                total_size_bytes: 0,
                average_row_size_bytes: 0.0,
                min_row_size_bytes: 0,
                max_row_size_bytes: 0,
                field_stats: HashMap::new(),
                generated_at: chrono::Utc::now(),
            });
            return Ok(());
        }

        let mut total_size = 0u64;
        let mut row_sizes = Vec::new();

        // Temporary structure for collecting field statistics
        #[derive(Default)]
        struct TempFieldStats {
            field_type: Option<String>,
            non_null_count: usize,
            null_count: usize,
            unique_values: std::collections::HashSet<serde_json::Value>,
            numeric_values: Vec<f64>,
            frequency: std::collections::HashMap<serde_json::Value, usize>,
        }

        let mut temp_field_stats: HashMap<String, TempFieldStats> = HashMap::new();

        // Get field names from first row
        let field_names = self.field_names();
        for field_name in &field_names {
            temp_field_stats.insert(field_name.to_string(), TempFieldStats::default());
        }

        // Process each row
        for row in &self.rows {
            let row_json = row.to_json();
            let row_size = serde_json::to_string(&row_json)
                .map_err(|e| Error::generic(format!("Failed to serialize row: {}", e)))?
                .len() as u64;

            total_size += row_size;
            row_sizes.push(row_size);

            // Update field statistics
            for (field_name, field_value) in &row.data {
                if let Some(temp_stats) = temp_field_stats.get_mut(field_name) {
                    match field_value {
                        serde_json::Value::Null => temp_stats.null_count += 1,
                        _ => {
                            temp_stats.non_null_count += 1;

                            // Type detection
                            let value_type = match field_value {
                                serde_json::Value::Bool(_) => "boolean",
                                serde_json::Value::Number(_) => "number",
                                serde_json::Value::String(_) => "string",
                                serde_json::Value::Array(_) => "array",
                                serde_json::Value::Object(_) => "object",
                                serde_json::Value::Null => unreachable!(),
                            };

                            if temp_stats.field_type.is_none() {
                                temp_stats.field_type = Some(value_type.to_string());
                            } else if temp_stats.field_type.as_ref()
                                != Some(&value_type.to_string())
                            {
                                temp_stats.field_type = Some("mixed".to_string());
                            }

                            // Collect unique values
                            temp_stats.unique_values.insert(field_value.clone());

                            // Collect numeric values for min/max/avg
                            if let serde_json::Value::Number(num) = field_value {
                                if let Some(f) = num.as_f64() {
                                    temp_stats.numeric_values.push(f);
                                }
                            }

                            // Update frequency
                            *temp_stats.frequency.entry(field_value.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        // Convert temporary stats to final FieldStats
        let mut field_stats: HashMap<String, FieldStats> = HashMap::new();
        for (field_name, temp_stats) in temp_field_stats {
            let field_type = temp_stats.field_type.unwrap_or_else(|| "unknown".to_string());

            let (min_value, max_value, average_value) = if field_type == "number"
                && !temp_stats.numeric_values.is_empty()
            {
                let min = temp_stats.numeric_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max =
                    temp_stats.numeric_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let sum: f64 = temp_stats.numeric_values.iter().sum();
                let avg = sum / temp_stats.numeric_values.len() as f64;
                (
                    Some(serde_json::Value::Number(
                        serde_json::Number::from_f64(min).unwrap_or(serde_json::Number::from(0)),
                    )),
                    Some(serde_json::Value::Number(
                        serde_json::Number::from_f64(max).unwrap_or(serde_json::Number::from(0)),
                    )),
                    Some(avg),
                )
            } else {
                (None, None, None)
            };

            // Get most common values (top 5)
            let mut most_common: Vec<(serde_json::Value, usize)> =
                temp_stats.frequency.into_iter().collect();
            most_common.sort_by(|a, b| b.1.cmp(&a.1));
            most_common.truncate(5);

            field_stats.insert(
                field_name.clone(),
                FieldStats {
                    field_name,
                    field_type,
                    non_null_count: temp_stats.non_null_count,
                    null_count: temp_stats.null_count,
                    unique_count: temp_stats.unique_values.len(),
                    min_value,
                    max_value,
                    average_value,
                    most_common_values: most_common,
                },
            );
        }

        let row_count = self.rows.len();
        let average_row_size = if row_count > 0 {
            total_size as f64 / row_count as f64
        } else {
            0.0
        };

        let min_row_size = row_sizes.iter().min().unwrap_or(&0);
        let max_row_size = row_sizes.iter().max().unwrap_or(&0);

        self.stats = Some(DatasetStats {
            row_count,
            column_count: field_names.len(),
            total_size_bytes: total_size,
            average_row_size_bytes: average_row_size,
            min_row_size_bytes: *min_row_size,
            max_row_size_bytes: *max_row_size,
            field_stats,
            generated_at: chrono::Utc::now(),
        });

        Ok(())
    }

    /// Validate dataset integrity
    pub fn validate(&self) -> DatasetValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check metadata
        if self.metadata.name.is_empty() {
            errors.push("Dataset name cannot be empty".to_string());
        }

        if self.metadata.schema_name.is_empty() {
            errors.push("Schema name cannot be empty".to_string());
        }

        // Check rows
        for (index, row) in self.rows.iter().enumerate() {
            if row.id.is_empty() {
                errors.push(format!("Row {} has empty ID", index));
            }

            if row.data.is_empty() {
                warnings.push(format!("Row {} has no data", index));
            }
        }

        DatasetValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
            total_rows_validated: self.rows.len(),
        }
    }

    /// Export dataset to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| Error::generic(format!("Failed to serialize dataset: {}", e)))
    }

    /// Export dataset rows to JSON array
    pub fn rows_to_json(&self) -> Result<String> {
        let rows_json: Vec<_> = self.rows.iter().map(|row| row.to_json()).collect();
        serde_json::to_string_pretty(&rows_json)
            .map_err(|e| Error::generic(format!("Failed to serialize dataset rows: {}", e)))
    }

    /// Get dataset summary
    pub fn summary(&self) -> String {
        format!(
            "Dataset '{}' - {} rows, {} columns, {}",
            self.metadata.name,
            self.rows.len(),
            self.field_names().len(),
            self.metadata.human_readable_size()
        )
    }
}

impl Default for Dataset {
    fn default() -> Self {
        Self::new(
            "Untitled Dataset".to_string(),
            "Unknown Schema".to_string(),
            DataConfig::default(),
            OutputFormat::Json,
        )
    }
}
