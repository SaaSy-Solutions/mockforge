//! The mock's catalog read from a file (`serve --mcp-mock-catalog`).
//!
//! The file is one of three documents:
//!
//! * a catalog, `kind: "mockforge-mcp-catalog"`: what the mock advertises and
//!   what each tool answers, written by hand or generated;
//! * a pinned contract, `kind: "mockforge-mcp-contract"`, the file
//!   `mockforge mcp-contract pin` writes. The mock then advertises what the
//!   pinned server advertised: the same tools, worded the same way, with the
//!   same instructions. An agent can be pointed at a copy of a real tool
//!   server that needs no credential and runs nothing;
//! * an agent red-team suite, `kind: "mockforge-agent-suite"`, the file
//!   `mockforge agent-redteam write` writes. Its `catalog` is served, so a
//!   hostile tool server is one file to give an agent.
//!
//! A file the mock cannot serve as written is refused with the reason. A
//! misspelt field never becomes a mock that quietly advertises something
//! else, and two tools never share a name.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use thiserror::Error;

use mockforge_contracts::agent_redteam::{AgentRedteamError, Suite, SUITE_KIND};
use mockforge_contracts::mcp_contract::{Contract, McpContractError, ToolContract, CONTRACT_KIND};

use super::{McpMockConfig, McpPrompt, McpResource, McpTool, McpToolResult};

/// The `kind` of a catalog document, so a reader can tell what it holds.
pub const CATALOG_KIND: &str = "mockforge-mcp-catalog";

/// The version of the catalog documents this version of MockForge reads.
pub const CATALOG_SCHEMA_VERSION: u32 = 1;

/// Why a file cannot be served as the mock's catalog.
#[derive(Error)]
pub enum McpCatalogError {
    /// The file could not be read.
    #[error("{path}: {reason}")]
    Read {
        /// The file that was named.
        path: String,
        /// What went wrong reading it.
        reason: String,
    },

    /// The file was read and is not a catalog or a pinned contract that can
    /// be served.
    #[error("{path}: {reason}")]
    Invalid {
        /// The file that was named.
        path: String,
        /// What is wrong with it.
        reason: String,
    },
}

/// Printed as the reason. A program that returns the error, as `mockforge`
/// does, then prints a sentence and not a structure.
impl fmt::Debug for McpCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl McpMockConfig {
    /// The catalog in a file: a catalog, or a pinned contract replayed. A file
    /// that cannot be served as written is an error, never a default.
    pub fn from_file(path: &Path) -> Result<Self, McpCatalogError> {
        let name = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|error| McpCatalogError::Read {
            path: name.clone(),
            reason: error.to_string(),
        })?;
        Self::from_json(&text, &name)
    }

    /// The catalog in a JSON document. `path` names the document in errors.
    pub fn from_json(text: &str, path: &str) -> Result<Self, McpCatalogError> {
        let document: Value = serde_json::from_str(text)
            .map_err(|error| invalid(path, &format!("not JSON: {error}")))?;
        let Some(object) = document.as_object() else {
            return Err(invalid(path, "the document is not a JSON object"));
        };
        match object.get("kind").and_then(Value::as_str) {
            Some(CATALOG_KIND) => catalog(object, path),
            // An agent red-team suite: its catalog is the server, and its task and
            // rules are for `agent-redteam judge`.
            Some(SUITE_KIND) => {
                let suite = Suite::from_json(text, path).map_err(|error| match error {
                    AgentRedteamError::Suite { reason, .. } => invalid(path, &reason),
                    other => invalid(path, &other.to_string()),
                })?;
                let Some(embedded) = suite.catalog.as_object() else {
                    return Err(invalid(path, "the suite's catalog is not an object"));
                };
                catalog(embedded, path)
            }
            Some(CONTRACT_KIND) => {
                let contract = Contract::from_json(text, path).map_err(|error| match error {
                    McpContractError::Contract { reason, .. } => invalid(path, &reason),
                    other => invalid(path, &other.to_string()),
                })?;
                replay(&contract, path)
            }
            other => Err(invalid(
                path,
                &format!(
                    "not an MCP catalog, a pinned MCP contract or an agent suite (its `kind` is {}); \
                     a catalog has the `kind` `{CATALOG_KIND}`",
                    other.unwrap_or("missing")
                ),
            )),
        }
    }
}

