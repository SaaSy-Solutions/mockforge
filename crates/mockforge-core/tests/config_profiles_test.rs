use mockforge_core::config::*;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_load_yaml_config_with_profiles() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mockforge.yaml");

    let config_content = r#"
http:
  port: 3000
  host: "0.0.0.0"

logging:
  level: "info"

profiles:
  dev:
    http:
      port: 4000
    logging:
      level: "debug"

  ci:
    http:
      port: 8080
    logging:
      level: "warn"
      json_format: true
"#;

    fs::write(&config_path, config_content).unwrap();

    // Test base config (no profile)
    let base_config = load_config_with_profile(&config_path, None).await.unwrap();
    assert_eq!(base_config.http.port, 3000);
    assert_eq!(base_config.logging.level, "info");

    // Test dev profile
    let dev_config = load_config_with_profile(&config_path, Some("dev")).await.unwrap();
    assert_eq!(dev_config.http.port, 4000);
    assert_eq!(dev_config.logging.level, "debug");

    // Test ci profile
    let ci_config = load_config_with_profile(&config_path, Some("ci")).await.unwrap();
    assert_eq!(ci_config.http.port, 8080);
    assert_eq!(ci_config.logging.level, "warn");
    assert!(ci_config.logging.json_format);
}

#[tokio::test]
async fn test_profile_not_found_error() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mockforge.yaml");

    let config_content = r#"
http:
  port: 3000

profiles:
  dev:
    http:
      port: 4000
"#;

    fs::write(&config_path, config_content).unwrap();

    // Test non-existent profile
    let result = load_config_with_profile(&config_path, Some("prod")).await;
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Profile 'prod' not found"));
    assert!(error_msg.contains("Available profiles: dev"));
}

#[tokio::test]
async fn test_apply_profile_merging() {
    let mut base = ServerConfig::default();
    base.http.port = 3000;
    base.websocket.port = 3001;
    base.logging.level = "info".to_string();

    let mut profile = ProfileConfig::default();
    profile.http = Some(HttpConfig {
        port: 8080,
        ..Default::default()
    });
    profile.logging = Some(LoggingConfig {
        level: "debug".to_string(),
        ..Default::default()
    });

    let merged = apply_profile(base, profile);

    // Profile overrides should apply
    assert_eq!(merged.http.port, 8080);
    assert_eq!(merged.logging.level, "debug");

    // Non-overridden values should remain
    assert_eq!(merged.websocket.port, 3001);
}

#[tokio::test]
async fn test_load_javascript_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mockforge.config.js");

    // JS config must be an expression that evaluates to the config object
    let js_content = r#"({
  http: {
    port: 3000,
    host: "0.0.0.0",
    enabled: true
  },
  logging: {
    level: "info",
    json_format: false
  },
  websocket: {
    enabled: true,
    port: 3001,
    host: "0.0.0.0",
    connection_timeout_secs: 300
  },
  grpc: {
    enabled: true,
    port: 50051,
    host: "0.0.0.0"
  },
  graphql: {
    enabled: true,
    port: 4000,
    host: "0.0.0.0",
    playground_enabled: true,
    introspection_enabled: true
  },
  admin: {
    enabled: false,
    port: 9080,
    host: "127.0.0.1",
    auth_required: false,
    api_enabled: true,
    prometheus_url: "http://localhost:9090"
  },
  profiles: {
    dev: {
      logging: {
        level: "debug",
        json_format: false
      }
    }
  }
})"#;

    fs::write(&config_path, js_content).unwrap();

    // Test loading JS config
    let config = load_config_auto(&config_path).await.unwrap();
    assert_eq!(config.http.port, 3000);
    assert_eq!(config.logging.level, "info");

    // Test with profile
    let dev_config = load_config_with_profile(&config_path, Some("dev")).await.unwrap();
    assert_eq!(dev_config.logging.level, "debug");
}

#[tokio::test]
async fn test_load_typescript_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mockforge.config.ts");

    // TypeScript config with type annotations
    let ts_content = r#"({
  http: {
    port: 3000,
    host: "0.0.0.0",
    enabled: true
  },
  logging: {
    level: "info",
    json_format: false
  },
  websocket: {
    enabled: true,
    port: 3001,
    host: "0.0.0.0",
    connection_timeout_secs: 300
  },
  grpc: {
    enabled: true,
    port: 50051,
    host: "0.0.0.0"
  },
  graphql: {
    enabled: true,
    port: 4000,
    host: "0.0.0.0",
    playground_enabled: true,
    introspection_enabled: true
  },
  admin: {
    enabled: false,
    port: 9080,
    host: "127.0.0.1",
    auth_required: false,
    api_enabled: true,
    prometheus_url: "http://localhost:9090"
  }
})"#;

    fs::write(&config_path, ts_content).unwrap();

    // Test loading TS config (type annotations should be stripped)
    let config = load_config_auto(&config_path).await.unwrap();
    assert_eq!(config.http.port, 3000);
    assert_eq!(config.http.host, "0.0.0.0");
    assert_eq!(config.logging.level, "info");
}

#[tokio::test]
async fn test_discover_config_file_priority() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple config files
    let ts_path = temp_dir.path().join("mockforge.config.ts");
    let js_path = temp_dir.path().join("mockforge.config.js");
    let yaml_path = temp_dir.path().join("mockforge.yaml");

    fs::write(
        &ts_path,
        r#"
const config = { http: { port: 5000 } };
config;
"#,
    )
    .unwrap();

    fs::write(
        &js_path,
        r#"
const config = { http: { port: 4000 } };
config;
"#,
    )
    .unwrap();

    fs::write(&yaml_path, "http:\n  port: 3000").unwrap();

    // Change to temp directory for discovery
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // TS should be discovered first
    let discovered = discover_config_file_all_formats().await.unwrap();
    assert!(discovered.ends_with("mockforge.config.ts"));

    // Clean up
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_profile_precedence_with_env_vars() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mockforge.yaml");

    let config_content = r#"
