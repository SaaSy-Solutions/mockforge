//! Dataset management and persistence
//!
//! This module has been refactored into sub-modules for better organization:
//! - core: Core dataset structures and basic operations
//! - collection: Dataset collection management and organization
//! - metadata: Dataset metadata tracking and management
//! - validation: Dataset validation and integrity checking
//! - persistence: Dataset storage, loading, and file operations

// Re-export sub-modules for backward compatibility
pub mod core;

// Re-export commonly used types
pub use core::*;

// Legacy imports for compatibility
use crate::{DataConfig, GenerationResult, OutputFormat, SchemaDefinition};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

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
    /// Create new metadata
    pub fn new(
        name: String,
        schema_name: String,
        result: &GenerationResult,
        config: DataConfig,
    ) -> Self {
        Self {
            name,
            description: None,
            schema_name,
            row_count: result.count,
            config,
            created_at: chrono::Utc::now(),
            generation_time_ms: result.generation_time_ms,
            format: OutputFormat::Json,
            file_size_bytes: None,
            tags: HashMap::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Set file size
    pub fn with_file_size(mut self, size: u64) -> Self {
        self.file_size_bytes = Some(size);
        self
    }
}

/// Dataset representation
#[derive(Debug)]
pub struct Dataset {
    /// Dataset metadata
    pub metadata: DatasetMetadata,
    /// Dataset data
    pub data: Vec<serde_json::Value>,
}

impl Dataset {
    /// Create a new dataset from generation result
    pub fn new(metadata: DatasetMetadata, data: Vec<serde_json::Value>) -> Self {
        Self { metadata, data }
    }

    /// Create dataset from generation result
    pub fn from_generation_result(
        name: String,
        schema_name: String,
        result: GenerationResult,
        config: DataConfig,
    ) -> Self {
        let metadata = DatasetMetadata::new(name, schema_name, &result, config);
        Self::new(metadata, result.data)
    }

    /// Get dataset as JSON string
    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.data)
            .map_err(|e| Error::generic(format!("Failed to serialize dataset: {}", e)))
    }

    /// Get dataset as JSON Lines string
    pub fn to_jsonl_string(&self) -> Result<String> {
        let lines: Result<Vec<String>> = self
            .data
            .iter()
            .map(|value| {
                serde_json::to_string(value)
                    .map_err(|e| Error::generic(format!("JSON serialization error: {}", e)))
            })
            .collect();

        lines.map(|lines| lines.join("\n"))
    }

    /// Get dataset as CSV string (basic implementation)
    pub fn to_csv_string(&self) -> Result<String> {
        if self.data.is_empty() {
            return Ok(String::new());
        }

        let mut csv_output = String::new();

        // Extract headers from first object
        if let Some(first_row) = self.data.first() {
            if let Some(obj) = first_row.as_object() {
                let headers: Vec<String> = obj.keys().cloned().collect();
                csv_output.push_str(&headers.join(","));
                csv_output.push('\n');

                // Add data rows
                for row in &self.data {
                    if let Some(obj) = row.as_object() {
                        let values: Vec<String> = headers
                            .iter()
                            .map(|header| {
                                obj.get(header)
                                    .map(|v| v.to_string().trim_matches('"').to_string())
                                    .unwrap_or_default()
                            })
                            .collect();
                        csv_output.push_str(&values.join(","));
                        csv_output.push('\n');
                    }
                }
            }
        }

        Ok(csv_output)
    }

    /// Get dataset as YAML string
    pub fn to_yaml_string(&self) -> Result<String> {
        serde_yaml::to_string(&self.data)
            .map_err(|e| Error::generic(format!("Failed to serialize dataset: {}", e)))
    }

    /// Save dataset to file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = match self.metadata.format {
            OutputFormat::Json => self.to_json_string()?,
            OutputFormat::JsonLines => self.to_jsonl_string()?,
            OutputFormat::Csv => self.to_csv_string()?,
            OutputFormat::Yaml => self.to_yaml_string()?,
        };

        fs::write(path, content)
            .await
            .map_err(|e| Error::generic(format!("Failed to write dataset file: {}", e)))
    }

    /// Load dataset from file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| Error::generic(format!("Failed to read dataset file: {}", e)))?;

        // Try to parse as JSON array first
        if let Ok(data) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
            let metadata = DatasetMetadata {
                name: "loaded_dataset".to_string(),
                description: None,
                schema_name: "unknown".to_string(),
                row_count: data.len(),
                config: DataConfig::default(),
                created_at: chrono::Utc::now(),
                generation_time_ms: 0,
                format: OutputFormat::Json,
                file_size_bytes: Some(content.len() as u64),
                tags: HashMap::new(),
            };

            return Ok(Self::new(metadata, data));
        }

        Err(Error::generic("Unsupported file format or invalid content"))
    }

    /// Get row count
    pub fn row_count(&self) -> usize {
        self.data.len()
    }

    /// Get sample rows
    pub fn sample(&self, count: usize) -> &[serde_json::Value] {
        let sample_count = count.min(self.data.len());
        &self.data[..sample_count]
    }

    /// Filter dataset by predicate
    pub fn filter<F>(&self, predicate: F) -> Dataset
    where
        F: Fn(&serde_json::Value) -> bool,
    {
        let filtered_data: Vec<serde_json::Value> =
            self.data.iter().filter(|row| predicate(row)).cloned().collect();

        let mut metadata = self.metadata.clone();
        metadata.row_count = filtered_data.len();

        Self::new(metadata, filtered_data)
    }

    /// Transform dataset with a mapping function
    pub fn map<F>(&self, mapper: F) -> Dataset
    where
        F: Fn(&serde_json::Value) -> serde_json::Value,
    {
        let mapped_data: Vec<serde_json::Value> = self.data.iter().map(mapper).collect();

        let metadata = self.metadata.clone();
        Self::new(metadata, mapped_data)
    }

    /// Validate this dataset against a schema
    pub fn validate_against_schema(&self, schema: &SchemaDefinition) -> Result<Vec<String>> {
        utils::validate_dataset_against_schema(self, schema)
    }

    /// Validate this dataset with detailed results
    pub fn validate_with_details(&self, schema: &SchemaDefinition) -> DatasetValidationResult {
        utils::validate_dataset_with_details(self, schema)
    }
}