fn invalid(path: &str, reason: &str) -> McpCatalogError {
    McpCatalogError::Invalid {
        path: path.to_string(),
        reason: reason.trim().to_string(),
    }
}

/// The `server` section of a catalog.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogServer {
    #[serde(default = "default_server_name")]
    name: String,
    #[serde(default = "default_server_version")]
    version: String,
    #[serde(default)]
    instructions: Option<String>,
    /// `Null` when the catalog says nothing: the default claims are used.
    #[serde(default)]
    capabilities: Value,
}

/// A catalog that leaves the section out gets the same server as one that
/// leaves its fields out.
impl Default for CatalogServer {
    fn default() -> Self {
        Self {
            name: default_server_name(),
            version: default_server_version(),
            instructions: None,
            capabilities: Value::Null,
        }
    }
}

fn default_server_name() -> String {
    "mockforge-mcp".to_string()
}

fn default_server_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// One tool of a catalog: what it declares, and what the mock answers.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogTool {
    name: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    /// A tool that takes no arguments may leave this out.
    #[serde(default = "no_arguments", rename = "inputSchema")]
    input_schema: Value,
    #[serde(default, rename = "outputSchema")]
    output_schema: Option<Value>,
    #[serde(default)]
    annotations: Option<Value>,
    #[serde(default)]
    icons: Option<Value>,
    /// Text, an object, or left out; see [`answer`].
    #[serde(default)]
    result: Value,
}

fn no_arguments() -> Value {
    json!({ "type": "object" })
}

const CATALOG_FIELDS: [&str; 6] = [
    "kind",
    "schema_version",
    "server",
    "tools",
    "resources",
    "prompts",
];

fn catalog(object: &Map<String, Value>, path: &str) -> Result<McpMockConfig, McpCatalogError> {
    if let Some(key) = object.keys().find(|key| !CATALOG_FIELDS.contains(&key.as_str())) {
        return Err(invalid(
            path,
            &format!(
                "unknown field `{key}`; a catalog has only `kind`, `schema_version`, \
                 `server`, `tools`, `resources` and `prompts`"
            ),
        ));
    }
    match object.get("schema_version").and_then(Value::as_u64) {
        Some(version) if version == u64::from(CATALOG_SCHEMA_VERSION) => {}
        Some(newer) if newer > u64::from(CATALOG_SCHEMA_VERSION) => {
            return Err(invalid(
                path,
                &format!(
                    "it was written for a newer MockForge (schema_version {newer}); \
                     upgrade MockForge to serve it"
                ),
            ));
        }
        _ => return Err(invalid(path, "its `schema_version` is missing or not a number")),
    }

    let server: CatalogServer = section(object, "server", path)?;
    if server.name.trim().is_empty() {
        return Err(invalid(path, "`server.name` is empty"));
    }
    if !server.capabilities.is_null() && !server.capabilities.is_object() {
        return Err(invalid(path, "`server.capabilities` is not an object"));
    }
    let mut server_info = Map::new();
    server_info.insert("name".to_string(), json!(server.name));
    server_info.insert("version".to_string(), json!(server.version));

    let tools: Vec<Value> = section(object, "tools", path)?;
    let resources: Vec<McpResource> = section(object, "resources", path)?;
    let mut prompts: Vec<McpPrompt> = section(object, "prompts", path)?;
    for (index, prompt) in prompts.iter_mut().enumerate() {
        let arguments = std::mem::take(&mut prompt.arguments);
        prompt.arguments = distinct(
            arguments,
            path,
            &format!("prompts[{index}].arguments"),
            "name",
            |argument| &argument.name,
        )?;
    }
    Ok(McpMockConfig {
        server_info: Value::Object(server_info),
        instructions: server.instructions,
        capabilities: if server.capabilities.is_null() {
            super::default_capabilities()
        } else {
            server.capabilities
        },
        tools: catalog_tools(tools, path)?,
        resources: distinct(resources, path, "resources", "uri", |resource| &resource.uri)?,
        prompts: distinct(prompts, path, "prompts", "name", |prompt| &prompt.name)?,
        record: None,
    })
}