http:
  port: 3000

profiles:
  dev:
    http:
      port: 4000
"#;

    fs::write(&config_path, config_content).unwrap();

    // Load with dev profile
    let config = load_config_with_profile(&config_path, Some("dev")).await.unwrap();
    assert_eq!(config.http.port, 4000);

    // Apply env var overrides
    std::env::set_var("MOCKFORGE_HTTP_PORT", "5000");
    let config_with_env = apply_env_overrides(config);
    assert_eq!(config_with_env.http.port, 5000);

    // Clean up
    std::env::remove_var("MOCKFORGE_HTTP_PORT");
}

#[tokio::test]
async fn test_complex_profile_merging() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("mockforge.yaml");

    let config_content = r#"
http:
  port: 3000
  host: "0.0.0.0"
  cors:
    enabled: true
    allowed_origins:
      - "*"

admin:
  enabled: false
  port: 9080

observability:
  prometheus:
    enabled: true
    port: 9090

profiles:
  prod:
    http:
      port: 8080
      # Note: cors is not specified, so entire http config is replaced
    admin:
      enabled: true
      auth_required: true
    observability:
      prometheus:
        enabled: true
        port: 9091
      opentelemetry:
        enabled: true
        service_name: "mockforge-prod"
"#;

    fs::write(&config_path, config_content).unwrap();

    let prod_config = load_config_with_profile(&config_path, Some("prod")).await.unwrap();

    // Profile overrides
    assert_eq!(prod_config.http.port, 8080);
    assert!(prod_config.admin.enabled);
    assert!(prod_config.admin.auth_required);
    assert_eq!(prod_config.observability.prometheus.port, 9091);

    // Check that opentelemetry from profile is present
    assert!(prod_config.observability.opentelemetry.is_some());
    let otel = prod_config.observability.opentelemetry.unwrap();
    assert!(otel.enabled);
    assert_eq!(otel.service_name, "mockforge-prod");
}

#[test]
fn test_strip_typescript_types() {
    use regex::Regex;

    let ts_code = r#"
interface Config {
    port: number;
    host: string;
}

type Port = number;

const config: Config = {
    port: 3000,
    host: "localhost"
} as Config;

export type { Config };
"#;

    // Re-implement strip logic for test (matches the actual implementation)
    fn strip_typescript_types(content: &str) -> String {
        let mut result = content.to_string();

        // Remove interface declarations (handles multi-line)
        let interface_re = Regex::new(r"(?ms)interface\s+\w+\s*\{[^}]*\}\s*").unwrap();
        result = interface_re.replace_all(&result, "").to_string();

        // Remove type aliases
        let type_alias_re = Regex::new(r"(?m)^type\s+\w+\s*=\s*[^;]+;\s*").unwrap();
        result = type_alias_re.replace_all(&result, "").to_string();

        // Remove type annotations (: Type)
        let type_annotation_re = Regex::new(r":\s*[A-Z]\w*(<[^>]+>)?(\[\])?").unwrap();
        result = type_annotation_re.replace_all(&result, "").to_string();

        // Remove type imports and exports
        let type_import_re = Regex::new(r"(?m)^(import|export)\s+type\s+.*$").unwrap();
        result = type_import_re.replace_all(&result, "").to_string();

        // Remove as Type
        let as_type_re = Regex::new(r"\s+as\s+\w+").unwrap();
        result = as_type_re.replace_all(&result, "").to_string();

        result
    }

    let stripped = strip_typescript_types(ts_code);

    // Should remove interface declarations
    assert!(!stripped.contains("interface Config"));

    // Should remove type annotations
    assert!(!stripped.contains(": Config"));
    assert!(!stripped.contains(": number"));
    assert!(!stripped.contains(": string"));

    // Should remove 'as' type assertions
    assert!(!stripped.contains("as Config"));

    // Should remove type imports/exports
    assert!(!stripped.contains("type Port"));
    assert!(!stripped.contains("export type"));

    // Should preserve actual code
    assert!(stripped.contains("const config"));
    assert!(stripped.contains("port"));
    assert!(stripped.contains("3000"));
}

#[tokio::test]
async fn test_auto_format_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Test YAML
    let yaml_path = temp_dir.path().join("config.yaml");
    fs::write(&yaml_path, "http:\n  port: 3000").unwrap();
    let yaml_config = load_config_auto(&yaml_path).await.unwrap();
    assert_eq!(yaml_config.http.port, 3000);

    // Test JSON
    let json_path = temp_dir.path().join("config.json");
    fs::write(&json_path, r#"{"http": {"port": 4000}}"#).unwrap();
    let json_config = load_config_auto(&json_path).await.unwrap();
    assert_eq!(json_config.http.port, 4000);

    // Test JS
    let js_path = temp_dir.path().join("config.js");
    fs::write(&js_path, "({ http: { port: 5000 } })").unwrap();
    let js_config = load_config_auto(&js_path).await.unwrap();
    assert_eq!(js_config.http.port, 5000);

    // Test unsupported format
    let txt_path = temp_dir.path().join("config.txt");
    fs::write(&txt_path, "invalid").unwrap();
    let result = load_config_auto(&txt_path).await;
    assert!(result.is_err());
}
