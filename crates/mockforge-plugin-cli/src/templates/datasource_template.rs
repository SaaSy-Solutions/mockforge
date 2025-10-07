//! DataSource plugin template

pub const DATASOURCE_TEMPLATE: &str = r#"//! {{plugin_name}} - DataSource Plugin
//!
//! This plugin provides custom data source integration for MockForge.

use mockforge_plugin_sdk::prelude::*;

#[derive(Debug)]
pub struct Plugin {
    config: Option<serde_json::Value>,
    connection: Option<DataConnection>,
}

impl Default for Plugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin {
    pub fn new() -> Self {
        Self {
            config: None,
            connection: None,
        }
    }
}

#[async_trait]
impl DataSourcePlugin for Plugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            name: "{{plugin_name}}".to_string(),
            version: "0.1.0".to_string(),
            description: "Custom data source plugin".to_string(),
        }
    }

    async fn initialize(&mut self, config: serde_json::Value) -> PluginResult<()> {
        self.config = Some(config);
        Ok(())
    }

    async fn connect(&mut self, context: &PluginContext) -> PluginResult<DataConnection> {
        // TODO: Implement your connection logic here

        // Example: Create a mock connection
        let connection = DataConnection {
            id: format!("conn-{}", Uuid::new_v4()),
            source_type: "{{plugin_name}}".to_string(),
            connected: true,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("plugin_version".to_string(), json!("0.1.0"));
                meta.insert("connected_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));
                meta
            },
        };

        self.connection = Some(connection.clone());
        Ok(connection)
    }

    async fn query(&self, context: &PluginContext, query: &DataQuery) -> PluginResult<DataResult> {
        // TODO: Implement your query logic here

        // Ensure we're connected
        if self.connection.is_none() {
            return Err(PluginError::InvalidState("Not connected".to_string()));
        }

        // Example: Handle different query types
        let rows = match query.query.as_str() {
            "SELECT * FROM users" | "users" => {
                vec![
                    DataRow {
                        values: {
                            let mut vals = HashMap::new();
                            vals.insert("id".to_string(), json!(1));
                            vals.insert("name".to_string(), json!("Alice"));
                            vals.insert("email".to_string(), json!("alice@example.com"));
                            vals
                        },
                    },
                    DataRow {
                        values: {
                            let mut vals = HashMap::new();
                            vals.insert("id".to_string(), json!(2));
                            vals.insert("name".to_string(), json!("Bob"));
                            vals.insert("email".to_string(), json!("bob@example.com"));
                            vals
                        },
                    },
                ]
            }
            "SELECT * FROM products" | "products" => {
                vec![
                    DataRow {
                        values: {
                            let mut vals = HashMap::new();
                            vals.insert("id".to_string(), json!(1));
                            vals.insert("name".to_string(), json!("Widget"));
                            vals.insert("price".to_string(), json!(9.99));
                            vals
                        },
                    },
                    DataRow {
                        values: {
                            let mut vals = HashMap::new();
                            vals.insert("id".to_string(), json!(2));
                            vals.insert("name".to_string(), json!("Gadget"));
                            vals.insert("price".to_string(), json!(19.99));
                            vals
                        },
                    },
                ]
            }
            _ => {
                vec![DataRow {
                    values: {
                        let mut vals = HashMap::new();
                        vals.insert("error".to_string(), json!("Unknown query"));
                        vals.insert("query".to_string(), json!(query.query.clone()));
                        vals
                    },
                }]
            }
        };

        // Apply limit from query
        let mut limited_rows = rows;
        if let Some(limit) = query.limit {
            limited_rows.truncate(limit);
        }

        Ok(DataResult {
            rows: limited_rows.clone(),
            total_count: limited_rows.len(),
            columns: self.get_columns_for_query(&query.query),
        })
    }

    async fn get_schema(&self, context: &PluginContext) -> PluginResult<Schema> {
        // TODO: Implement schema discovery

        let tables = vec![
            TableInfo {
                name: "users".to_string(),
                columns: vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        data_type: "integer".to_string(),
                        nullable: false,
                    },
                    ColumnInfo {
                        name: "name".to_string(),
                        data_type: "string".to_string(),
                        nullable: false,
                    },
                    ColumnInfo {
                        name: "email".to_string(),
                        data_type: "string".to_string(),
                        nullable: true,
                    },
                ],
            },
            TableInfo {
                name: "products".to_string(),
                columns: vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        data_type: "integer".to_string(),
                        nullable: false,
                    },
                    ColumnInfo {
                        name: "name".to_string(),
                        data_type: "string".to_string(),
                        nullable: false,
                    },
                    ColumnInfo {
                        name: "price".to_string(),
                        data_type: "number".to_string(),
                        nullable: false,
                    },
                ],
            },
        ];

        Ok(Schema { tables })
    }

    async fn test_connection(&self, context: &PluginContext) -> PluginResult<bool> {
        // TODO: Implement connection test
        // Example: Always return true for mock data source
        Ok(true)
    }

    async fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()> {
        if !config.is_object() {
            return Err(PluginError::ConfigError(
                "Configuration must be an object".to_string()
            ));
        }
        Ok(())
    }

    async fn cleanup(&mut self) -> PluginResult<()> {
        self.connection = None;
        self.config = None;
        Ok(())
    }
}