/// The value of a section, or its empty default when the catalog leaves it
/// out. A section that is there but malformed names the section and what is
/// wrong with it.
fn section<T: DeserializeOwned + Default>(
    object: &Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<T, McpCatalogError> {
    match object.get(key).filter(|value| !value.is_null()) {
        Some(value) => serde_json::from_value(value.clone())
            .map_err(|error| invalid(path, &format!("`{key}` is not shaped as expected: {error}"))),
        None => Ok(T::default()),
    }
}

fn catalog_tools(tools: Vec<Value>, path: &str) -> Result<Vec<McpTool>, McpCatalogError> {
    let mut first_named: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = Vec::with_capacity(tools.len());
    for (index, value) in tools.into_iter().enumerate() {
        let at = format!("tools[{index}]");
        let tool: CatalogTool = serde_json::from_value(value)
            .map_err(|error| invalid(path, &format!("{at}: {error}")))?;
        if tool.name.trim().is_empty() {
            return Err(invalid(path, &format!("{at}: its `name` is empty")));
        }
        if !tool.input_schema.is_object() {
            return Err(invalid(
                path,
                &format!("{at}: `{}` has an `inputSchema` that is not an object", tool.name),
            ));
        }
        if tool.output_schema.as_ref().is_some_and(|schema| !schema.is_object()) {
            return Err(invalid(
                path,
                &format!("{at}: `{}` has an `outputSchema` that is not an object", tool.name),
            ));
        }
        if let Some(&first) = first_named.get(&tool.name) {
            return Err(invalid(
                path,
                &format!("{at} is also named `{}`; tools[{first}] is", tool.name),
            ));
        }
        first_named.insert(tool.name.clone(), index);
        let answer = answer(&tool.result, &tool.name, &at, path)?;
        out.push(McpTool {
            declaration: as_declared(&ToolContract::from(tool)),
            result: answer,
        });
    }
    Ok(out)
}

impl From<CatalogTool> for ToolContract {
    fn from(tool: CatalogTool) -> Self {
        Self {
            name: tool.name,
            title: tool.title,
            description: tool.description,
            input_schema: tool.input_schema,
            output_schema: tool.output_schema,
            annotations: tool.annotations,
            icons: tool.icons,
            extra: BTreeMap::new(),
        }
    }
}

/// What a catalog says a tool answers:
///
/// * text, the short form of a result that is not an error;
/// * `{ "text": "...", "isError": true }`, a result flagged as an error;
/// * `{ "reflect": "argument" }`, the value of one string argument.
///
/// A tool without a `result` answers that the catalog gives it none.
fn answer(
    result: &Value,
    tool: &str,
    at: &str,
    path: &str,
) -> Result<McpToolResult, McpCatalogError> {
    let wrong = |reason: &str| invalid(path, &format!("{at}: {reason}"));
    match result {
        Value::Null => Ok(McpToolResult::text(format!(
            "This is a MockForge mock of `{tool}`; its catalog gives it no result."
        ))),
        Value::String(text) => Ok(McpToolResult::text(text.clone())),
        Value::Object(fields) => {
            if let Some(key) = fields
                .keys()
                .find(|key| !matches!(key.as_str(), "text" | "isError" | "reflect"))
            {
                return Err(wrong(&format!(
                    "`result` has no field `{key}`; it has `text` with `isError`, or `reflect`"
                )));
            }
            if let Some(reflect) = fields.get("reflect") {
                if fields.len() > 1 {
                    return Err(wrong(
                        "`result` has `reflect` and another field; it is one or the other",
                    ));
                }
                return match reflect {
                    Value::String(argument) if !argument.trim().is_empty() => {
                        Ok(McpToolResult::Reflect(argument.clone()))
                    }
                    _ => Err(wrong("`result.reflect` is not the name of an argument")),
                };
            }
            let Some(Value::String(text)) = fields.get("text") else {
                return Err(wrong(if fields.contains_key("text") {
                    "`result.text` is not text"
                } else {
                    "`result` has neither `text` nor `reflect`"
                }));
            };
            let is_error = match fields.get("isError") {
                None => false,
                Some(Value::Bool(flag)) => *flag,
                Some(_) => return Err(wrong("`result.isError` is not true or false")),
            };
            Ok(McpToolResult::Text {
                text: text.clone(),
                is_error,
            })
        }
        _ => Err(wrong("`result` is neither text nor an object")),
    }
}

/// A list whose entries each have a distinct `key`, and none empty. Two
/// resources with one uri, or two prompts with one name, cannot both be
/// reached, so neither is served.
fn distinct<T>(
    entries: Vec<T>,
    path: &str,
    section: &str,
    key: &str,
    key_of: impl Fn(&T) -> &str,
) -> Result<Vec<T>, McpCatalogError> {
    let mut first_with: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, entry) in entries.iter().enumerate() {
        let value = key_of(entry);
        if value.trim().is_empty() {
            return Err(invalid(path, &format!("{section}[{index}] has an empty `{key}`")));
        }
        match first_with.get(value) {
            Some(&first) => {
                return Err(invalid(
                    path,
                    &format!("{section}[{index}] has the `{key}` `{value}` of {section}[{first}]"),
                ));
            }
            None => first_with.insert(value, index),
        };
    }
    Ok(entries)
}

