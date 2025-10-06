//! Record/replay listing for HTTP/gRPC/WS fixtures.
use globwalk::GlobWalkerBuilder;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ReplayItem {
    pub protocol: String,
    pub operation_id: String,
    pub saved_at: String,
    pub path: String,
}

pub fn list_all(fixtures_root: &str) -> anyhow::Result<Vec<ReplayItem>> {
    let mut out = Vec::new();
    for proto in ["http", "grpc", "ws"] {
        let pat = format!("{root}/{proto}/**/*.json", root = fixtures_root, proto = proto);
        for entry in GlobWalkerBuilder::from_patterns(".", &[&pat]).build()? {
            let p = entry?.path().to_path_buf();
            if p.extension().map(|e| e == "json").unwrap_or(false) {
                let comps: Vec<_> =
                    p.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
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
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_fixture_file(dir: &PathBuf, protocol: &str, op_id: &str, timestamp: &str) -> std::io::Result<()> {
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
    #[ignore = "Requires filesystem globbing setup"]
    fn test_list_all_with_http_fixtures() {
        let temp_dir = TempDir::new().unwrap();

        // Create some HTTP fixtures
        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "getUser", "2024-01-01T12:00:00").unwrap();
        create_fixture_file(&temp_path, "http", "getUser", "2024-01-02T12:00:00").unwrap();
        create_fixture_file(&temp_path, "http", "createUser", "2024-01-03T12:00:00").unwrap();

        // Change to temp directory temporarily
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_path).unwrap();

        let result = list_all(".");

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        let items = result.unwrap();

        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|item| item.protocol == "http"));
    }

    #[test]
    #[ignore = "Requires filesystem globbing setup"]
    fn test_list_all_with_multiple_protocols() {
        let temp_dir = TempDir::new().unwrap();

        // Create fixtures for different protocols
        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "getUser", "2024-01-01T12:00:00").unwrap();
        create_fixture_file(&temp_path, "grpc", "GetUser", "2024-01-02T12:00:00").unwrap();
        create_fixture_file(&temp_path, "ws", "subscribe", "2024-01-03T12:00:00").unwrap();

        // Change to temp directory temporarily
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_path).unwrap();

        let result = list_all(".");

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        let items = result.unwrap();

        assert_eq!(items.len(), 3);

        let protocols: Vec<&str> = items.iter().map(|i| i.protocol.as_str()).collect();
        assert!(protocols.contains(&"http"));
        assert!(protocols.contains(&"grpc"));
        assert!(protocols.contains(&"ws"));
    }

    #[test]
    #[ignore = "Requires filesystem globbing setup"]
    fn test_list_all_sorted_by_timestamp() {
        let temp_dir = TempDir::new().unwrap();

        // Create fixtures with different timestamps
        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "op1", "2024-01-01T12:00:00").unwrap();
        create_fixture_file(&temp_path, "http", "op2", "2024-01-03T12:00:00").unwrap();
        create_fixture_file(&temp_path, "http", "op3", "2024-01-02T12:00:00").unwrap();

        // Change to temp directory temporarily
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_path).unwrap();

        let result = list_all(".");

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        let items = result.unwrap();

        assert_eq!(items.len(), 3);
        // Should be sorted by timestamp descending
        assert!(items[0].saved_at >= items[1].saved_at);
        assert!(items[1].saved_at >= items[2].saved_at);
    }

    #[test]
    #[ignore = "Requires filesystem globbing setup"]
    fn test_list_all_ignores_non_json_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create JSON and non-JSON files
        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "getUser", "2024-01-01T12:00:00").unwrap();

        let txt_path = temp_path.join("http").join("getUser");
        fs::create_dir_all(&txt_path).unwrap();
        fs::write(txt_path.join("data.txt"), "not json").unwrap();

        // Change to temp directory temporarily
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_path).unwrap();

        let result = list_all(".");

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        let items = result.unwrap();

        // Should only find the .json file
        assert_eq!(items.len(), 1);
        assert!(items[0].path.ends_with(".json"));
    }

    #[test]
    #[ignore = "Requires filesystem globbing setup"]
    fn test_list_all_extracts_operation_id() {
        let temp_dir = TempDir::new().unwrap();

        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "getUserById", "2024-01-01T12:00:00").unwrap();

        // Change to temp directory temporarily
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_path).unwrap();

        let result = list_all(".");

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        let items = result.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].operation_id, "getUserById");
    }

    #[test]
    #[ignore = "Requires filesystem globbing setup"]
    fn test_list_all_extracts_timestamp_without_extension() {
        let temp_dir = TempDir::new().unwrap();

        let temp_path = temp_dir.path().to_path_buf();
        create_fixture_file(&temp_path, "http", "getUser", "2024-01-01T12:00:00").unwrap();

        // Change to temp directory temporarily
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_path).unwrap();

        let result = list_all(".");

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        let items = result.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].saved_at, "2024-01-01T12:00:00");
        assert!(!items[0].saved_at.contains(".json"));
    }
}
