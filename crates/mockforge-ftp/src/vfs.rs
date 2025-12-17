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
    #[allow(dead_code)]
    root: PathBuf,
    files: Arc<RwLock<HashMap<PathBuf, VirtualFile>>>,
    fixtures: Arc<RwLock<HashMap<PathBuf, FileFixture>>>,
}

impl VirtualFileSystem {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            files: Arc::new(RwLock::new(HashMap::new())),
            fixtures: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_file(&self, path: PathBuf, file: VirtualFile) -> Result<()> {
        let mut files = self.files.blocking_write();
        files.insert(path, file);
        Ok(())
    }

    pub fn get_file(&self, path: &Path) -> Option<VirtualFile> {
        let files = self.files.blocking_read();
        if let Some(file) = files.get(path) {
            return Some(file.clone());
        }

        // Check fixtures
        let fixtures = self.fixtures.blocking_read();
        if let Some(fixture) = fixtures.get(path) {
            return Some(fixture.clone().to_virtual_file());
        }

        None
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

    pub fn add_fixture(&self, fixture: FileFixture) -> Result<()> {
        let mut fixtures = self.fixtures.blocking_write();
        fixtures.insert(fixture.path.clone(), fixture);
        Ok(())
    }

    pub fn load_fixtures(&self, fixtures: Vec<FileFixture>) -> Result<()> {
        for fixture in fixtures {
            self.add_fixture(fixture)?;
        }
        Ok(())
    }

    /// Async version of add_file - use this in async contexts
    pub async fn add_file_async(&self, path: PathBuf, file: VirtualFile) -> Result<()> {
        let mut files = self.files.write().await;
        files.insert(path, file);
        Ok(())
    }

    /// Async version of get_file - use this in async contexts
    pub async fn get_file_async(&self, path: &Path) -> Option<VirtualFile> {
        let files = self.files.read().await;
        if let Some(file) = files.get(path) {
            return Some(file.clone());
        }

        // Check fixtures
        let fixtures = self.fixtures.read().await;
        if let Some(fixture) = fixtures.get(path) {
            return Some(fixture.clone().to_virtual_file());
        }

        None
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

impl FileFixture {
    pub fn to_virtual_file(self) -> VirtualFile {
        VirtualFile::new(self.path, self.content, self.metadata)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_metadata_default() {
        let metadata = FileMetadata::default();
        assert_eq!(metadata.permissions, "644");
        assert_eq!(metadata.owner, "mockforge");
        assert_eq!(metadata.group, "users");
        assert_eq!(metadata.size, 0);
    }

    #[test]
    fn test_file_metadata_clone() {
        let metadata = FileMetadata {
            permissions: "755".to_string(),
            owner: "root".to_string(),
            group: "root".to_string(),
            size: 1024,
        };

        let cloned = metadata.clone();
        assert_eq!(metadata.permissions, cloned.permissions);
        assert_eq!(metadata.owner, cloned.owner);
        assert_eq!(metadata.size, cloned.size);
    }

    #[test]
    fn test_generation_pattern_clone() {
        let pattern = GenerationPattern::Random;
        let _cloned = pattern.clone();
        // Just verify it can be cloned
    }

    #[test]
    fn test_generation_pattern_debug() {
        let pattern = GenerationPattern::Zeros;
        let debug = format!("{:?}", pattern);
        assert!(debug.contains("Zeros"));
    }

    #[test]
    fn test_file_content_static() {
        let content = FileContent::Static(b"hello world".to_vec());
        let debug = format!("{:?}", content);
        assert!(debug.contains("Static"));
    }

    #[test]
    fn test_file_content_template() {
        let content = FileContent::Template("Hello {{name}}".to_string());
        let debug = format!("{:?}", content);
        assert!(debug.contains("Template"));
    }

    #[test]
    fn test_file_content_generated() {
        let content = FileContent::Generated {
            size: 100,
            pattern: GenerationPattern::Ones,
        };
        let debug = format!("{:?}", content);
        assert!(debug.contains("Generated"));
    }

    #[test]
    fn test_virtual_file_new() {
        let file = VirtualFile::new(
            PathBuf::from("/test.txt"),
            FileContent::Static(b"content".to_vec()),
            FileMetadata::default(),
        );

        assert_eq!(file.path, PathBuf::from("/test.txt"));
    }

    #[test]
    fn test_virtual_file_render_static() {
        let file = VirtualFile::new(
            PathBuf::from("/test.txt"),
            FileContent::Static(b"hello".to_vec()),
            FileMetadata::default(),
        );

        let content = file.render_content().unwrap();
        assert_eq!(content, b"hello".to_vec());
    }

    #[test]
    fn test_virtual_file_render_generated_zeros() {
        let file = VirtualFile::new(
            PathBuf::from("/zeros.bin"),
            FileContent::Generated {
                size: 10,
                pattern: GenerationPattern::Zeros,
            },
            FileMetadata::default(),
        );

        let content = file.render_content().unwrap();
        assert_eq!(content.len(), 10);
        assert!(content.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_virtual_file_render_generated_ones() {
        let file = VirtualFile::new(
            PathBuf::from("/ones.bin"),
            FileContent::Generated {
                size: 10,
                pattern: GenerationPattern::Ones,
            },
            FileMetadata::default(),
        );

        let content = file.render_content().unwrap();
        assert_eq!(content.len(), 10);
        assert!(content.iter().all(|&b| b == 1));
    }

    #[test]
    fn test_virtual_file_render_generated_incremental() {
        let file = VirtualFile::new(
            PathBuf::from("/inc.bin"),
            FileContent::Generated {
                size: 256,
                pattern: GenerationPattern::Incremental,
            },
            FileMetadata::default(),
        );

        let content = file.render_content().unwrap();
        assert_eq!(content.len(), 256);
        for (i, &b) in content.iter().enumerate() {
            assert_eq!(b, i as u8);
        }
    }

    #[test]
    fn test_virtual_file_render_generated_random() {
        let file = VirtualFile::new(
            PathBuf::from("/random.bin"),
            FileContent::Generated {
                size: 100,
                pattern: GenerationPattern::Random,
            },
            FileMetadata::default(),
        );

        let content = file.render_content().unwrap();
        assert_eq!(content.len(), 100);
    }

    #[test]
    fn test_virtual_file_clone() {
        let file = VirtualFile::new(
            PathBuf::from("/test.txt"),
            FileContent::Static(b"test".to_vec()),
            FileMetadata::default(),
        );

        let cloned = file.clone();
        assert_eq!(file.path, cloned.path);
    }

    #[test]
    fn test_virtual_file_debug() {
        let file = VirtualFile::new(
            PathBuf::from("/test.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );

        let debug = format!("{:?}", file);
        assert!(debug.contains("VirtualFile"));
    }

    #[test]
    fn test_file_fixture_to_virtual_file() {
        let fixture = FileFixture {
            path: PathBuf::from("/fixture.txt"),
            content: FileContent::Static(b"fixture content".to_vec()),
            metadata: FileMetadata::default(),
        };

        let file = fixture.to_virtual_file();
        assert_eq!(file.path, PathBuf::from("/fixture.txt"));
    }

    #[test]
    fn test_file_fixture_clone() {
        let fixture = FileFixture {
            path: PathBuf::from("/test.txt"),
            content: FileContent::Static(vec![]),
            metadata: FileMetadata::default(),
        };

        let cloned = fixture.clone();
        assert_eq!(fixture.path, cloned.path);
    }

    #[test]
    fn test_vfs_new() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));
        let files = vfs.list_files(&PathBuf::from("/"));
        assert!(files.is_empty());
    }

    #[test]
    fn test_vfs_add_file() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));
        let file = VirtualFile::new(
            PathBuf::from("/test.txt"),
            FileContent::Static(b"hello".to_vec()),
            FileMetadata::default(),
        );

        vfs.add_file(PathBuf::from("/test.txt"), file).unwrap();

        let retrieved = vfs.get_file(&PathBuf::from("/test.txt"));
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_vfs_get_file_not_found() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));
        let retrieved = vfs.get_file(&PathBuf::from("/nonexistent.txt"));
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_vfs_remove_file() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));
        let file = VirtualFile::new(
            PathBuf::from("/test.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );

        vfs.add_file(PathBuf::from("/test.txt"), file).unwrap();
        vfs.remove_file(&PathBuf::from("/test.txt")).unwrap();

        let retrieved = vfs.get_file(&PathBuf::from("/test.txt"));
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_vfs_list_files() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));

        let file1 = VirtualFile::new(
            PathBuf::from("/dir/file1.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );
        let file2 = VirtualFile::new(
            PathBuf::from("/dir/file2.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );

        vfs.add_file(PathBuf::from("/dir/file1.txt"), file1).unwrap();
        vfs.add_file(PathBuf::from("/dir/file2.txt"), file2).unwrap();

        let files = vfs.list_files(&PathBuf::from("/dir"));
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_vfs_clear() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));

        let file = VirtualFile::new(
            PathBuf::from("/test.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );
        vfs.add_file(PathBuf::from("/test.txt"), file).unwrap();

        vfs.clear().unwrap();

        let files = vfs.list_files(&PathBuf::from("/"));
        assert!(files.is_empty());
    }

    #[test]
    fn test_vfs_add_fixture() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));
        let fixture = FileFixture {
            path: PathBuf::from("/fixture.txt"),
            content: FileContent::Static(b"fixture".to_vec()),
            metadata: FileMetadata::default(),
        };

        vfs.add_fixture(fixture).unwrap();

        let file = vfs.get_file(&PathBuf::from("/fixture.txt"));
        assert!(file.is_some());
    }

    #[test]
    fn test_vfs_load_fixtures() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));
        let fixtures = vec![
            FileFixture {
                path: PathBuf::from("/f1.txt"),
                content: FileContent::Static(vec![]),
                metadata: FileMetadata::default(),
            },
            FileFixture {
                path: PathBuf::from("/f2.txt"),
                content: FileContent::Static(vec![]),
                metadata: FileMetadata::default(),
            },
        ];

        vfs.load_fixtures(fixtures).unwrap();

        assert!(vfs.get_file(&PathBuf::from("/f1.txt")).is_some());
        assert!(vfs.get_file(&PathBuf::from("/f2.txt")).is_some());
    }

    #[test]
    fn test_vfs_clone() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));
        let _cloned = vfs.clone();
        // Just verify it can be cloned
    }

    #[test]
    fn test_vfs_debug() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));
        let debug = format!("{:?}", vfs);
        assert!(debug.contains("VirtualFileSystem"));
    }

    #[test]
    fn test_template_context_has_expected_fields() {
        let context = create_template_context();
        assert!(context.get("now").is_some());
        assert!(context.get("timestamp").is_some());
        assert!(context.get("date").is_some());
        assert!(context.get("uuid").is_some());
        assert!(context.get("faker").is_some());
    }

    #[test]
    fn test_virtual_file_render_template() {
        let file = VirtualFile::new(
            PathBuf::from("/template.txt"),
            FileContent::Template("Hello {{faker.name}}!".to_string()),
            FileMetadata::default(),
        );

        let content = file.render_content().unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Hello"));
        assert!(text.contains("John Doe")); // From the faker context
    }

    #[test]
    fn test_virtual_file_render_template_with_timestamp() {
        let file = VirtualFile::new(
            PathBuf::from("/timestamp.txt"),
            FileContent::Template("Current timestamp: {{timestamp}}".to_string()),
            FileMetadata::default(),
        );

        let content = file.render_content().unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("Current timestamp:"));
    }

    #[test]
    fn test_virtual_file_render_template_with_uuid() {
        let file = VirtualFile::new(
            PathBuf::from("/uuid.txt"),
            FileContent::Template("ID: {{uuid}}".to_string()),
            FileMetadata::default(),
        );

        let content = file.render_content().unwrap();
        let text = String::from_utf8(content).unwrap();
        assert!(text.starts_with("ID: "));
        // UUID should be present and not empty
        let uuid_part = text.trim_start_matches("ID: ");
        assert!(!uuid_part.is_empty());
    }

    #[test]
    fn test_vfs_get_file_from_fixtures() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));
        let fixture = FileFixture {
            path: PathBuf::from("/fixture.txt"),
            content: FileContent::Static(b"fixture content".to_vec()),
            metadata: FileMetadata::default(),
        };

        vfs.add_fixture(fixture).unwrap();

        let file = vfs.get_file(&PathBuf::from("/fixture.txt"));
        assert!(file.is_some());
        let content = file.unwrap().render_content().unwrap();
        assert_eq!(content, b"fixture content");
    }

    #[test]
    fn test_vfs_files_priority_over_fixtures() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));

        // Add a fixture
        let fixture = FileFixture {
            path: PathBuf::from("/test.txt"),
            content: FileContent::Static(b"fixture".to_vec()),
            metadata: FileMetadata::default(),
        };
        vfs.add_fixture(fixture).unwrap();

        // Add a regular file with same path
        let file = VirtualFile::new(
            PathBuf::from("/test.txt"),
            FileContent::Static(b"file".to_vec()),
            FileMetadata::default(),
        );
        vfs.add_file(PathBuf::from("/test.txt"), file).unwrap();

        // Files should take priority over fixtures
        let retrieved = vfs.get_file(&PathBuf::from("/test.txt")).unwrap();
        let content = retrieved.render_content().unwrap();
        assert_eq!(content, b"file");
    }

    #[test]
    fn test_vfs_list_files_empty_path() {
        let vfs = VirtualFileSystem::new(PathBuf::from("/"));

        let file1 = VirtualFile::new(
            PathBuf::from("/file1.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );
        let file2 = VirtualFile::new(
            PathBuf::from("/subdir/file2.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );

        vfs.add_file(PathBuf::from("/file1.txt"), file1).unwrap();
        vfs.add_file(PathBuf::from("/subdir/file2.txt"), file2).unwrap();

        // List all files from root
        let files = vfs.list_files(&PathBuf::from("/"));
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_virtual_file_serialization() {
        let file = VirtualFile::new(
            PathBuf::from("/test.txt"),
            FileContent::Static(b"test".to_vec()),
            FileMetadata::default(),
        );

        // Test serialization
        let serialized = serde_json::to_string(&file);
        assert!(serialized.is_ok());

        // Test deserialization
        let deserialized: Result<VirtualFile, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());
    }

    #[test]
    fn test_file_metadata_serialization() {
        let metadata = FileMetadata {
            permissions: "755".to_string(),
            owner: "root".to_string(),
            group: "admin".to_string(),
            size: 2048,
        };

        let serialized = serde_json::to_string(&metadata);
        assert!(serialized.is_ok());

        let deserialized: Result<FileMetadata, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());
    }

    #[test]
    fn test_file_content_serialization() {
        let content = FileContent::Static(b"test content".to_vec());
        let serialized = serde_json::to_string(&content);
        assert!(serialized.is_ok());
    }

    #[test]
    fn test_generation_pattern_serialization() {
        let pattern = GenerationPattern::Random;
        let serialized = serde_json::to_string(&pattern);
        assert!(serialized.is_ok());
    }
}
