//! Data-driven testing support for load testing
//!
//! This module provides functionality to load test data from CSV or JSON files
//! and generate k6 scripts that use SharedArray for memory-efficient data distribution.

use crate::error::{BenchError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Strategy for distributing data across VUs and iterations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DataDistribution {
    /// Each VU gets a unique row (VU 1 gets row 0, VU 2 gets row 1, etc.)
    UniquePerVu,
    /// Each iteration gets a unique row (wraps around when data is exhausted)
    UniquePerIteration,
    /// Random row selection on each iteration
    Random,
    /// Sequential iteration through all rows (same for all VUs)
    Sequential,
}

impl Default for DataDistribution {
    fn default() -> Self {
        Self::UniquePerVu
    }
}

impl std::str::FromStr for DataDistribution {
    type Err = BenchError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "unique-per-vu" | "uniquepervu" => Ok(Self::UniquePerVu),
            "unique-per-iteration" | "uniqueperiteration" => Ok(Self::UniquePerIteration),
            "random" => Ok(Self::Random),
            "sequential" => Ok(Self::Sequential),
            _ => Err(BenchError::Other(format!(
                "Invalid data distribution: '{}'. Valid options: unique-per-vu, unique-per-iteration, random, sequential",
                s
            ))),
        }
    }
}

/// Mapping of data columns to request fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMapping {
    /// Source column name in the data file
    pub column: String,
    /// Target field path in the request (e.g., "body.name", "path.id", "header.X-Custom")
    pub target: String,
}

impl DataMapping {
    /// Create a new data mapping
    pub fn new(column: String, target: String) -> Self {
        Self { column, target }
    }

    /// Parse mappings from a comma-separated string
    /// Format: "column1:target1,column2:target2"
    pub fn parse_mappings(s: &str) -> Result<Vec<Self>> {
        if s.is_empty() {
            return Ok(Vec::new());
        }

        s.split(',')
            .map(|pair| {
                let parts: Vec<&str> = pair.trim().splitn(2, ':').collect();
                if parts.len() != 2 {
                    return Err(BenchError::Other(format!(
                        "Invalid mapping format: '{}'. Expected 'column:target'",
                        pair
                    )));
                }
                Ok(DataMapping::new(
                    parts[0].trim().to_string(),
                    parts[1].trim().to_string(),
                ))
            })
            .collect()
    }
}

/// Configuration for data-driven testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDrivenConfig {
    /// Path to the data file (CSV or JSON)
    pub file_path: String,
    /// Data distribution strategy
    #[serde(default)]
    pub distribution: DataDistribution,
    /// Column to field mappings
    #[serde(default)]
    pub mappings: Vec<DataMapping>,
    /// Whether the CSV has a header row
    #[serde(default = "default_true")]
    pub csv_has_header: bool,
}

fn default_true() -> bool {
    true
}

impl DataDrivenConfig {
    /// Create a new data-driven config
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            distribution: DataDistribution::default(),
            mappings: Vec::new(),
            csv_has_header: true,
        }
    }

    /// Set the distribution strategy
    pub fn with_distribution(mut self, distribution: DataDistribution) -> Self {
        self.distribution = distribution;
        self
    }

    /// Add mappings
    pub fn with_mappings(mut self, mappings: Vec<DataMapping>) -> Self {
        self.mappings = mappings;
        self
    }

    /// Detect file type from extension
    pub fn file_type(&self) -> DataFileType {
        if self.file_path.ends_with(".csv") {
            DataFileType::Csv
        } else if self.file_path.ends_with(".json") {
            DataFileType::Json
        } else {
            // Default to CSV
            DataFileType::Csv
        }
    }
}

/// Type of data file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFileType {
    Csv,
    Json,
}

/// Generates k6 JavaScript code for data-driven testing
pub struct DataDrivenGenerator;

impl DataDrivenGenerator {
    /// Generate k6 imports for data-driven testing
    pub fn generate_imports(file_type: DataFileType) -> String {
        let mut imports = String::new();

        imports.push_str("import { SharedArray } from 'k6/data';\n");

        if file_type == DataFileType::Csv {
            imports.push_str("import papaparse from 'https://jslib.k6.io/papaparse/5.1.1/index.js';\n");
        }

        imports
    }

