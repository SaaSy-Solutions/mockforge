use anyhow::Result;
use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Virtual File System for FTP server
#[derive(Debug, Clone)]
pub struct VirtualFileSystem {
    root: PathBuf,
    files: Arc<RwLock<HashMap<PathBuf, VirtualFile>>>,
    fixtures: HashMap<PathBuf, FileFixture>,
}

impl VirtualFileSystem {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            files: Arc::new(RwLock::new(HashMap::new())),
            fixtures: HashMap::new(),
        }
    }

    pub fn add_file(&self, path: PathBuf, file: VirtualFile) -> Result<()> {
        let mut files = self.files.blocking_write();
        files.insert(path, file);
        Ok(())
    }

    pub fn get_file(&self, path: &Path) -> Option<VirtualFile> {
        let files = self.files.blocking_read();
        files.get(path).cloned()
    }

    pub fn remove_file(&self, path: &Path) -> Result<()> {
        let mut files = self.files.blocking_write();
        files.remove(path);
        Ok(())
    }

    pub fn list_files(&self, path: &Path) -> Vec<VirtualFile> {
        let files = self.files.blocking_read();
        files
            .iter()
            .filter(|(file_path, _)| file_path.starts_with(path))
            .map(|(_, file)| file.clone())
            .collect()
    }

    pub fn clear(&self) -> Result<()> {
        let mut files = self.files.blocking_write();
        files.clear();
        Ok(())
    }
}

/// Virtual file representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualFile {
    pub path: PathBuf,
    pub content: FileContent,
    pub metadata: FileMetadata,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl VirtualFile {
    pub fn new(path: PathBuf, content: FileContent, metadata: FileMetadata) -> Self {
        let now = Utc::now();
        Self {
            path,
            content,
            metadata,
            created_at: now,
            modified_at: now,
        }
    }

    pub fn render_content(&self) -> Result<Vec<u8>> {
        match &self.content {
            FileContent::Static(data) => Ok(data.clone()),
            FileContent::Template(template) => {
                // Render template using Handlebars
                let handlebars = Handlebars::new();
                let context = create_template_context();
                let rendered = handlebars.render_template(template, &context)?;
                Ok(rendered.into_bytes())
            }
            FileContent::Generated { size, pattern } => match pattern {
                GenerationPattern::Random => Ok((0..*size).map(|_| rand::random::<u8>()).collect()),
                GenerationPattern::Zeros => Ok(vec![0; *size]),
                GenerationPattern::Ones => Ok(vec![1; *size]),
                GenerationPattern::Incremental => Ok((0..*size).map(|i| (i % 256) as u8).collect()),
            },
        }
    }
}

/// File content types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileContent {
    Static(Vec<u8>),
    Template(String),
    Generated {
        size: usize,
        pattern: GenerationPattern,
    },
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub permissions: String,
    pub owner: String,
    pub group: String,
    pub size: u64,
}

impl Default for FileMetadata {
    fn default() -> Self {
        Self {
            permissions: "644".to_string(),
            owner: "mockforge".to_string(),
            group: "users".to_string(),
            size: 0,
        }
    }
}

/// Generation patterns for synthetic files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerationPattern {
    Random,
    Zeros,
    Ones,
    Incremental,
}

/// File fixture for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFixture {
    pub path: PathBuf,
    pub content: FileContent,
    pub metadata: FileMetadata,
}

/// Create a template context with common variables for template rendering
fn create_template_context() -> Value {
    let mut context = serde_json::Map::new();

    // Add current timestamp
    let now = Utc::now();
    context.insert("now".to_string(), Value::String(now.to_rfc3339()));
    context.insert("timestamp".to_string(), Value::Number(now.timestamp().into()));
    context.insert("date".to_string(), Value::String(now.format("%Y-%m-%d").to_string()));
    context.insert("time".to_string(), Value::String(now.format("%H:%M:%S").to_string()));

    // Add random values
    context.insert("random_int".to_string(), Value::Number(rand::random::<i64>().into()));
    context.insert(
        "random_float".to_string(),
        Value::String(format!("{:.6}", rand::random::<f64>())),
    );

    // Add UUID
    context.insert("uuid".to_string(), Value::String(uuid::Uuid::new_v4().to_string()));

    // Add some sample data
    let mut faker = serde_json::Map::new();
    faker.insert("name".to_string(), Value::String("John Doe".to_string()));
    faker.insert("email".to_string(), Value::String("john.doe@example.com".to_string()));
    faker.insert("age".to_string(), Value::Number(30.into()));
    context.insert("faker".to_string(), Value::Object(faker));

    Value::Object(context)
}
