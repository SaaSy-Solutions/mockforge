//! Svelte Client Generator Plugin
//!
//! Generates Svelte stores and TypeScript types from OpenAPI specifications
//! for easy integration with Svelte applications.

use crate::client_generator::{
    ClientGenerationResult, ClientGeneratorConfig, ClientGeneratorPlugin, GeneratedFile,
    GenerationMetadata, OpenApiSpec,
};
use crate::types::{PluginError, PluginMetadata, Result};
use handlebars::Handlebars;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Svelte client generator plugin
pub struct SvelteClientGenerator {
    /// Template registry for code generation
    templates: Handlebars<'static>,
}

impl SvelteClientGenerator {
    /// Create a new Svelte client generator
    pub fn new() -> Result<Self> {
        let mut templates = Handlebars::new();

        // Register templates for Svelte code generation
        Self::register_templates(&mut templates)?;

        Ok(Self { templates })
    }

    /// Register Handlebars templates for Svelte code generation
    fn register_templates(templates: &mut Handlebars<'static>) -> Result<()> {
        // TypeScript types template
        templates
            .register_template_string(
                "types",
                r#"// Generated TypeScript types for {{api_title}}
// API Version: {{api_version}}

{{#each schemas}}
export interface {{@key}} {
{{#each this.properties}}
  {{#if (lookup ../this.required @key)}}
  {{@key}}: {{> typescript_type this}};
  {{else}}
  {{@key}}?: {{> typescript_type this}};
  {{/if}}
{{/each}}
}

{{/each}}

// API Response types
{{#each operations}}
export interface {{operation_id}}Response {
{{#each responses}}
{{#if (eq @key "200")}}
{{#if this.content}}
{{#each this.content}}
{{#if (eq @key "application/json")}}
{{#if this.schema}}
{{#if this.schema.properties}}
{{#each this.schema.properties}}
  {{@key}}{{#unless (lookup ../this.schema.required @key)}}?{{/unless}}: {{> typescript_type this}};
{{/each}}
{{else}}
{{> typescript_type this.schema}}
{{/if}}
{{/if}}
{{/if}}
{{/each}}
{{/if}}
{{/if}}
{{/each}}
}

{{/each}}

// API Request types
{{#each operations}}
{{#if request_body}}
export interface {{operation_id}}Request {
{{#each request_body.content}}
{{#if (eq @key "application/json")}}
{{#if this.schema}}
{{> typescript_type this.schema}}
{{/if}}
{{/if}}
{{/each}}
}

{{/if}}
{{/each}}"#,
            )
            .map_err(|e| {
                PluginError::execution(format!("Failed to register types template: {}", e))
            })?;

        // Svelte stores template
        templates.register_template_string(
            "stores",
            r#"// Generated Svelte stores for {{api_title}}
// API Version: {{api_version}}

import { writable, derived, type Writable } from 'svelte/store';

// Base API configuration
export interface ApiConfig {
  baseUrl: string;
  headers?: Record<string, string>;
}

// Default API configuration
const defaultConfig: ApiConfig = {
  baseUrl: '{{base_url}}',
  headers: {
    'Content-Type': 'application/json',
  },
};

// Global configuration store
export const apiConfig: Writable<ApiConfig> = writable(defaultConfig);

// Generic API client
class ApiClient {
  private config: ApiConfig;

  constructor(config: ApiConfig) {
    this.config = config;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.config.baseUrl}${endpoint}`;
    const response = await fetch(url, {
      ...options,
      headers: {
        ...this.config.headers,
        ...options.headers,
      },
    });

    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

{{#each operations}}
  // {{summary}}
  async {{operation_id}}({{#if request_body}}data?: {{operation_id}}Request{{/if}}): Promise<{{operation_id}}Response> {
    const endpoint = '{{path}}'{{#if (eq method "GET")}}{{#if request_body}} + '?' + new URLSearchParams(data as any).toString(){{/if}}{{/if}};

    return this.request<{{operation_id}}Response>(endpoint, {
      method: '{{method}}',
      {{#if request_body}}{{#unless (eq method "GET")}}body: JSON.stringify(data),{{/unless}}{{/if}}
    });
  }

{{/each}}
}

// Create API client instance
let apiClient: ApiClient;

// Update API client when config changes
apiConfig.subscribe(config => {
  apiClient = new ApiClient(config);
});

// Initialize with default config
apiClient = new ApiClient(defaultConfig);

// Svelte stores for each operation
{{#each operations}}
export const {{operation_id}}Store = (() => {
  const data = writable<{{operation_id}}Response | null>(null);
  const loading = writable<boolean>(false);
  const error = writable<Error | null>(null);

  const execute = async ({{#if request_body}}requestData?: {{operation_id}}Request{{/if}}) => {
    loading.set(true);
    error.set(null);

    try {
      const response = await apiClient.{{operation_id}}({{#if request_body}}requestData{{/if}});
      data.set(response);
    } catch (err) {
      error.set(err as Error);
    } finally {
      loading.set(false);
    }
  };

  {{#if (eq method "GET")}}
  // Auto-execute for GET requests
  execute();
  {{/if}}

  return {
    data: derived(data, $data => $data),
    loading: derived(loading, $loading => $loading),
    error: derived(error, $error => $error),
    {{#unless (eq method "GET")}}execute,{{/unless}}
    {{#if (eq method "GET")}}refresh: execute,{{/if}}
  };
})();

{{/each}}

// Export the API client for direct use
export { apiClient };

// Export types
export * from './types';"#,
        ).map_err(|e| PluginError::execution(format!("Failed to register stores template: {}", e)))?;

        // Svelte component template
        templates
            .register_template_string(
                "component",
                r#"<!-- Generated Svelte component for {{api_title}} -->
<!-- API Version: {{api_version}} -->

<script lang="ts">
  import \{ onMount \} from 'svelte';
  import \{ {{{operation_id}}}Store \} from './stores';
  import type \{ {{{operation_id}}}Response \} from './types';

  // Reactive variables using store subscriptions
  let data: any = null;
  let loading: boolean = false;
  let error: Error | null = null;

  // Subscribe to stores
  {{{operation_id}}}Store.data.subscribe(value => data = value);
  {{{operation_id}}}Store.loading.subscribe(value => loading = value);
  {{{operation_id}}}Store.error.subscribe(value => error = value);

  // Component logic
  function handleRefresh() \{
    {{{operation_id}}}Store.refresh();
  \}
</script>

<div class="api-component">
  <h2>{{api_title}} API</h2>

  <div class="controls">
    <button on:click=\{handleRefresh\} disabled=\{loading\}>
      \{loading ? 'Loading...' : 'Refresh'\}
    </button>
  </div>

  \{#if loading\}
    <div class="loading">Loading...</div>
  \{:else if error\}
    <div class="error">Error: \{error.message\}</div>
  \{:else if data\}
    <div class="data">
      <pre>\{JSON.stringify(data, null, 2)\}</pre>
    </div>
  \{:else\}
    <div class="no-data">No data available</div>
  \{/if\}
</div>

<style>
  .api-component \{
    padding: 1rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    margin: 1rem 0;
  \}

  .controls \{
    margin-bottom: 1rem;
  \}

  .loading \{
    color: #666;
    font-style: italic;
  \}

  .error \{
    color: #d32f2f;
    background-color: #ffebee;
    padding: 0.5rem;
    border-radius: 4px;
  \}

  .data \{
    background-color: #f5f5f5;
    padding: 1rem;
    border-radius: 4px;
    overflow-x: auto;
  \}

  .no-data \{
    color: #666;
    font-style: italic;
  \}

  button \{
    padding: 0.5rem 1rem;
    background-color: #1976d2;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  \}

  button:disabled \{
    background-color: #ccc;
    cursor: not-allowed;
  \}
</style>"#,
            )
            .map_err(|e| {
                PluginError::execution(format!("Failed to register component template: {}", e))
            })?;

        // TypeScript type helper template
        templates.register_template_string(
            "typescript_type",
            r#"{{#if (eq type "string")}}string{{/if}}{{#if (eq type "integer")}}number{{/if}}{{#if (eq type "number")}}number{{/if}}{{#if (eq type "boolean")}}boolean{{/if}}{{#if (eq type "array")}}{{#if items}}{{> typescript_type items}}[]{{else}}any[]{{/if}}{{/if}}{{#if (eq type "object")}}{{#if properties}}{ {{#each properties}}{{@key}}: {{> typescript_type this}}{{#unless @last}}, {{/unless}}{{/each}} }{{else}}Record<string, any>{{/if}}{{/if}}{{#unless type}}any{{/unless}}"#,
        ).map_err(|e| PluginError::execution(format!("Failed to register typescript_type template: {}", e)))?;

        Ok(())
    }

    /// Generate Svelte client code from OpenAPI specification
    fn generate_svelte_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult> {
        let mut files = Vec::new();
        let warnings = Vec::new();

        // Prepare template context
        let context = self.prepare_template_context(spec, config)?;

        // Generate TypeScript types
        let types_content = self.templates.render("types", &context).map_err(|e| {
            PluginError::execution(format!("Failed to render types template: {}", e))
        })?;

        files.push(GeneratedFile {
            path: "types.ts".to_string(),
            content: types_content,
            file_type: "typescript".to_string(),
        });

        // Generate Svelte stores
        let stores_content = self.templates.render("stores", &context).map_err(|e| {
            PluginError::execution(format!("Failed to render stores template: {}", e))
        })?;

        files.push(GeneratedFile {
            path: "stores.ts".to_string(),
            content: stores_content,
            file_type: "typescript".to_string(),
        });

        // Generate example Svelte component
        let component_content = self.templates.render("component", &context).map_err(|e| {
            PluginError::execution(format!("Failed to render component template: {}", e))
        })?;

        files.push(GeneratedFile {
            path: "ApiComponent.svelte".to_string(),
            content: component_content,
            file_type: "svelte".to_string(),
        });

        // Generate package.json for the client
        let package_json = self.generate_package_json(spec, config)?;
        files.push(GeneratedFile {
            path: "package.json".to_string(),
            content: package_json,
            file_type: "json".to_string(),
        });

        // Generate README
        let readme = self.generate_readme(spec, config)?;
        files.push(GeneratedFile {
            path: "README.md".to_string(),
            content: readme,
            file_type: "markdown".to_string(),
        });

        let metadata = GenerationMetadata {
            framework: "svelte".to_string(),
            client_name: format!("{}-client", spec.info.title.to_lowercase().replace(' ', "-")),
            api_title: spec.info.title.clone(),
            api_version: spec.info.version.clone(),
            operation_count: self.count_operations(spec),
            schema_count: self.count_schemas(spec),
        };

        Ok(ClientGenerationResult {
            files,
            warnings,
            metadata,
        })
    }

    /// Prepare template context from OpenAPI spec
    fn prepare_template_context(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<Value> {
        let mut operations = Vec::new();
        let mut schemas = HashMap::new();

        // Process operations
        for (path, path_item) in &spec.paths {
            for (method, operation) in &path_item.operations {
                let normalized_op =
                    crate::client_generator::helpers::normalize_operation(method, path, operation);

                operations.push(json!({
                    "method": normalized_op.method,
                    "path": normalized_op.path,
                    "operation_id": normalized_op.operation_id,
                    "summary": normalized_op.summary,
                    "description": normalized_op.description,
                    "parameters": normalized_op.parameters,
                    "request_body": normalized_op.request_body,
                    "responses": normalized_op.responses,
                    "tags": normalized_op.tags,
                }));
            }
        }

        // Process schemas
        if let Some(components) = &spec.components {
            if let Some(spec_schemas) = &components.schemas {
                for (name, schema) in spec_schemas {
                    schemas.insert(name.clone(), schema.clone());
                }
            }
        }

        Ok(json!({
            "api_title": spec.info.title,
            "api_version": spec.info.version,
            "api_description": spec.info.description,
            "base_url": config.base_url.as_ref().unwrap_or(&"http://localhost:3000".to_string()),
            "operations": operations,
            "schemas": schemas,
        }))
    }

    /// Generate package.json for the Svelte client
    fn generate_package_json(
        &self,
        spec: &OpenApiSpec,
        _config: &ClientGeneratorConfig,
    ) -> Result<String> {
        let package_name = format!("{}-client", spec.info.title.to_lowercase().replace(' ', "-"));

        let package_json = json!({
            "name": package_name,
            "version": "1.0.0",
            "description": format!("Svelte client for {}", spec.info.title),
            "main": "stores.ts",
            "types": "types.ts",
            "scripts": {
                "build": "rollup -c",
                "dev": "rollup -c -w",
                "start": "sirv public --no-cors"
            },
            "dependencies": {
                "svelte": "^4.0.0"
            },
            "devDependencies": {
                "@rollup/plugin-commonjs": "^17.0.0",
                "@rollup/plugin-node-resolve": "^11.0.0",
                "rollup": "^2.0.0",
                "rollup-plugin-css-only": "^3.0.0",
                "rollup-plugin-livereload": "^2.0.0",
                "rollup-plugin-svelte": "^7.0.0",
                "sirv-cli": "^2.0.0",
                "typescript": "^5.0.0"
            },
            "peerDependencies": {
                "svelte": ">=3.0.0"
            }
        });

        serde_json::to_string_pretty(&package_json)
            .map_err(|e| PluginError::execution(format!("Failed to serialize package.json: {}", e)))
    }

    /// Generate README for the Svelte client
    fn generate_readme(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<String> {
        let default_url = "http://localhost:3000".to_string();
        let base_url = config.base_url.as_ref().unwrap_or(&default_url);
        let operations_list = self.generate_operations_list(spec);

        let readme = format!(
            r#"# {} Svelte Client

Generated Svelte client for {} API (v{}).

## Installation

```bash
npm install
```

## Usage

### Using Svelte Stores

```svelte
<script>
  import {{ createGetUsersStore }} from './store';

  const {{ data, loading, error, refresh }} = createGetUsersStore();
</script>

<div>
  {{{{#if $loading}}}}
    <div>Loading...</div>
  {{{{:else if $error}}}}
    <div>Error: {{{{ $error.message }}}}</div>
  {{{{:else if $data}}}}
    <div>
      {{{{#each $data as user (user.id)}}}}
        <div>{{{{user.name}}}}</div>
      {{{{/each}}}}
    </div>
  {{{{/if}}}}
  <button on:click={{refresh}}>Refresh</button>
</div>
```

### Using Svelte Component

```svelte
<script lang="ts">
  import {{ {}Component }} from './{}.svelte';
</script>

<{}Component operationId="getUsers" />
```

### Using API Client Directly

```typescript
import {{ apiClient }} from './store';

async function fetchData() {{
  try {{
    const users = await apiClient.getUsers();
    console.log(users);
  }} catch (error) {{
    console.error('API Error:', error);
  }}
}}
```

## Configuration

The client is configured to use the following base URL: `{}`

You can modify the configuration by updating the `defaultConfig` object in `store.ts`.

## Generated Files

- `types.ts` - TypeScript type definitions
- `store.ts` - Svelte stores and API client
- `{}.svelte` - Svelte component
- `package.json` - Package configuration
- `README.md` - This documentation

## API Operations

{}

## Development

```bash
# Build TypeScript
npm run build

# Watch mode
npm run dev
```
"#,
            spec.info.title,
            spec.info.title,
            spec.info.version,
            spec.info.title.replace(' ', ""),
            spec.info.title.to_lowercase().replace(' ', "-"),
            spec.info.title.replace(' ', ""),
            base_url,
            spec.info.title.to_lowercase().replace(' ', "-"),
            operations_list
        );

        Ok(readme)
    }

    /// Generate list of operations for README
    fn generate_operations_list(&self, spec: &OpenApiSpec) -> String {
        let mut operations = Vec::new();

        for (path, path_item) in &spec.paths {
            for (method, operation) in &path_item.operations {
                let fallback_summary = format!("{} {}", method.to_uppercase(), path);
                let summary = operation
                    .summary
                    .as_ref()
                    .unwrap_or(&operation.operation_id.as_ref().unwrap_or(&fallback_summary));

                operations.push(format!("- **{} {}** - {}", method.to_uppercase(), path, summary));
            }
        }

        operations.join("\n")
    }

    /// Count operations in the spec
    fn count_operations(&self, spec: &OpenApiSpec) -> usize {
        spec.paths.values().map(|path_item| path_item.operations.len()).sum()
    }

    /// Count schemas in the spec
    fn count_schemas(&self, spec: &OpenApiSpec) -> usize {
        spec.components
            .as_ref()
            .and_then(|c| c.schemas.as_ref())
            .map(|s| s.len())
            .unwrap_or(0)
    }
}

#[async_trait::async_trait]
impl ClientGeneratorPlugin for SvelteClientGenerator {
    fn framework_name(&self) -> &str {
        "svelte"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["ts", "js", "svelte"]
    }

    async fn generate_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult> {
        self.generate_svelte_client(spec, config)
    }

    async fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Svelte Client Generator").with_capability("client_generator")
    }
}

impl Default for SvelteClientGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create SvelteClientGenerator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_generator::{ApiInfo, OpenApiSpec};

    #[test]
    fn test_svelte_client_generator_creation() {
        let generator = SvelteClientGenerator::new();
        assert!(generator.is_ok());
    }

    #[test]
    fn test_framework_name() {
        let generator = SvelteClientGenerator::new().unwrap();
        assert_eq!(generator.framework_name(), "svelte");
    }

    #[test]
    fn test_supported_extensions() {
        let generator = SvelteClientGenerator::new().unwrap();
        let extensions = generator.supported_extensions();
        assert!(extensions.contains(&"ts"));
        assert!(extensions.contains(&"js"));
        assert!(extensions.contains(&"svelte"));
    }

    #[tokio::test]
    async fn test_generate_client() {
        let generator = SvelteClientGenerator::new().unwrap();

        let spec = OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API".to_string()),
            },
            servers: None,
            paths: std::collections::HashMap::new(),
            components: None,
        };

        let config = ClientGeneratorConfig {
            output_dir: "./output".to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            include_types: true,
            include_mocks: false,
            template_dir: None,
            options: std::collections::HashMap::new(),
        };

        let result = generator.generate_client(&spec, &config).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(!result.files.is_empty());
        assert_eq!(result.metadata.framework, "svelte");
    }
}
