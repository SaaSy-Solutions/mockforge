//! Record/replay listing for HTTP/gRPC/WS fixtures.
use globwalk::GlobWalkerBuilder;
use serde::Serialize;

/// A single replay fixture item
#[derive(Debug, Serialize)]
pub struct ReplayItem {
    /// Protocol name (http, grpc, ws)
    pub protocol: String,
    /// OpenAPI operation ID
    pub operation_id: String,
    /// Timestamp when the fixture was saved
    pub saved_at: String,
    /// File system path to the fixture file
    pub path: String,
}

/// List all replay fixtures from the fixtures root directory
///
/// # Arguments
/// * `fixtures_root` - Root directory containing protocol subdirectories (http, grpc, ws)
///
/// # Returns
/// Vector of replay items sorted by timestamp (newest first)
pub fn list_all(fixtures_root: &str) -> anyhow::Result<Vec<ReplayItem>> {
    use std::path::Path;

    let fixtures_path = Path::new(fixtures_root);
    let mut out = Vec::new();

    for proto in ["http", "grpc", "ws"] {
        // Use the fixtures_root as the base directory for globbing
        let proto_pattern = format!("{proto}/**/*.json");
        for entry in GlobWalkerBuilder::from_patterns(fixtures_path, &[&proto_pattern]).build()? {
            let p = entry?.path().to_path_buf();
            if p.extension().map(|e| e == "json").unwrap_or(false) {
                // Get relative path from fixtures_root for consistent path handling
                let relative_path = p.strip_prefix(fixtures_path)
                    .unwrap_or(&p)
                    .to_path_buf();

                let comps: Vec<_> =
                    relative_path.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
                let len = comps.len();
                let (op_id, ts) = if len >= 2 {
                    (comps[len - 2].clone(), comps[len - 1].replace(".json", ""))
                } else {
                    ("unknown".into(), "unknown".into())
                };
                out.push(ReplayItem {
                    protocol: proto.to_string(),
                    operation_id: op_id,
                    saved_at: ts,
                    path: p.to_string_lossy().to_string(),
                });
            }
        }
    }
    out.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_fixture_file(
        dir: &Path,
        protocol: &str,
        op_id: &str,
        timestamp: &str,
    ) -> std::io::Result<()> {
        let path = dir.join(protocol).join(op_id);
        fs::create_dir_all(&path)?;
        let file_path = path.join(format!("{}.json", timestamp));
        fs::write(file_path, "{}")?;
        Ok(())
    }

    #[test]
    fn test_replay_item_structure() {
        let item = ReplayItem {
            protocol: "http".to_string(),
            operation_id: "getUser".to_string(),
            saved_at: "2024-01-01T12:00:00".to_string(),
            path: "/fixtures/http/getUser/2024-01-01T12:00:00.json".to_string(),
        };

        assert_eq!(item.protocol, "http");
        assert_eq!(item.operation_id, "getUser");
        assert_eq!(item.saved_at, "2024-01-01T12:00:00");
    }

    #[test]
    fn test_list_all_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_root = temp_dir.path().to_str().unwrap();

        let result = list_all(fixtures_root);
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_list_all_with_http_fixtures() {
        let temp_dir = TempDir::new().unwrap();

        // Create some HTTP fixtures
        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "getUser", "2024-01-01T12:00:00").unwrap();
        create_fixture_file(&temp_path, "http", "getUser", "2024-01-02T12:00:00").unwrap();
        create_fixture_file(&temp_path, "http", "createUser", "2024-01-03T12:00:00").unwrap();

        let fixtures_root = temp_path.to_str().unwrap();
        let result = list_all(fixtures_root);

        assert!(result.is_ok());
        let items = result.unwrap();

        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|item| item.protocol == "http"));
    }

    #[test]
    fn test_list_all_with_multiple_protocols() {
        let temp_dir = TempDir::new().unwrap();

        // Create fixtures for different protocols
        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "getUser", "2024-01-01T12:00:00").unwrap();
        create_fixture_file(&temp_path, "grpc", "GetUser", "2024-01-02T12:00:00").unwrap();
        create_fixture_file(&temp_path, "ws", "subscribe", "2024-01-03T12:00:00").unwrap();

        let fixtures_root = temp_path.to_str().unwrap();
        let result = list_all(fixtures_root);

        assert!(result.is_ok());
        let items = result.unwrap();

        assert_eq!(items.len(), 3);

        let protocols: Vec<&str> = items.iter().map(|i| i.protocol.as_str()).collect();
        assert!(protocols.contains(&"http"));
        assert!(protocols.contains(&"grpc"));
        assert!(protocols.contains(&"ws"));
    }

    #[test]
    fn test_list_all_sorted_by_timestamp() {
        let temp_dir = TempDir::new().unwrap();

        // Create fixtures with different timestamps
        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "op1", "2024-01-01T12:00:00").unwrap();
        create_fixture_file(&temp_path, "http", "op2", "2024-01-03T12:00:00").unwrap();
        create_fixture_file(&temp_path, "http", "op3", "2024-01-02T12:00:00").unwrap();

        let fixtures_root = temp_path.to_str().unwrap();
        let result = list_all(fixtures_root);

        assert!(result.is_ok());
        let items = result.unwrap();

        assert_eq!(items.len(), 3);
        // Should be sorted by timestamp descending
        assert!(items[0].saved_at >= items[1].saved_at);
        assert!(items[1].saved_at >= items[2].saved_at);
    }

    #[test]
    fn test_list_all_ignores_non_json_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create JSON and non-JSON files
        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "getUser", "2024-01-01T12:00:00").unwrap();

        let txt_path = temp_path.join("http").join("getUser");
        fs::create_dir_all(&txt_path).unwrap();
        fs::write(txt_path.join("data.txt"), "not json").unwrap();

        let fixtures_root = temp_path.to_str().unwrap();
        let result = list_all(fixtures_root);

        assert!(result.is_ok());
        let items = result.unwrap();

        // Should only find the .json file
        assert_eq!(items.len(), 1);
        assert!(items[0].path.ends_with(".json"));
    }

    #[test]
    fn test_list_all_extracts_operation_id() {
        let temp_dir = TempDir::new().unwrap();

        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "getUserById", "2024-01-01T12:00:00").unwrap();

        let fixtures_root = temp_path.to_str().unwrap();
        let result = list_all(fixtures_root);

        assert!(result.is_ok());
        let items = result.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].operation_id, "getUserById");
    }

    #[test]
    fn test_list_all_extracts_timestamp_without_extension() {
        let temp_dir = TempDir::new().unwrap();

        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "getUser", "2024-01-01T12:00:00").unwrap();

        let fixtures_root = temp_path.to_str().unwrap();
        let result = list_all(fixtures_root);

        assert!(result.is_ok());
        let items = result.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].saved_at, "2024-01-01T12:00:00");
        assert!(!items[0].saved_at.contains(".json"));
    }
}