impl Plugin {
    fn get_columns_for_query(&self, query: &str) -> Vec<ColumnInfo> {
        match query {
            q if q.contains("users") => vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "integer".to_string(),
                    nullable: false,
                },
                ColumnInfo {
                    name: "name".to_string(),
                    data_type: "string".to_string(),
                    nullable: false,
                },
                ColumnInfo {
                    name: "email".to_string(),
                    data_type: "string".to_string(),
                    nullable: true,
                },
            ],
            q if q.contains("products") => vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "integer".to_string(),
                    nullable: false,
                },
                ColumnInfo {
                    name: "name".to_string(),
                    data_type: "string".to_string(),
                    nullable: false,
                },
                ColumnInfo {
                    name: "price".to_string(),
                    data_type: "number".to_string(),
                    nullable: false,
                },
            ],
            _ => vec![],
        }
    }
}

// Export the plugin
export_plugin!(Plugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect() {
        let mut plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let result = plugin.connect(&context).await;

        assert_plugin_ok!(result);
        if let Ok(connection) = result {
            assert!(connection.connected);
            assert_eq!(connection.source_type, "{{plugin_name}}");
        }
    }

    #[tokio::test]
    async fn test_query_users() {
        let mut plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        // Connect first
        plugin.connect(&context).await.unwrap();

        let query = DataQuery {
            query: "SELECT * FROM users".to_string(),
            parameters: HashMap::new(),
            limit: None,
            offset: None,
        };

        let result = plugin.query(&context, &query).await;

        assert_plugin_ok!(result);
        if let Ok(data_result) = result {
            assert_eq!(data_result.rows.len(), 2);
            assert_eq!(data_result.total_count, 2);
            assert_eq!(data_result.columns.len(), 3);
        }
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let mut plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        plugin.connect(&context).await.unwrap();

        let query = DataQuery {
            query: "SELECT * FROM users".to_string(),
            parameters: HashMap::new(),
            limit: Some(1),
            offset: None,
        };

        let result = plugin.query(&context, &query).await;

        assert_plugin_ok!(result);
        if let Ok(data_result) = result {
            assert_eq!(data_result.rows.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_get_schema() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let result = plugin.get_schema(&context).await;

        assert_plugin_ok!(result);
        if let Ok(schema) = result {
            assert_eq!(schema.tables.len(), 2);
            assert!(schema.tables.iter().any(|t| t.name == "users"));
            assert!(schema.tables.iter().any(|t| t.name == "products"));
        }
    }

    #[tokio::test]
    async fn test_connection_test() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let result = plugin.test_connection(&context).await;

        assert_plugin_ok!(result);
        if let Ok(connected) = result {
            assert!(connected);
        }
    }
}
"#;