/// Dataset collection for managing multiple datasets
#[derive(Debug)]
pub struct DatasetCollection {
    /// Datasets indexed by name
    datasets: HashMap<String, Dataset>,
    /// Collection metadata
    #[allow(dead_code)]
    metadata: HashMap<String, String>,
}

impl DatasetCollection {
    /// Create a new dataset collection
    pub fn new() -> Self {
        Self {
            datasets: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a dataset to the collection
    pub fn add_dataset(&mut self, dataset: Dataset) -> Result<()> {
        let name = dataset.metadata.name.clone();
        self.datasets.insert(name, dataset);
        Ok(())
    }

    /// Get a dataset by name
    pub fn get_dataset(&self, name: &str) -> Option<&Dataset> {
        self.datasets.get(name)
    }

    /// Remove a dataset
    pub fn remove_dataset(&mut self, name: &str) -> Option<Dataset> {
        self.datasets.remove(name)
    }

    /// List all dataset names
    pub fn list_datasets(&self) -> Vec<String> {
        self.datasets.keys().cloned().collect()
    }

    /// Get collection size
    pub fn size(&self) -> usize {
        self.datasets.len()
    }

    /// Save entire collection to directory
    pub async fn save_to_directory<P: AsRef<Path>>(&self, dir_path: P) -> Result<()> {
        fs::create_dir_all(&dir_path)
            .await
            .map_err(|e| Error::generic(format!("Failed to create directory: {}", e)))?;

        for (name, dataset) in &self.datasets {
            let file_path = dir_path.as_ref().join(format!("{}.json", name));
            dataset.save_to_file(file_path).await?;
        }

        Ok(())
    }

    /// Load collection from directory
    pub async fn load_from_directory<P: AsRef<Path>>(dir_path: P) -> Result<Self> {
        let mut collection = Self::new();
        let mut entries = fs::read_dir(dir_path)
            .await
            .map_err(|e| Error::generic(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(_file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    let dataset = Dataset::load_from_file(&path).await?;
                    collection.add_dataset(dataset)?;
                }
            }
        }

        Ok(collection)
    }

    /// Get collection statistics
    pub fn statistics(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();

        stats.insert("total_datasets".to_string(), self.size().into());
        stats.insert(
            "total_rows".to_string(),
            self.datasets.values().map(|d| d.row_count()).sum::<usize>().into(),
        );

        let dataset_info: Vec<serde_json::Value> = self
            .datasets
            .values()
            .map(|d| {
                serde_json::json!({
                    "name": d.metadata.name,
                    "schema": d.metadata.schema_name,
                    "rows": d.row_count(),
                    "format": format!("{:?}", d.metadata.format),
                })
            })
            .collect();

        stats.insert("datasets".to_string(), dataset_info.into());

        stats
    }
}

impl Default for DatasetCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// Dataset utilities
pub mod utils {
    use super::*;

    /// Create a sample dataset collection with common schemas
    pub async fn create_sample_collection() -> Result<DatasetCollection> {
        let mut collection = DatasetCollection::new();

        // Create user dataset
        let users_result = crate::generator::utils::generate_users(50).await?;
        let users_dataset = Dataset::from_generation_result(
            "users".to_string(),
            "User".to_string(),
            users_result,
            DataConfig {
                rows: 50,
                ..Default::default()
            },
        );
        collection.add_dataset(users_dataset)?;

        // Create product dataset
        let products_result = crate::generator::utils::generate_products(25).await?;
        let products_dataset = Dataset::from_generation_result(
            "products".to_string(),
            "Product".to_string(),
            products_result,
            DataConfig {
                rows: 25,
                ..Default::default()
            },
        );
        collection.add_dataset(products_dataset)?;

        Ok(collection)
    }

    /// Export dataset to different formats
    pub async fn export_dataset(
        dataset: &Dataset,
        format: OutputFormat,
        output_path: &Path,
    ) -> Result<()> {
        let content = match format {
            OutputFormat::Json => dataset.to_json_string()?,
            OutputFormat::JsonLines => dataset.to_jsonl_string()?,
            OutputFormat::Csv => dataset.to_csv_string()?,
            OutputFormat::Yaml => dataset.to_yaml_string()?,
        };

        fs::write(output_path, content)
            .await
            .map_err(|e| Error::generic(format!("Failed to export dataset: {}", e)))
    }

    /// Validate dataset against schema
    pub fn validate_dataset_against_schema(
        dataset: &Dataset,
        schema: &SchemaDefinition,
    ) -> Result<Vec<String>> {
        let mut errors = Vec::new();

        // Validate each row in the dataset
        for (row_index, row) in dataset.data.iter().enumerate() {
            match row {
                serde_json::Value::Object(row_obj) => {
                    // Validate each field in the schema
                    for field in &schema.fields {
                        let field_name = &field.name;

                        if let Some(field_value) = row_obj.get(field_name) {
                            // Validate the field value
                            if let Err(validation_error) = field.validate_value(field_value) {
                                errors.push(format!(
                                    "Row {}: Field '{}': {}",
                                    row_index + 1,
                                    field_name,
                                    validation_error
                                ));
                            }
                        } else if field.required {
                            errors.push(format!(
                                "Row {}: Required field '{}' is missing",
                                row_index + 1,
                                field_name
                            ));
                        }
                    }

                    // Check for unexpected fields
                    for (key, _) in row_obj {
                        let field_exists_in_schema = schema.fields.iter().any(|f| f.name == *key);
                        if !field_exists_in_schema {
                            errors.push(format!(
                                "Row {}: Unexpected field '{}' not defined in schema",
                                row_index + 1,
                                key
                            ));
                        }
                    }
                }
                _ => {
                    errors.push(format!("Row {}: Expected object, got {}", row_index + 1, row));
                }
            }
        }

        // Validate dataset-level constraints
        if let Err(count_error) = validate_dataset_size(dataset, schema) {
            errors.push(count_error.to_string());
        }

        Ok(errors)
    }

    /// Validate dataset size constraints
    fn validate_dataset_size(dataset: &Dataset, schema: &SchemaDefinition) -> Result<()> {
        // Check if there are any size constraints in schema metadata
        if let Some(min_rows) = schema.metadata.get("min_rows") {
            if let Some(min_count) = min_rows.as_u64() {
                if dataset.data.len() < min_count as usize {
                    return Err(Error::validation(format!(
                        "Dataset has {} rows, but schema requires at least {} rows",
                        dataset.data.len(),
                        min_count
                    )));
                }
            }
        }

        if let Some(max_rows) = schema.metadata.get("max_rows") {
            if let Some(max_count) = max_rows.as_u64() {
                if dataset.data.len() > max_count as usize {
                    return Err(Error::validation(format!(
                        "Dataset has {} rows, but schema allows at most {} rows",
                        dataset.data.len(),
                        max_count
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate dataset and return detailed result
    pub fn validate_dataset_with_details(
        dataset: &Dataset,
        schema: &SchemaDefinition,
    ) -> DatasetValidationResult {
        let errors = validate_dataset_against_schema(dataset, schema);

        match errors {
            Ok(validation_errors) => {
                let warnings = Vec::new(); // Could add warnings for deprecated fields, etc.
                DatasetValidationResult {
                    valid: validation_errors.is_empty(),
                    errors: validation_errors,
                    warnings,
                    total_rows_validated: dataset.data.len(),
                }
            }
            Err(e) => DatasetValidationResult {
                valid: false,
                errors: vec![format!("Validation failed: {}", e)],
                warnings: Vec::new(),
                total_rows_validated: dataset.data.len(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // DatasetValidationResult tests
    // =========================================================================

    #[test]
    fn test_dataset_validation_result_creation() {
        let result = DatasetValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
            total_rows_validated: 100,
        };

        assert!(result.valid);
        assert_eq!(result.total_rows_validated, 100);
    }

    #[test]
    fn test_dataset_validation_result_with_errors() {
        let result = DatasetValidationResult {
            valid: false,
            errors: vec!["Error 1".to_string(), "Error 2".to_string()],
            warnings: vec![],
            total_rows_validated: 50,
        };

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_dataset_validation_result_with_warnings() {
        let result = DatasetValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec!["Warning 1".to_string()],
            total_rows_validated: 75,
        };

        assert!(result.valid);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_dataset_validation_result_clone() {
        let result = DatasetValidationResult {
            valid: true,
            errors: vec!["err".to_string()],
            warnings: vec!["warn".to_string()],
            total_rows_validated: 50,
        };
        let cloned = result.clone();
        assert_eq!(cloned.total_rows_validated, 50);
        assert_eq!(cloned.errors.len(), 1);
    }

    #[test]
    fn test_dataset_validation_result_serialize() {
        let result = DatasetValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
            total_rows_validated: 25,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("true"));
        assert!(json.contains("25"));
    }

    #[test]
    fn test_dataset_validation_result_deserialize() {
        let json =
            r#"{"valid": false, "errors": ["e1"], "warnings": [], "total_rows_validated": 10}"#;
        let result: DatasetValidationResult = serde_json::from_str(json).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_dataset_validation_result_debug() {
        let result = DatasetValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
            total_rows_validated: 0,
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("valid"));
    }

    // =========================================================================
    // DatasetMetadata tests
    // =========================================================================

    #[test]
    fn test_dataset_metadata_creation() {
        let config = DataConfig::default();
        let metadata = DatasetMetadata {
            name: "TestDataset".to_string(),
            description: Some("Test description".to_string()),
            schema_name: "TestSchema".to_string(),
            row_count: 100,
            config,
            created_at: chrono::Utc::now(),
            generation_time_ms: 1000,
            format: OutputFormat::Json,
            file_size_bytes: Some(1024),
            tags: HashMap::new(),
        };

        assert_eq!(metadata.name, "TestDataset");
        assert_eq!(metadata.row_count, 100);
        assert!(metadata.description.is_some());
        assert_eq!(metadata.generation_time_ms, 1000);
    }

    #[test]
    fn test_dataset_metadata_default() {
        let metadata = DatasetMetadata::default();
        assert!(metadata.name.is_empty());
        assert!(metadata.description.is_none());
        assert_eq!(metadata.row_count, 0);
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn test_dataset_metadata_new() {
        let result = GenerationResult {
            data: vec![serde_json::json!({"id": 1}), serde_json::json!({"id": 2})],
            count: 2,
            generation_time_ms: 100,
            warnings: vec![],
        };
        let config = DataConfig::default();
        let metadata = DatasetMetadata::new(
            "my_dataset".to_string(),
            "TestSchema".to_string(),
            &result,
            config,
        );

        assert_eq!(metadata.name, "my_dataset");
        assert_eq!(metadata.schema_name, "TestSchema");
        assert_eq!(metadata.row_count, 2);
        assert_eq!(metadata.generation_time_ms, 100);
    }

    #[test]
    fn test_dataset_metadata_with_description() {
        let metadata = DatasetMetadata::default().with_description("A test dataset".to_string());
        assert_eq!(metadata.description, Some("A test dataset".to_string()));
    }

    #[test]
    fn test_dataset_metadata_with_tag() {
        let metadata = DatasetMetadata::default()
            .with_tag("env".to_string(), "test".to_string())
            .with_tag("version".to_string(), "1.0".to_string());
        assert_eq!(metadata.tags.get("env"), Some(&"test".to_string()));
        assert_eq!(metadata.tags.get("version"), Some(&"1.0".to_string()));
    }

    #[test]
    fn test_dataset_metadata_with_file_size() {
        let metadata = DatasetMetadata::default().with_file_size(2048);
        assert_eq!(metadata.file_size_bytes, Some(2048));
    }

    #[test]
    fn test_dataset_metadata_clone() {
        let metadata = DatasetMetadata {
            name: "cloneable".to_string(),
            ..Default::default()
        };
        let cloned = metadata.clone();
        assert_eq!(cloned.name, "cloneable");
    }

    #[test]
    fn test_dataset_metadata_serialize() {
        let metadata = DatasetMetadata::default();
        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("name"));
        assert!(json.contains("row_count"));
    }

    // =========================================================================
    // Dataset tests
    // =========================================================================

    #[test]
    fn test_dataset_new() {
        let metadata = DatasetMetadata::default();
        let data = vec![
            serde_json::json!({"id": 1, "name": "Alice"}),
            serde_json::json!({"id": 2, "name": "Bob"}),
        ];
        let dataset = Dataset::new(metadata, data);
        assert_eq!(dataset.row_count(), 2);
    }

    #[test]
    fn test_dataset_from_generation_result() {
        let result = GenerationResult {
            data: vec![serde_json::json!({"id": 1})],
            count: 1,
            generation_time_ms: 50,
            warnings: vec![],
        };
        let config = DataConfig::default();
        let dataset = Dataset::from_generation_result(
            "test_dataset".to_string(),
            "TestSchema".to_string(),
            result,
            config,
        );
        assert_eq!(dataset.metadata.name, "test_dataset");
        assert_eq!(dataset.row_count(), 1);
    }

    #[test]
    fn test_dataset_to_json_string() {
        let metadata = DatasetMetadata::default();
        let data = vec![serde_json::json!({"id": 1}), serde_json::json!({"id": 2})];
        let dataset = Dataset::new(metadata, data);
        let json = dataset.to_json_string().unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("1"));
        assert!(json.contains("2"));
    }

    #[test]
    fn test_dataset_to_jsonl_string() {
        let metadata = DatasetMetadata::default();
        let data = vec![serde_json::json!({"id": 1}), serde_json::json!({"id": 2})];
        let dataset = Dataset::new(metadata, data);
        let jsonl = dataset.to_jsonl_string().unwrap();
        let lines: Vec<&str> = jsonl.split('\n').collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_dataset_to_csv_string() {
        let metadata = DatasetMetadata::default();
        let data = vec![
            serde_json::json!({"id": 1, "name": "Alice"}),
            serde_json::json!({"id": 2, "name": "Bob"}),
        ];
        let dataset = Dataset::new(metadata, data);
        let csv = dataset.to_csv_string().unwrap();
        assert!(csv.contains("id") || csv.contains("name")); // Headers
        assert!(csv.contains("Alice") || csv.contains("Bob")); // Data
    }

    #[test]
    fn test_dataset_to_csv_string_empty() {
        let metadata = DatasetMetadata::default();
        let dataset = Dataset::new(metadata, vec![]);
        let csv = dataset.to_csv_string().unwrap();
        assert!(csv.is_empty());
    }

    #[test]
    fn test_dataset_to_yaml_string() {
        let metadata = DatasetMetadata::default();
        let data = vec![serde_json::json!({"id": 1})];
        let dataset = Dataset::new(metadata, data);
        let yaml = dataset.to_yaml_string().unwrap();
        assert!(yaml.contains("id"));
    }

    #[test]
    fn test_dataset_row_count() {
        let metadata = DatasetMetadata::default();
        let data = vec![
            serde_json::json!({}),
            serde_json::json!({}),
            serde_json::json!({}),
        ];
        let dataset = Dataset::new(metadata, data);
        assert_eq!(dataset.row_count(), 3);
    }

    #[test]
    fn test_dataset_sample() {
        let metadata = DatasetMetadata::default();
        let data: Vec<serde_json::Value> = (0..10).map(|i| serde_json::json!({"id": i})).collect();
        let dataset = Dataset::new(metadata, data);

        let sample = dataset.sample(3);
        assert_eq!(sample.len(), 3);

        let big_sample = dataset.sample(100);
        assert_eq!(big_sample.len(), 10); // Capped at dataset size
    }

    #[test]
    fn test_dataset_filter() {
        let metadata = DatasetMetadata {
            name: "filterable".to_string(),
            ..Default::default()
        };
        let data = vec![
            serde_json::json!({"id": 1, "active": true}),
            serde_json::json!({"id": 2, "active": false}),
            serde_json::json!({"id": 3, "active": true}),
        ];
        let dataset = Dataset::new(metadata, data);

        let filtered =
            dataset.filter(|row| row.get("active").and_then(|v| v.as_bool()).unwrap_or(false));

        assert_eq!(filtered.row_count(), 2);
        assert_eq!(filtered.metadata.row_count, 2);
    }

    #[test]
    fn test_dataset_map() {
        let metadata = DatasetMetadata::default();
        let data = vec![
            serde_json::json!({"value": 1}),
            serde_json::json!({"value": 2}),
        ];
        let dataset = Dataset::new(metadata, data);

        let mapped = dataset.map(|row| {
            let mut new_row = row.clone();
            if let Some(obj) = new_row.as_object_mut() {
                obj.insert("doubled".to_string(), serde_json::json!(true));
            }
            new_row
        });

        assert_eq!(mapped.row_count(), 2);
        assert!(mapped.data[0].get("doubled").is_some());
    }

    #[test]
    fn test_dataset_debug() {
        let metadata = DatasetMetadata {
            name: "debug_test".to_string(),
            ..Default::default()
        };
        let dataset = Dataset::new(metadata, vec![]);
        let debug_str = format!("{:?}", dataset);
        assert!(debug_str.contains("metadata"));
    }

    // =========================================================================
    // DatasetCollection tests
    // =========================================================================

    #[test]
    fn test_dataset_collection_new() {
        let collection = DatasetCollection::new();
        assert_eq!(collection.size(), 0);
    }

    #[test]
    fn test_dataset_collection_default() {
        let collection = DatasetCollection::default();
        assert_eq!(collection.size(), 0);
    }

    #[test]
    fn test_dataset_collection_add_dataset() {
        let mut collection = DatasetCollection::new();
        let dataset = Dataset::new(
            DatasetMetadata {
                name: "test1".to_string(),
                ..Default::default()
            },
            vec![],
        );
        collection.add_dataset(dataset).unwrap();
        assert_eq!(collection.size(), 1);
    }

    #[test]
    fn test_dataset_collection_get_dataset() {
        let mut collection = DatasetCollection::new();
        let dataset = Dataset::new(
            DatasetMetadata {
                name: "findme".to_string(),
                ..Default::default()
            },
            vec![serde_json::json!({"id": 1})],
        );
        collection.add_dataset(dataset).unwrap();

        let found = collection.get_dataset("findme");
        assert!(found.is_some());
        assert_eq!(found.unwrap().row_count(), 1);
    }

    #[test]
    fn test_dataset_collection_get_dataset_not_found() {
        let collection = DatasetCollection::new();
        assert!(collection.get_dataset("nonexistent").is_none());
    }

    #[test]
    fn test_dataset_collection_remove_dataset() {
        let mut collection = DatasetCollection::new();
        let dataset = Dataset::new(
            DatasetMetadata {
                name: "removable".to_string(),
                ..Default::default()
            },
            vec![],
        );
        collection.add_dataset(dataset).unwrap();

        let removed = collection.remove_dataset("removable");
        assert!(removed.is_some());
        assert_eq!(collection.size(), 0);
    }

    #[test]
    fn test_dataset_collection_list_datasets() {
        let mut collection = DatasetCollection::new();
        collection
            .add_dataset(Dataset::new(
                DatasetMetadata {
                    name: "a".to_string(),
                    ..Default::default()
                },
                vec![],
            ))
            .unwrap();
        collection
            .add_dataset(Dataset::new(
                DatasetMetadata {
                    name: "b".to_string(),
                    ..Default::default()
                },
                vec![],
            ))
            .unwrap();

        let names = collection.list_datasets();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
    }

    #[test]
    fn test_dataset_collection_size() {
        let mut collection = DatasetCollection::new();
        assert_eq!(collection.size(), 0);

        collection
            .add_dataset(Dataset::new(
                DatasetMetadata {
                    name: "x".to_string(),
                    ..Default::default()
                },
                vec![],
            ))
            .unwrap();
        assert_eq!(collection.size(), 1);
    }

    #[test]
    fn test_dataset_collection_statistics() {
        let mut collection = DatasetCollection::new();
        collection
            .add_dataset(Dataset::new(
                DatasetMetadata {
                    name: "ds1".to_string(),
                    schema_name: "Schema1".to_string(),
                    ..Default::default()
                },
                vec![serde_json::json!({}), serde_json::json!({})],
            ))
            .unwrap();
        collection
            .add_dataset(Dataset::new(
                DatasetMetadata {
                    name: "ds2".to_string(),
                    schema_name: "Schema2".to_string(),
                    ..Default::default()
                },
                vec![serde_json::json!({})],
            ))
            .unwrap();

        let stats = collection.statistics();
        assert_eq!(stats.get("total_datasets").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(stats.get("total_rows").and_then(|v| v.as_u64()), Some(3));
    }

    #[test]
    fn test_dataset_collection_debug() {
        let collection = DatasetCollection::new();
        let debug_str = format!("{:?}", collection);
        assert!(debug_str.contains("datasets"));
    }
}
