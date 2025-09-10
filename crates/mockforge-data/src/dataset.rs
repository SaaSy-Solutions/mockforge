//! Dataset management and persistence

use crate::{DataConfig, GenerationResult, OutputFormat};
use mockforge_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

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

impl DatasetMetadata {
    /// Create new metadata
    pub fn new(name: String, schema_name: String, result: &GenerationResult, config: DataConfig) -> Self {
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
            .map_err(|e| mockforge_core::Error::generic(format!("Failed to serialize dataset: {}", e)))
    }

    /// Get dataset as JSON Lines string
    pub fn to_jsonl_string(&self) -> Result<String> {
        let lines: Result<Vec<String>> = self.data
            .iter()
            .map(|value| serde_json::to_string(value).map_err(|e| mockforge_core::Error::generic(format!("JSON serialization error: {}", e))))
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
            .map_err(|e| mockforge_core::Error::generic(format!("Failed to serialize dataset: {}", e)))
    }

    /// Save dataset to file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = match self.metadata.format {
            OutputFormat::Json => self.to_json_string()?,
            OutputFormat::JsonLines => self.to_jsonl_string()?,
            OutputFormat::Csv => self.to_csv_string()?,
            OutputFormat::Yaml => self.to_yaml_string()?,
        };

        fs::write(path, content).await
            .map_err(|e| mockforge_core::Error::generic(format!("Failed to write dataset file: {}", e)))
    }

    /// Load dataset from file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).await
            .map_err(|e| mockforge_core::Error::generic(format!("Failed to read dataset file: {}", e)))?;

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

        Err(mockforge_core::Error::generic("Unsupported file format or invalid content"))
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
        let filtered_data: Vec<serde_json::Value> = self.data
            .iter()
            .filter(|row| predicate(row))
            .cloned()
            .collect();

        let mut metadata = self.metadata.clone();
        metadata.row_count = filtered_data.len();

        Self::new(metadata, filtered_data)
    }

    /// Transform dataset with a mapping function
    pub fn map<F>(&self, mapper: F) -> Dataset
    where
        F: Fn(&serde_json::Value) -> serde_json::Value,
    {
        let mapped_data: Vec<serde_json::Value> = self.data
            .iter()
            .map(|row| mapper(row))
            .collect();

        let metadata = self.metadata.clone();
        Self::new(metadata, mapped_data)
    }
}

/// Dataset collection for managing multiple datasets
#[derive(Debug)]
pub struct DatasetCollection {
    /// Datasets indexed by name
    datasets: HashMap<String, Dataset>,
    /// Collection metadata
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
        fs::create_dir_all(&dir_path).await
            .map_err(|e| mockforge_core::Error::generic(format!("Failed to create directory: {}", e)))?;

        for (name, dataset) in &self.datasets {
            let file_path = dir_path.as_ref().join(format!("{}.json", name));
            dataset.save_to_file(file_path).await?;
        }

        Ok(())
    }

    /// Load collection from directory
    pub async fn load_from_directory<P: AsRef<Path>>(dir_path: P) -> Result<Self> {
        let mut collection = Self::new();
        let mut entries = fs::read_dir(dir_path).await
            .map_err(|e| mockforge_core::Error::generic(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| mockforge_core::Error::generic(format!("Failed to read directory entry: {}", e)))? {
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
        stats.insert("total_rows".to_string(),
            self.datasets.values().map(|d| d.row_count()).sum::<usize>().into());

        let dataset_info: Vec<serde_json::Value> = self.datasets.values()
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

        fs::write(output_path, content).await
            .map_err(|e| mockforge_core::Error::generic(format!("Failed to export dataset: {}", e)))
    }

    /// Validate dataset against schema (placeholder)
    pub fn validate_dataset_against_schema(
        _dataset: &Dataset,
        _schema: &crate::schema::SchemaDefinition,
    ) -> Result<Vec<String>> {
        // TODO: Implement schema validation for datasets
        tracing::warn!("Dataset schema validation not yet implemented");
        Ok(Vec::new())
    }
}