    /// Generate k6 code to load the data file
    pub fn generate_data_loading(config: &DataDrivenConfig) -> String {
        let mut code = String::new();

        code.push_str("// Load test data using SharedArray for memory efficiency\n");
        code.push_str("const testData = new SharedArray('test data', function() {\n");

        match config.file_type() {
            DataFileType::Csv => {
                code.push_str(&format!(
                    "  const csvData = open('{}');\n",
                    config.file_path
                ));
                if config.csv_has_header {
                    code.push_str("  return papaparse.parse(csvData, { header: true }).data;\n");
                } else {
                    code.push_str("  return papaparse.parse(csvData, { header: false }).data;\n");
                }
            }
            DataFileType::Json => {
                code.push_str(&format!(
                    "  return JSON.parse(open('{}'));\n",
                    config.file_path
                ));
            }
        }

        code.push_str("});\n\n");

        code
    }

    /// Generate k6 code for row selection based on distribution strategy
    pub fn generate_row_selection(distribution: DataDistribution) -> String {
        match distribution {
            DataDistribution::UniquePerVu => {
                "// Unique row per VU (wraps if more VUs than data rows)\n\
                 const rowIndex = (__VU - 1) % testData.length;\n\
                 const row = testData[rowIndex];\n".to_string()
            }
            DataDistribution::UniquePerIteration => {
                "// Unique row per iteration (cycles through data)\n\
                 const rowIndex = __ITER % testData.length;\n\
                 const row = testData[rowIndex];\n".to_string()
            }
            DataDistribution::Random => {
                "// Random row selection\n\
                 const rowIndex = Math.floor(Math.random() * testData.length);\n\
                 const row = testData[rowIndex];\n".to_string()
            }
            DataDistribution::Sequential => {
                "// Sequential iteration (same for all VUs, based on iteration)\n\
                 const rowIndex = __ITER % testData.length;\n\
                 const row = testData[rowIndex];\n".to_string()
            }
        }
    }

    /// Generate k6 code to apply mappings from row data
    pub fn generate_apply_mappings(mappings: &[DataMapping]) -> String {
        if mappings.is_empty() {
            return "// No explicit mappings - row data available as 'row' object\n".to_string();
        }

        let mut code = String::new();
        code.push_str("// Apply data mappings\n");

        for mapping in mappings {
            let target_parts: Vec<&str> = mapping.target.splitn(2, '.').collect();
            if target_parts.len() == 2 {
                let target_type = target_parts[0];
                let field_name = target_parts[1];

                match target_type {
                    "body" => {
                        code.push_str(&format!(
                            "requestBody['{}'] = row['{}'];\n",
                            field_name, mapping.column
                        ));
                    }
                    "path" => {
                        code.push_str(&format!(
                            "pathParams['{}'] = row['{}'];\n",
                            field_name, mapping.column
                        ));
                    }
                    "query" => {
                        code.push_str(&format!(
                            "queryParams['{}'] = row['{}'];\n",
                            field_name, mapping.column
                        ));
                    }
                    "header" => {
                        code.push_str(&format!(
                            "requestHeaders['{}'] = row['{}'];\n",
                            field_name, mapping.column
                        ));
                    }
                    _ => {
                        code.push_str(&format!(
                            "// Unknown target type '{}' for column '{}'\n",
                            target_type, mapping.column
                        ));
                    }
                }
            } else {
                // Simple mapping without type prefix - assume body
                code.push_str(&format!(
                    "requestBody['{}'] = row['{}'];\n",
                    mapping.target, mapping.column
                ));
            }
        }

        code
    }

    /// Generate complete data-driven test setup code
    pub fn generate_setup(config: &DataDrivenConfig) -> String {
        let mut code = String::new();

        code.push_str(&Self::generate_imports(config.file_type()));
        code.push('\n');
        code.push_str(&Self::generate_data_loading(config));

        code
    }

    /// Generate code for within the default function
    pub fn generate_iteration_code(config: &DataDrivenConfig) -> String {
        let mut code = String::new();

        code.push_str(&Self::generate_row_selection(config.distribution));
        code.push('\n');
        code.push_str(&Self::generate_apply_mappings(&config.mappings));

        code
    }
}

/// Validate a data file exists and has the expected format
pub fn validate_data_file(path: &Path) -> Result<DataFileInfo> {
    if !path.exists() {
        return Err(BenchError::Other(format!(
            "Data file not found: {}",
            path.display()
        )));
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| BenchError::Other(format!("Failed to read data file: {}", e)))?;

    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "csv" => validate_csv(&content),
        "json" => validate_json(&content),
        _ => Err(BenchError::Other(format!(
            "Unsupported data file format: .{}. Use .csv or .json",
            extension
        ))),
    }
}

