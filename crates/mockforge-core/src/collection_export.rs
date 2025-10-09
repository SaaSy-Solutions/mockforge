/// API Collection export functionality
///
/// Generates Postman, Insomnia, and Hoppscotch collections from OpenAPI/GraphQL schemas
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Supported collection formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionFormat {
    Postman,
    Insomnia,
    Hoppscotch,
}

/// Postman collection v2.1 format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanCollection {
    pub info: PostmanInfo,
    pub item: Vec<PostmanItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<Vec<PostmanVariable>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanInfo {
    pub name: String,
    pub description: String,
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanItem {
    pub name: String,
    pub request: PostmanRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanRequest {
    pub method: String,
    pub header: Vec<PostmanHeader>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<PostmanBody>,
    pub url: PostmanUrl,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanHeader {
    pub key: String,
    pub value: String,
    #[serde(rename = "type")]
    pub header_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanBody {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanUrl {
    pub raw: String,
    pub host: Vec<String>,
    pub path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<Vec<PostmanQueryParam>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanQueryParam {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanVariable {
    pub key: String,
    pub value: String,
    #[serde(rename = "type")]
    pub var_type: String,
}

/// Insomnia collection format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsomniaCollection {
    pub _type: String,
    pub __export_format: u8,
    pub __export_date: String,
    pub __export_source: String,
    pub resources: Vec<InsomniaResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsomniaResource {
    pub _id: String,
    pub _type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<InsomniaHeader>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsomniaHeader {
    pub name: String,
    pub value: String,
}

/// Hoppscotch collection format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchCollection {
    pub name: String,
    pub requests: Vec<HoppscotchRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folders: Option<Vec<HoppscotchFolder>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchRequest {
    pub name: String,
    pub method: String,
    pub endpoint: String,
    pub headers: Vec<HoppscotchHeader>,
    pub params: Vec<HoppscotchParam>,
    pub body: HoppscotchBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchHeader {
    pub key: String,
    pub value: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchParam {
    pub key: String,
    pub value: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchBody {
    pub content_type: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchFolder {
    pub name: String,
    pub requests: Vec<HoppscotchRequest>,
}

/// Collection exporter
pub struct CollectionExporter {
    base_url: String,
}

impl CollectionExporter {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Generate Postman collection from OpenAPI spec
    pub fn to_postman(&self, spec: &crate::openapi::OpenApiSpec) -> PostmanCollection {
        let mut items = Vec::new();

        for (path, path_item_ref) in &spec.spec.paths.paths {
            // Unwrap ReferenceOr
            if let openapiv3::ReferenceOr::Item(path_item) = path_item_ref {
                let operations = vec![
                    ("GET", path_item.get.as_ref()),
                    ("POST", path_item.post.as_ref()),
                    ("PUT", path_item.put.as_ref()),
                    ("DELETE", path_item.delete.as_ref()),
                    ("PATCH", path_item.patch.as_ref()),
                    ("HEAD", path_item.head.as_ref()),
                    ("OPTIONS", path_item.options.as_ref()),
                ];

                for (method, op_opt) in operations {
                    if let Some(op) = op_opt {
                        let name = op.operation_id.clone()
                            .or_else(|| op.summary.clone())
                            .unwrap_or_else(|| format!("{} {}", method, path));

                        let request = PostmanRequest {
                            method: method.to_string(),
                            header: vec![
                                PostmanHeader {
                                    key: "Content-Type".to_string(),
                                    value: "application/json".to_string(),
                                    header_type: "text".to_string(),
                                }
                            ],
                            body: if matches!(method, "POST" | "PUT" | "PATCH") {
                                Some(PostmanBody {
                                    mode: "raw".to_string(),
                                    raw: Some("{}".to_string()),
                                    options: Some(serde_json::json!({
                                        "raw": {
                                            "language": "json"
                                        }
                                    })),
                                })
                            } else {
                                None
                            },
                            url: PostmanUrl {
                                raw: format!("{}{}", self.base_url, path),
                                host: vec![self.base_url.clone()],
                                path: path.split('/').filter(|s| !s.is_empty()).map(String::from).collect(),
                                query: None,
                            },
                            description: op.description.clone(),
                        };

                        items.push(PostmanItem {
                            name,
                            request,
                            response: None,
                        });
                    }
                }
            }
        }

        PostmanCollection {
            info: PostmanInfo {
                name: spec.spec.info.title.clone(),
                description: spec.spec.info.description.clone().unwrap_or_default(),
                schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json".to_string(),
                version: Some(spec.spec.info.version.clone()),
            },
            item: items,
            variable: Some(vec![
                PostmanVariable {
                    key: "baseUrl".to_string(),
                    value: self.base_url.clone(),
                    var_type: "string".to_string(),
                }
            ]),
        }
    }

    /// Generate Insomnia collection
    pub fn to_insomnia(&self, spec: &crate::openapi::OpenApiSpec) -> InsomniaCollection {
        let mut resources = Vec::new();

        // Add workspace resource
        resources.push(InsomniaResource {
            _id: "wrk_1".to_string(),
            _type: "workspace".to_string(),
            name: spec.spec.info.title.clone(),
            method: None,
            url: None,
            body: None,
            headers: None,
        });

        // Add request resources
        let mut id_counter = 1;
        for (path, path_item_ref) in &spec.spec.paths.paths {
            if let openapiv3::ReferenceOr::Item(path_item) = path_item_ref {
                let operations = vec![
                    ("GET", path_item.get.as_ref()),
                    ("POST", path_item.post.as_ref()),
                    ("PUT", path_item.put.as_ref()),
                    ("DELETE", path_item.delete.as_ref()),
                    ("PATCH", path_item.patch.as_ref()),
                    ("HEAD", path_item.head.as_ref()),
                    ("OPTIONS", path_item.options.as_ref()),
                ];

                for (method, op_opt) in operations {
                    if let Some(op) = op_opt {
                        id_counter += 1;

                        let name = op.operation_id.clone()
                            .or_else(|| op.summary.clone())
                            .unwrap_or_else(|| format!("{} {}", method, path));

                        resources.push(InsomniaResource {
                            _id: format!("req_{}", id_counter),
                            _type: "request".to_string(),
                            name,
                            method: Some(method.to_string()),
                            url: Some(format!("{}{}", self.base_url, path)),
                            body: if matches!(method, "POST" | "PUT" | "PATCH") {
                                Some(serde_json::json!({
                                    "mimeType": "application/json",
                                    "text": "{}"
                                }))
                            } else {
                                None
                            },
                            headers: Some(vec![
                                InsomniaHeader {
                                    name: "Content-Type".to_string(),
                                    value: "application/json".to_string(),
                                }
                            ]),
                        });
                    }
                }
            }
        }

        InsomniaCollection {
            _type: "export".to_string(),
            __export_format: 4,
            __export_date: chrono::Utc::now().to_rfc3339(),
            __export_source: "mockforge".to_string(),
            resources,
        }
    }

    /// Generate Hoppscotch collection
    pub fn to_hoppscotch(&self, spec: &crate::openapi::OpenApiSpec) -> HoppscotchCollection {
        let mut requests = Vec::new();

        for (path, path_item_ref) in &spec.spec.paths.paths {
            if let openapiv3::ReferenceOr::Item(path_item) = path_item_ref {
                let operations = vec![
                    ("GET", path_item.get.as_ref()),
                    ("POST", path_item.post.as_ref()),
                    ("PUT", path_item.put.as_ref()),
                    ("DELETE", path_item.delete.as_ref()),
                    ("PATCH", path_item.patch.as_ref()),
                    ("HEAD", path_item.head.as_ref()),
                    ("OPTIONS", path_item.options.as_ref()),
                ];

                for (method, op_opt) in operations {
                    if let Some(op) = op_opt {
                        let name = op.operation_id.clone()
                            .or_else(|| op.summary.clone())
                            .unwrap_or_else(|| format!("{} {}", method, path));

                        requests.push(HoppscotchRequest {
                            name,
                            method: method.to_string(),
                            endpoint: format!("{}{}", self.base_url, path),
                            headers: vec![
                                HoppscotchHeader {
                                    key: "Content-Type".to_string(),
                                    value: "application/json".to_string(),
                                    active: true,
                                }
                            ],
                            params: vec![],
                            body: HoppscotchBody {
                                content_type: "application/json".to_string(),
                                body: "{}".to_string(),
                            },
                        });
                    }
                }
            }
        }

        HoppscotchCollection {
            name: spec.spec.info.title.clone(),
            requests,
            folders: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postman_collection_structure() {
        let collection = PostmanCollection {
            info: PostmanInfo {
                name: "Test API".to_string(),
                description: "Test description".to_string(),
                schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json".to_string(),
                version: Some("1.0.0".to_string()),
            },
            item: vec![],
            variable: None,
        };

        assert_eq!(collection.info.name, "Test API");
    }

    #[test]
    fn test_insomnia_collection_structure() {
        let collection = InsomniaCollection {
            _type: "export".to_string(),
            __export_format: 4,
            __export_date: "2024-01-01T00:00:00Z".to_string(),
            __export_source: "mockforge".to_string(),
            resources: vec![],
        };

        assert_eq!(collection._type, "export");
        assert_eq!(collection.__export_format, 4);
    }

    #[test]
    fn test_hoppscotch_collection_structure() {
        let collection = HoppscotchCollection {
            name: "Test API".to_string(),
            requests: vec![],
            folders: None,
        };

        assert_eq!(collection.name, "Test API");
    }
}
