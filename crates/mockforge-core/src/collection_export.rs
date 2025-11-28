/// API Collection export functionality
///
/// Generates Postman, Insomnia, and Hoppscotch collections from OpenAPI/GraphQL schemas
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Supported collection formats for API export
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionFormat {
    /// Postman Collection v2.1 format
    Postman,
    /// Insomnia Collection format
    Insomnia,
    /// Hoppscotch Collection format
    Hoppscotch,
}

/// Postman collection v2.1 format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanCollection {
    /// Collection metadata and information
    pub info: PostmanInfo,
    /// Array of request items in the collection
    pub item: Vec<PostmanItem>,
    /// Optional collection-level variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<Vec<PostmanVariable>>,
}

/// Postman collection information/metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanInfo {
    /// Collection name
    pub name: String,
    /// Collection description
    pub description: String,
    /// Postman schema URL
    pub schema: String,
    /// Optional collection version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Postman collection item (request or folder)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanItem {
    /// Item name
    pub name: String,
    /// Request details
    pub request: PostmanRequest,
    /// Optional saved response examples
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Vec<Value>>,
}

/// Postman request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanRequest {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request headers
    pub header: Vec<PostmanHeader>,
    /// Optional request body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<PostmanBody>,
    /// Request URL structure
    pub url: PostmanUrl,
    /// Optional request description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Postman header entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanHeader {
    /// Header name
    pub key: String,
    /// Header value
    pub value: String,
    /// Header type (e.g., "text")
    #[serde(rename = "type")]
    pub header_type: String,
}

/// Postman request body structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanBody {
    /// Body mode (e.g., "raw", "formdata")
    pub mode: String,
    /// Optional raw body content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    /// Optional body options/metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
}

/// Postman URL structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanUrl {
    /// Raw URL string
    pub raw: String,
    /// URL host segments
    pub host: Vec<String>,
    /// URL path segments
    pub path: Vec<String>,
    /// Optional query parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<Vec<PostmanQueryParam>>,
}

/// Postman query parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanQueryParam {
    /// Query parameter key
    pub key: String,
    /// Query parameter value
    pub value: String,
}

/// Postman collection variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanVariable {
    /// Variable name
    pub key: String,
    /// Variable value
    pub value: String,
    /// Variable type (e.g., "string", "number")
    #[serde(rename = "type")]
    pub var_type: String,
}

/// Insomnia collection format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsomniaCollection {
    /// Resource type identifier
    pub _type: String,
    /// Export format version
    pub __export_format: u8,
    /// Export timestamp (ISO 8601)
    pub __export_date: String,
    /// Export source application
    pub __export_source: String,
    /// Collection resources (workspaces, requests, etc.)
    pub resources: Vec<InsomniaResource>,
}

/// Insomnia resource (workspace, request, folder, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsomniaResource {
    /// Unique resource identifier
    pub _id: String,
    /// Resource type (workspace, request, etc.)
    pub _type: String,
    /// Resource name
    pub name: String,
    /// HTTP method (for request resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Request URL (for request resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Request body (for request resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    /// Request headers (for request resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<InsomniaHeader>>,
}

/// Insomnia header entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsomniaHeader {
    /// Header name
    pub name: String,
    /// Header value
    pub value: String,
}

/// Hoppscotch collection format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchCollection {
    /// Collection name
    pub name: String,
    /// Array of requests in the collection
    pub requests: Vec<HoppscotchRequest>,
    /// Optional folders for organizing requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folders: Option<Vec<HoppscotchFolder>>,
}

/// Hoppscotch request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchRequest {
    /// Request name
    pub name: String,
    /// HTTP method
    pub method: String,
    /// API endpoint URL
    pub endpoint: String,
    /// Request headers
    pub headers: Vec<HoppscotchHeader>,
    /// Query parameters
    pub params: Vec<HoppscotchParam>,
    /// Request body
    pub body: HoppscotchBody,
}

/// Hoppscotch header entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchHeader {
    /// Header name
    pub key: String,
    /// Header value
    pub value: String,
    /// Whether the header is active/enabled
    pub active: bool,
}

/// Hoppscotch query parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchParam {
    /// Parameter key
    pub key: String,
    /// Parameter value
    pub value: String,
    /// Whether the parameter is active/enabled
    pub active: bool,
}

/// Hoppscotch request body
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchBody {
    /// Content-Type of the body
    pub content_type: String,
    /// Body content as string
    pub body: String,
}

/// Hoppscotch folder for organizing requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoppscotchFolder {
    /// Folder name
    pub name: String,
    /// Requests contained in this folder
    pub requests: Vec<HoppscotchRequest>,
}

/// Exports OpenAPI specifications to various API collection formats
pub struct CollectionExporter {
    /// Base URL for generated requests
    base_url: String,
}

impl CollectionExporter {
    /// Create a new collection exporter with the specified base URL
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
                        let name = op
                            .operation_id
                            .clone()
                            .or_else(|| op.summary.clone())
                            .unwrap_or_else(|| format!("{} {}", method, path));

                        let request = PostmanRequest {
                            method: method.to_string(),
                            header: vec![PostmanHeader {
                                key: "Content-Type".to_string(),
                                value: "application/json".to_string(),
                                header_type: "text".to_string(),
                            }],
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
                                path: path
                                    .split('/')
                                    .filter(|s| !s.is_empty())
                                    .map(String::from)
                                    .collect(),
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
                schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
                    .to_string(),
                version: Some(spec.spec.info.version.clone()),
            },
            item: items,
            variable: Some(vec![PostmanVariable {
                key: "baseUrl".to_string(),
                value: self.base_url.clone(),
                var_type: "string".to_string(),
            }]),
        }
    }

    /// Generate Insomnia collection from OpenAPI spec
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

                        let name = op
                            .operation_id
                            .clone()
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
                            headers: Some(vec![InsomniaHeader {
                                name: "Content-Type".to_string(),
                                value: "application/json".to_string(),
                            }]),
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
                        let name = op
                            .operation_id
                            .clone()
                            .or_else(|| op.summary.clone())
                            .unwrap_or_else(|| format!("{} {}", method, path));

                        requests.push(HoppscotchRequest {
                            name,
                            method: method.to_string(),
                            endpoint: format!("{}{}", self.base_url, path),
                            headers: vec![HoppscotchHeader {
                                key: "Content-Type".to_string(),
                                value: "application/json".to_string(),
                                active: true,
                            }],
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
                schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
                    .to_string(),
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