/// Information about a validated data file
#[derive(Debug, Clone)]
pub struct DataFileInfo {
    /// Number of rows in the file
    pub row_count: usize,
    /// Column names (if available)
    pub columns: Vec<String>,
    /// File type
    pub file_type: DataFileType,
}

fn validate_csv(content: &str) -> Result<DataFileInfo> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Err(BenchError::Other("CSV file is empty".to_string()));
    }

    // Assume first line is header
    let header = lines[0];
    let columns: Vec<String> = header.split(',').map(|s| s.trim().to_string()).collect();
    let row_count = lines.len() - 1; // Exclude header

    Ok(DataFileInfo {
        row_count,
        columns,
        file_type: DataFileType::Csv,
    })
}

fn validate_json(content: &str) -> Result<DataFileInfo> {
    let value: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| BenchError::Other(format!("Invalid JSON: {}", e)))?;

    match value {
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return Err(BenchError::Other("JSON array is empty".to_string()));
            }

            // Get columns from first object
            let columns = if let Some(serde_json::Value::Object(obj)) = arr.first() {
                obj.keys().cloned().collect()
            } else {
                Vec::new()
            };

            Ok(DataFileInfo {
                row_count: arr.len(),
                columns,
                file_type: DataFileType::Json,
            })
        }
        _ => Err(BenchError::Other(
            "JSON data must be an array of objects".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_data_distribution_default() {
        assert_eq!(DataDistribution::default(), DataDistribution::UniquePerVu);
    }

    #[test]
    fn test_data_distribution_from_str() {
        assert_eq!(
            DataDistribution::from_str("unique-per-vu").unwrap(),
            DataDistribution::UniquePerVu
        );
        assert_eq!(
            DataDistribution::from_str("unique-per-iteration").unwrap(),
            DataDistribution::UniquePerIteration
        );
        assert_eq!(
            DataDistribution::from_str("random").unwrap(),
            DataDistribution::Random
        );
        assert_eq!(
            DataDistribution::from_str("sequential").unwrap(),
            DataDistribution::Sequential
        );
    }

    #[test]
    fn test_data_distribution_from_str_variants() {
        // Test with underscores
        assert_eq!(
            DataDistribution::from_str("unique_per_vu").unwrap(),
            DataDistribution::UniquePerVu
        );

        // Test camelCase-ish
        assert_eq!(
            DataDistribution::from_str("uniquePerVu").unwrap(),
            DataDistribution::UniquePerVu
        );
    }

    #[test]
    fn test_data_distribution_from_str_invalid() {
        assert!(DataDistribution::from_str("invalid").is_err());
    }

    #[test]
    fn test_data_mapping_parse() {
        let mappings = DataMapping::parse_mappings("name:body.username,id:path.userId").unwrap();
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].column, "name");
        assert_eq!(mappings[0].target, "body.username");
        assert_eq!(mappings[1].column, "id");
        assert_eq!(mappings[1].target, "path.userId");
    }

    #[test]
    fn test_data_mapping_parse_empty() {
        let mappings = DataMapping::parse_mappings("").unwrap();
        assert!(mappings.is_empty());
    }

    #[test]
    fn test_data_mapping_parse_invalid() {
        assert!(DataMapping::parse_mappings("invalid").is_err());
    }

    #[test]
    fn test_data_driven_config_file_type() {
        let csv_config = DataDrivenConfig::new("data.csv".to_string());
        assert_eq!(csv_config.file_type(), DataFileType::Csv);

        let json_config = DataDrivenConfig::new("data.json".to_string());
        assert_eq!(json_config.file_type(), DataFileType::Json);

        let unknown_config = DataDrivenConfig::new("data.txt".to_string());
        assert_eq!(unknown_config.file_type(), DataFileType::Csv); // Default to CSV
    }

    #[test]
    fn test_generate_imports_csv() {
        let imports = DataDrivenGenerator::generate_imports(DataFileType::Csv);
        assert!(imports.contains("SharedArray"));
        assert!(imports.contains("papaparse"));
    }

    #[test]
    fn test_generate_imports_json() {
        let imports = DataDrivenGenerator::generate_imports(DataFileType::Json);
        assert!(imports.contains("SharedArray"));
        assert!(!imports.contains("papaparse"));
    }

    #[test]
    fn test_generate_data_loading_csv() {
        let config = DataDrivenConfig::new("test.csv".to_string());
        let code = DataDrivenGenerator::generate_data_loading(&config);

        assert!(code.contains("SharedArray"));
        assert!(code.contains("open('test.csv')"));
        assert!(code.contains("papaparse.parse"));
        assert!(code.contains("header: true"));
    }

    #[test]
    fn test_generate_data_loading_json() {
        let config = DataDrivenConfig::new("test.json".to_string());
        let code = DataDrivenGenerator::generate_data_loading(&config);

        assert!(code.contains("SharedArray"));
        assert!(code.contains("open('test.json')"));
        assert!(code.contains("JSON.parse"));
    }

    #[test]
    fn test_generate_row_selection_unique_per_vu() {
        let code = DataDrivenGenerator::generate_row_selection(DataDistribution::UniquePerVu);
        assert!(code.contains("__VU - 1"));
        assert!(code.contains("testData.length"));
    }

    #[test]
    fn test_generate_row_selection_unique_per_iteration() {
        let code = DataDrivenGenerator::generate_row_selection(DataDistribution::UniquePerIteration);
        assert!(code.contains("__ITER"));
        assert!(code.contains("testData.length"));
    }

    #[test]
    fn test_generate_row_selection_random() {
        let code = DataDrivenGenerator::generate_row_selection(DataDistribution::Random);
        assert!(code.contains("Math.random()"));
        assert!(code.contains("testData.length"));
    }

    #[test]
    fn test_generate_apply_mappings() {
        let mappings = vec![
            DataMapping::new("name".to_string(), "body.username".to_string()),
            DataMapping::new("id".to_string(), "path.userId".to_string()),
            DataMapping::new("token".to_string(), "header.Authorization".to_string()),
        ];

        let code = DataDrivenGenerator::generate_apply_mappings(&mappings);

        assert!(code.contains("requestBody['username'] = row['name']"));
        assert!(code.contains("pathParams['userId'] = row['id']"));
        assert!(code.contains("requestHeaders['Authorization'] = row['token']"));
    }

    #[test]
    fn test_generate_apply_mappings_empty() {
        let code = DataDrivenGenerator::generate_apply_mappings(&[]);
        assert!(code.contains("No explicit mappings"));
    }

    #[test]
    fn test_validate_csv() {
        let content = "name,email,age\nAlice,alice@test.com,30\nBob,bob@test.com,25";
        let info = validate_csv(content).unwrap();

        assert_eq!(info.row_count, 2);
        assert_eq!(info.columns, vec!["name", "email", "age"]);
        assert_eq!(info.file_type, DataFileType::Csv);
    }

    #[test]
    fn test_validate_csv_empty() {
        let content = "";
        assert!(validate_csv(content).is_err());
    }

    #[test]
    fn test_validate_json() {
        let content = r#"[{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]"#;
        let info = validate_json(content).unwrap();

        assert_eq!(info.row_count, 2);
        assert!(info.columns.contains(&"name".to_string()));
        assert!(info.columns.contains(&"age".to_string()));
        assert_eq!(info.file_type, DataFileType::Json);
    }

    #[test]
    fn test_validate_json_empty_array() {
        let content = "[]";
        assert!(validate_json(content).is_err());
    }

    #[test]
    fn test_validate_json_not_array() {
        let content = r#"{"name": "Alice"}"#;
        assert!(validate_json(content).is_err());
    }

    #[test]
    fn test_generate_setup() {
        let config = DataDrivenConfig::new("users.csv".to_string())
            .with_distribution(DataDistribution::Random);

        let code = DataDrivenGenerator::generate_setup(&config);

        assert!(code.contains("SharedArray"));
        assert!(code.contains("papaparse"));
        assert!(code.contains("users.csv"));
    }

    #[test]
    fn test_generate_iteration_code() {
        let config = DataDrivenConfig::new("data.csv".to_string())
            .with_distribution(DataDistribution::UniquePerVu)
            .with_mappings(vec![DataMapping::new(
                "email".to_string(),
                "body.email".to_string(),
            )]);

        let code = DataDrivenGenerator::generate_iteration_code(&config);

        assert!(code.contains("__VU - 1"));
        assert!(code.contains("requestBody['email'] = row['email']"));
    }
}