/// The mock as the pinned server: what it said about itself, and its tools as
/// it declared them. A pin records declarations, not results, so every tool
/// answers that its calls were not recorded.
fn replay(contract: &Contract, path: &str) -> Result<McpMockConfig, McpCatalogError> {
    let server = &contract.server;
    let named = server
        .implementation
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| !name.trim().is_empty());
    if !server.implementation.is_object() || !named {
        return Err(invalid(
            path,
            "the pinned server did not say what it is, so there is nothing to replay as",
        ));
    }
    if !server.capabilities.is_object() {
        return Err(invalid(
            path,
            "the pinned server declared no capabilities, so there is nothing to replay as",
        ));
    }
    let tools = contract
        .tools
        .iter()
        .map(|tool| McpTool {
            declaration: as_declared(tool),
            result: McpToolResult::text(format!(
                "This is a MockForge replay of `{}`. The pinned contract records what the server \
                 declared, not what the tool returned.",
                tool.name
            )),
        })
        .collect();
    Ok(McpMockConfig {
        server_info: server.implementation.clone(),
        instructions: server.instructions.clone(),
        capabilities: server.capabilities.clone(),
        tools,
        resources: Vec::new(),
        prompts: Vec::new(),
        record: None,
    })
}

/// A pinned tool as the server declared it: the fields of the pin, and the
/// fields the pin kept as extras, back where the server put them. What was
/// not declared is not in the declaration.
fn as_declared(tool: &ToolContract) -> Value {
    let mut declaration = Map::new();
    declaration.insert("name".to_string(), json!(tool.name));
    if let Some(title) = &tool.title {
        declaration.insert("title".to_string(), json!(title));
    }
    if let Some(description) = &tool.description {
        declaration.insert("description".to_string(), json!(description));
    }
    declaration.insert("inputSchema".to_string(), tool.input_schema.clone());
    for (key, value) in [
        ("outputSchema", &tool.output_schema),
        ("annotations", &tool.annotations),
        ("icons", &tool.icons),
    ] {
        if let Some(value) = value {
            declaration.insert(key.to_string(), value.clone());
        }
    }
    declaration.extend(tool.extra.iter().map(|(key, value)| (key.clone(), value.clone())));
    Value::Object(declaration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_contracts::mcp_contract::{ClientIdentity, Era, ServerContract};

    const PATH: &str = "catalog.json";

    fn load(text: &str) -> Result<McpMockConfig, McpCatalogError> {
        McpMockConfig::from_json(text, PATH)
    }

    /// Why the document is refused.
    fn reason(text: &str) -> String {
        match load(text) {
            Ok(_) => panic!("the document was served"),
            Err(McpCatalogError::Invalid { reason, .. }) => reason,
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    fn catalog_with(body: &str) -> String {
        format!(r#"{{ "kind": "{CATALOG_KIND}", "schema_version": 1{body} }}"#)
    }

    fn one_tool(fields: &str) -> String {
        catalog_with(&format!(r#", "tools": [{{ "name": "x"{fields}}}]"#))
    }

    fn one_resource(fields: &str) -> String {
        catalog_with(&format!(
            r#", "resources": [{{ "uri": "u", "name": "n", "description": "d", "mimeType": "m", "text": "t"{fields}}}]"#
        ))
    }

    /// A catalog with one prompt, `p`, and no arguments unless `fields` adds them.
    fn one_prompt(fields: &str) -> String {
        catalog_with(&format!(
            r#", "prompts": [{{ "name": "p", "description": "d", "text": "t"{fields}}}]"#
        ))
    }

    const RESOURCE: &str =
        r#"{ "uri": "u", "name": "n", "description": "d", "mimeType": "m", "text": "t" }"#;

    #[test]
    fn a_catalog_is_served_as_written() {
        let config = load(&catalog_with(
            r#",
              "server": { "name": "acme-tools", "version": "1.2.3", "instructions": "Be brief." },
              "tools": [{
                "name": "echo", "title": "Echo", "description": "Echo text.",
                "inputSchema": { "type": "object" },
                "outputSchema": { "type": "object" },
                "annotations": { "readOnlyHint": true },
                "icons": [{ "src": "https://example.com/e.png" }],
                "result": { "reflect": "text" }
              }, {
                "name": "boom", "result": { "text": "it failed", "isError": true }
              }, { "name": "plain", "result": "plain text" }, { "name": "silent" }],
              "resources": [{ "uri": "file:///a", "name": "A", "description": "d", "mimeType": "text/plain", "text": "hello" }],
              "prompts": [{ "name": "p", "description": "d", "arguments": [{ "name": "x", "description": "d", "required": true }], "text": "t" }]
            "#,
        ))
        .unwrap();
        assert_eq!(config.server_info, json!({ "name": "acme-tools", "version": "1.2.3" }));
        assert_eq!(config.instructions.as_deref(), Some("Be brief."));

        let echo = &config.tools[0];
        assert_eq!(
            echo.declaration,
            json!({
                "name": "echo", "title": "Echo", "description": "Echo text.",
                "inputSchema": { "type": "object" }, "outputSchema": { "type": "object" },
                "annotations": { "readOnlyHint": true },
                "icons": [{ "src": "https://example.com/e.png" }],
            })
        );
        assert_eq!(echo.name(), "echo");
        assert_eq!(echo.result, McpToolResult::Reflect("text".to_string()));
        assert_eq!(config.tools[1].result, McpToolResult::error("it failed"));
        assert_eq!(config.tools[2].result, McpToolResult::text("plain text"));
        let silent = &config.tools[3];
        assert_eq!(
            silent.declaration,
            json!({ "name": "silent", "inputSchema": { "type": "object" } }),
            "what is not declared is not advertised"
        );
        assert_eq!(
            silent.result,
            McpToolResult::text(
                "This is a MockForge mock of `silent`; its catalog gives it no result."
            )
        );
        assert_eq!(config.resources[0].uri, "file:///a");
        assert_eq!(config.prompts[0].name, "p");
        assert_eq!(config.prompts[0].arguments[0].name, "x");
    }

    #[test]
    fn every_section_can_be_left_out() {
        let config = load(&catalog_with("")).unwrap();
        let defaults = McpMockConfig::default();
        assert!(
            config.tools.is_empty() && config.resources.is_empty() && config.prompts.is_empty()
        );
        assert_eq!(config.server_info, defaults.server_info);
        assert_eq!(config.capabilities, defaults.capabilities);
        assert_eq!(config.instructions, None);
    }

    #[test]
    fn the_servers_own_claims_are_used_when_it_makes_any() {
        let claims = json!({ "tools": { "listChanged": true } });
        let config =
            load(&catalog_with(&format!(r#", "server": {{ "capabilities": {claims} }}"#))).unwrap();
        assert_eq!(config.capabilities, claims);
    }

    #[test]
    fn a_document_that_is_not_a_catalog_is_refused() {
        assert_eq!(
            reason("{}"),
            "not an MCP catalog, a pinned MCP contract or an agent suite (its `kind` is missing); \
             a catalog has the `kind` `mockforge-mcp-catalog`"
        );
        assert!(reason(r#"{"kind":"other"}"#).contains("its `kind` is other"));
        assert!(reason("[]").contains("the document is not a JSON object"));
        assert!(reason("{not json").starts_with("not JSON:"));
    }

    #[test]
    fn a_version_this_mockforge_does_not_read_is_named() {
        let newer = format!(r#"{{ "kind": "{CATALOG_KIND}", "schema_version": 2 }}"#);
        assert_eq!(
            reason(&newer),
            "it was written for a newer MockForge (schema_version 2); upgrade MockForge to serve it"
        );
        assert_eq!(
            reason(&format!(r#"{{ "kind": "{CATALOG_KIND}" }}"#)),
            "its `schema_version` is missing or not a number"
        );
        assert_eq!(
            reason(&catalog_with("").replace("\"schema_version\": 1", "\"schema_version\": 0")),
            "its `schema_version` is missing or not a number"
        );
    }

    #[test]
    fn a_field_the_catalog_does_not_have_is_refused_where_it_is() {
        assert_eq!(
            reason(&catalog_with(r#", "tolos": []"#)),
            "unknown field `tolos`; a catalog has only `kind`, `schema_version`, `server`, \
             `tools`, `resources` and `prompts`"
        );
        assert!(reason(&catalog_with(
            r#", "server": { "name": "x", "version": "1", "tips": "y" }"#
        ))
        .contains("`server` is not shaped as expected: unknown field `tips`"));
        assert!(
            reason(&one_tool(r#", "camera": "typo""#)).contains("tools[0]: unknown field `camera`")
        );
        assert!(reason(&one_tool(r#", "input_schema": {}"#))
            .contains("tools[0]: unknown field `input_schema`"));
        assert!(reason(&one_resource(r#", "extra": 1"#)).contains("unknown field `extra`"));
        assert!(reason(&one_resource(r#", "mimetype": "m""#)).contains("unknown field `mimetype`"));
        assert!(reason(&one_prompt(r#", "extra": 1"#)).contains("unknown field `extra`"));
        assert!(reason(&one_prompt(
            r#", "arguments": [{ "name": "x", "description": "d", "required": true, "tip": 1 }]"#
        ))
        .contains("unknown field `tip`"));
    }

    #[test]
    fn a_tool_needs_a_name_and_object_schemas() {
        assert!(reason(&catalog_with(r#", "tools": [{ "description": "no name" }]"#))
            .contains("tools[0]: missing field `name`"));
        assert!(reason(&one_tool(r#", "inputSchema": "string""#))
            .contains("tools[0]: `x` has an `inputSchema` that is not an object"));
        assert!(reason(&one_tool(r#", "outputSchema": []"#))
            .contains("tools[0]: `x` has an `outputSchema` that is not an object"));
        assert_eq!(
            reason(&catalog_with(r#", "tools": [{ "name": "  " }]"#)),
            "tools[0]: its `name` is empty"
        );
        assert_eq!(
            reason(&catalog_with(r#", "server": { "name": "" }"#)),
            "`server.name` is empty"
        );
        assert_eq!(
            reason(&catalog_with(r#", "server": { "capabilities": [] }"#)),
            "`server.capabilities` is not an object"
        );
        assert!(reason(&catalog_with(r#", "tools": "none""#))
            .contains("`tools` is not shaped as expected"));
    }

    #[test]
    fn nothing_shares_a_name() {
        assert_eq!(
            reason(&catalog_with(
                r#", "tools": [{ "name": "echo" }, { "name": "add" }, { "name": "echo" }]"#
            )),
            "tools[2] is also named `echo`; tools[0] is"
        );
        assert_eq!(
            reason(&catalog_with(&format!(r#", "resources": [{RESOURCE}, {RESOURCE}]"#))),
            "resources[1] has the `uri` `u` of resources[0]"
        );
        let prompt = r#"{ "name": "p", "description": "d", "text": "t" }"#;
        assert_eq!(
            reason(&catalog_with(&format!(r#", "prompts": [{prompt}, {prompt}]"#))),
            "prompts[1] has the `name` `p` of prompts[0]"
        );
        assert_eq!(
            reason(&catalog_with(
                r#", "prompts": [{ "name": "p", "description": "d", "text": "t", "arguments": [{ "name": "x", "description": "d", "required": true }, { "name": "x", "description": "d", "required": false }] }]"#,
            )),
            "prompts[0].arguments[1] has the `name` `x` of prompts[0].arguments[0]"
        );
        assert_eq!(
            reason(&catalog_with(
                r#", "resources": [{ "uri": " ", "name": "n", "description": "d", "mimeType": "m", "text": "t" }]"#
            )),
            "resources[0] has an empty `uri`"
        );
        assert_eq!(
            reason(&catalog_with(r#", "tools": [{ "name": "a" }, { "name": "" }]"#)),
            "tools[1]: its `name` is empty"
        );
        // A prompt takes arguments or none.
        load(&one_prompt("")).unwrap();
    }

    #[test]
    fn a_result_is_text_a_flagged_text_or_an_argument() {
        let with = |result: &str| one_tool(&format!(r#", "result": {result}"#));
        for (result, expected) in [
            (r#""text""#, McpToolResult::text("text")),
            (r#"{ "text": "t" }"#, McpToolResult::text("t")),
            (r#"{ "text": "t", "isError": false }"#, McpToolResult::text("t")),
            (r#"{ "text": "t", "isError": true }"#, McpToolResult::error("t")),
            (r#"{ "reflect": "q" }"#, McpToolResult::Reflect("q".to_string())),
        ] {
            assert_eq!(load(&with(result)).unwrap().tools[0].result, expected, "{result}");
        }
        for (result, expected) in [
            (r#"7"#, "tools[0]: `result` is neither text nor an object"),
            (r#"[]"#, "tools[0]: `result` is neither text nor an object"),
            (
                r#"{ "txt": "t" }"#,
                "tools[0]: `result` has no field `txt`; it has `text` with `isError`, or `reflect`",
            ),
            (r#"{}"#, "tools[0]: `result` has neither `text` nor `reflect`"),
            (r#"{ "text": 7 }"#, "tools[0]: `result.text` is not text"),
            (
                r#"{ "text": "t", "isError": "yes" }"#,
                "tools[0]: `result.isError` is not true or false",
            ),
            (
                r#"{ "reflect": "" }"#,
                "tools[0]: `result.reflect` is not the name of an argument",
            ),
            (
                r#"{ "reflect": "q", "isError": true }"#,
                "tools[0]: `result` has `reflect` and another field; it is one or the other",
            ),
        ] {
            assert_eq!(reason(&with(result)), expected, "{result}");
        }
    }

    fn server(
        implementation: Value,
        capabilities: Value,
        instructions: Option<&str>,
    ) -> ServerContract {
        ServerContract {
            url: "https://tools.example.com/mcp".to_string(),
            era: Era::Modern,
            protocol_version: "2026-07-28".to_string(),
            implementation,
            capabilities,
            instructions: instructions.map(str::to_string),
            headers: BTreeMap::new(),
            auth: None,
        }
    }

    fn client() -> ClientIdentity {
        ClientIdentity {
            name: "agent".to_string(),
            version: "1".to_string(),
        }
    }

    #[test]
    fn a_pinned_contract_is_replayed_as_the_server_that_was_pinned() {
        let pinned_server = server(
            json!({ "name": "acme", "version": "9.9.9", "title": "Acme" }),
            json!({ "tools": { "listChanged": true }, "logging": {} }),
            Some("Prefer `search`."),
        );
        let original = Contract::pin(
            pinned_server.clone(),
            client(),
            vec![
                json!({
                    "name": "search", "title": "Search", "description": "Find documents.",
                    "inputSchema": { "type": "object", "properties": { "q": { "type": "string" } } },
                    "outputSchema": { "type": "object" }, "annotations": { "readOnlyHint": true },
                    "x-vendor": "kept",
                }),
                json!({ "name": "bare", "inputSchema": { "type": "object" } }),
            ],
            "0.0.0",
            "2026-11-24T00:00:00Z",
        )
        .unwrap();

        let replayed = load(&serde_json::to_string(&original).unwrap()).unwrap();
        assert_eq!(
            replayed.server_info,
            json!({ "name": "acme", "version": "9.9.9", "title": "Acme" })
        );
        assert_eq!(replayed.instructions.as_deref(), Some("Prefer `search`."));
        assert_eq!(
            replayed.capabilities,
            json!({ "tools": { "listChanged": true }, "logging": {} })
        );
        assert!(replayed.resources.is_empty() && replayed.prompts.is_empty());

        // A pin sorts its tools by name, so `bare` comes first.
        assert_eq!(replayed.tools.len(), 2);
        assert_eq!(
            replayed.tools[1].declaration,
            json!({
                "name": "search", "title": "Search", "description": "Find documents.",
                "inputSchema": { "type": "object", "properties": { "q": { "type": "string" } } },
                "outputSchema": { "type": "object" }, "annotations": { "readOnlyHint": true },
                "x-vendor": "kept",
            })
        );
        // A tool the server described without a description is served without one.
        assert_eq!(
            replayed.tools[0].declaration,
            json!({ "name": "bare", "inputSchema": { "type": "object" } })
        );
        assert_eq!(replayed.tools[1].name(), "search");
        let McpToolResult::Text { text, is_error } = &replayed.tools[1].result else {
            panic!("a replayed tool answers with text");
        };
        assert!(!is_error);
        assert_eq!(
            text,
            "This is a MockForge replay of `search`. The pinned contract records what the server \
             declared, not what the tool returned."
        );

        // Pinned again, by the same client, the replay is the same contract.
        let again = Contract::pin(
            pinned_server,
            client(),
            replayed.tools.iter().map(|tool| tool.declaration.clone()).collect(),
            "0.0.0",
            "2026-11-25T00:00:00Z",
        )
        .unwrap();
        assert_eq!(again.digest, original.digest);
        assert_eq!(again.tools, original.tools);
    }

    fn pinned(implementation: Value, capabilities: Value) -> String {
        serde_json::to_string(
            &Contract::pin(
                server(implementation, capabilities, None),
                client(),
                vec![],
                "0.0.0",
                "2026-11-24T00:00:00Z",
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn a_pin_that_does_not_say_what_the_server_is_cannot_be_replayed() {
        load(&pinned(json!({ "name": "acme" }), json!({}))).unwrap();
        assert_eq!(
            reason(&pinned(Value::Null, json!({}))),
            "the pinned server did not say what it is, so there is nothing to replay as"
        );
        assert_eq!(
            reason(&pinned(json!({ "version": "1" }), json!({}))),
            "the pinned server did not say what it is, so there is nothing to replay as"
        );
        assert_eq!(
            reason(&pinned(json!({ "name": "acme" }), Value::Null)),
            "the pinned server declared no capabilities, so there is nothing to replay as"
        );
    }

    #[test]
    fn a_pin_that_was_edited_after_pinning_is_refused() {
        let text = pinned(json!({ "name": "acme" }), json!({}));
        load(&text).unwrap();
        let mut edited: Value = serde_json::from_str(&text).unwrap();
        edited["server"]["instructions"] = json!("added after pinning");
        assert!(reason(&serde_json::to_string(&edited).unwrap()).contains(
            "its digest does not match its content, so it changed after pinning; pin it again"
        ));
    }

    #[test]
    fn the_shipped_example_is_served_as_documented() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/mcp-mock-catalog.json");
        let config = McpMockConfig::from_file(&path).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(config.server_info, json!({ "name": "acme-tools", "version": "1.2.3" }));
        assert!(config.instructions.as_deref().is_some_and(|text| !text.is_empty()));
        let names: Vec<&str> = config.tools.iter().map(McpTool::name).collect();
        assert_eq!(names, ["search", "count", "fetch_page"]);
        assert_eq!(config.tools[0].result, McpToolResult::Reflect("q".to_string()));
        assert_eq!(config.tools[1].result, McpToolResult::text("7 rows"));
        assert_eq!(config.tools[2].result, McpToolResult::error("upstream timed out"));
        assert_eq!(config.tools[0].declaration["annotations"], json!({ "readOnlyHint": true }));
        assert_eq!((config.resources.len(), config.prompts.len()), (1, 1));
    }

    #[test]
    fn an_agent_suite_is_served_by_its_catalog() {
        let suite = mockforge_contracts::agent_redteam::shipped("tool-poisoning").unwrap();
        let config = load(suite).unwrap();
        assert_eq!(config.server_info["name"], "oncall-notes");
        assert_eq!(
            config.instructions.as_deref(),
            Some("The on-call notes are internal. Answer from them and name the rotation.")
        );
        let names: Vec<&str> = config.tools.iter().map(McpTool::name).collect();
        assert_eq!(names, ["search", "get_service"]);
        // The attack is in what is served.
        assert!(config.tools[0].declaration["description"]
            .as_str()
            .unwrap()
            .contains("MF-CANARY-7F3A"));
        assert!(config.record.is_none());

        // A suite that cannot be run is refused with what is wrong with it.
        let refused =
            load(r#"{ "kind": "mockforge-agent-suite", "schema_version": 1 }"#).unwrap_err();
        assert!(
            refused
                .to_string()
                .contains("a field is missing or mistyped: missing field `id`"),
            "{refused}"
        );
    }

    #[test]
    fn a_file_that_cannot_be_read_says_so() {
        let error = McpMockConfig::from_file(Path::new("no-such-catalog.json")).unwrap_err();
        let McpCatalogError::Read { path, reason } = error else {
            panic!("expected a read error, got {error:?}");
        };
        assert_eq!(path, "no-such-catalog.json");
        assert!(!reason.is_empty());
    }
}
