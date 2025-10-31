//! Vue Client Generator Plugin
//!
//! Generates Vue 3 Composition API composables and TypeScript types from OpenAPI specifications
//! for easy integration with Vue applications.

use crate::client_generator::{
    ClientGenerationResult, ClientGeneratorConfig, ClientGeneratorPlugin, GeneratedFile,
    GenerationMetadata, OpenApiSpec,
};
use crate::types::{PluginError, PluginMetadata, Result};
use handlebars::Handlebars;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Vue client generator plugin
pub struct VueClientGenerator {
    /// Template registry for code generation
    templates: Handlebars<'static>,
}

impl VueClientGenerator {
    /// Create a new Vue client generator
    pub fn new() -> Result<Self> {
        let mut templates = Handlebars::new();

        // Register templates for Vue code generation
        Self::register_templates(&mut templates)?;

        Ok(Self { templates })
    }

    /// Register Handlebars templates for Vue code generation
    fn register_templates(templates: &mut Handlebars<'static>) -> Result<()> {
        // TypeScript types template (same as React)
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

        // Vue composables template
        templates.register_template_string(
            "composables",
            r#"// Generated Vue 3 composables for {{api_title}}
// API Version: {{api_version}}

import { ref, computed, type Ref } from 'vue';

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

// Generic API client
class ApiClient {
  constructor(private config: ApiConfig) {}

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
  async {{operation_id}}({{#if request_body}}data: {{operation_id}}Request{{/if}}): Promise<{{operation_id}}Response> {
    const endpoint = '{{path}}'{{#if (eq method "GET")}}{{#if request_body}} + '?' + new URLSearchParams(data as any).toString(){{/if}}{{/if}};

    return this.request<{{operation_id}}Response>(endpoint, {
      method: '{{method}}',
      {{#if request_body}}{{#unless (eq method "GET")}}body: JSON.stringify(data),{{/unless}}{{/if}}
    });
  }

  {{/each}}
}

// Vue composables for each operation
{{#each operations}}
export function use{{operation_id}}({{#if request_body}}data?: Ref<{{operation_id}}Request> | {{operation_id}}Request{{/if}}) {
  const result = ref<{{operation_id}}Response | null>(null);
  const loading = ref(false);
  const error = ref<Error | null>(null);

  const execute = async ({{#if request_body}}requestData?: {{operation_id}}Request{{/if}}) => {
    loading.value = true;
    error.value = null;

    try {
      const client = new ApiClient(defaultConfig);
      const response = await client.{{operation_id}}({{#if request_body}}requestData || (data && 'value' in data ? data.value : data){{/if}});
      result.value = response;
    } catch (err) {
      error.value = err as Error;
    } finally {
      loading.value = false;
    }
  };

  {{#if (eq method "GET")}}
  // Auto-execute for GET requests
  execute();
  {{/if}}

  return {
    {{#if (eq method "GET")}}data: computed(() => result.value),{{/if}}
    {{#unless (eq method "GET")}}result: computed(() => result.value),{{/unless}}
    loading: computed(() => loading.value),
    error: computed(() => error.value),
    {{#unless (eq method "GET")}}execute,{{/unless}}
  };
}

{{/each}}

// Export the API client for direct use
export const apiClient = new ApiClient(defaultConfig);

// Export types
export * from './types';"#,
        ).map_err(|e| PluginError::execution(format!("Failed to register composables template: {}", e)))?;

        // Pinia store template
        templates.register_template_string(
            "store",
            r#"// Generated Pinia store for {{api_title}}
// API Version: {{api_version}}

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { apiClient } from './composables';

export const use{{api_title}}Store = defineStore('{{api_title}}', () => {
  // State
  const data = ref<Record<string, any>>({});
  const loading = ref(false);
  const error = ref<Error | null>(null);

  // Getters
  const isLoading = computed(() => loading.value);
  const hasError = computed(() => error.value !== null);
  const errorMessage = computed(() => error.value?.message || '');

  // Actions
  const setLoading = (value: boolean) => {
    loading.value = value;
  };

  const setError = (err: Error | null) => {
    error.value = err;
  };

  const setData = (key: string, value: any) => {
    data.value[key] = value;
  };

  const clearError = () => {
    error.value = null;
  };

  {{#each operations}}
  const {{operation_id}} = async ({{#if request_body}}requestData: {{operation_id}}Request{{/if}}) => {
    setLoading(true);
    clearError();

    try {
      const response = await apiClient.{{operation_id}}({{#if request_body}}requestData{{/if}});
      {{#if (eq method "GET")}}
      setData('{{operation_id}}', response);
      {{/if}}
      return response;
    } catch (err) {
      setError(err as Error);
      throw err;
    } finally {
      setLoading(false);
    }
  };

  {{/each}}

  return {
    // State
    data: computed(() => data.value),
    loading: isLoading,
    error: hasError,
    errorMessage,

    // Actions
    {{#each operations}}
    {{operation_id}},
    {{/each}}
    clearError,
  };
});"#,
        ).map_err(|e| PluginError::execution(format!("Failed to register store template: {}", e)))?;

        // TypeScript type helper template
        templates.register_template_string(
            "typescript_type",
            r#"{{#if (eq type "string")}}string{{/if}}{{#if (eq type "integer")}}number{{/if}}{{#if (eq type "number")}}number{{/if}}{{#if (eq type "boolean")}}boolean{{/if}}{{#if (eq type "array")}}{{#if items}}{{> typescript_type items}}[]{{else}}any[]{{/if}}{{/if}}{{#if (eq type "object")}}{{#if properties}}{ {{#each properties}}{{@key}}: {{> typescript_type this}}{{#unless @last}}, {{/unless}}{{/each}} }{{else}}Record<string, any>{{/if}}{{/if}}{{#unless type}}any{{/unless}}"#,
        ).map_err(|e| PluginError::execution(format!("Failed to register typescript_type template: {}", e)))?;

        Ok(())
    }

    /// Generate Vue client code from OpenAPI specification
    fn generate_vue_client(
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

        // Generate Vue composables
        let composables_content = self.templates.render("composables", &context).map_err(|e| {
            PluginError::execution(format!("Failed to render composables template: {}", e))
        })?;

        files.push(GeneratedFile {
            path: "composables.ts".to_string(),
            content: composables_content,
            file_type: "typescript".to_string(),
        });

        // Generate Pinia store
        let store_content = self.templates.render("store", &context).map_err(|e| {
            PluginError::execution(format!("Failed to render store template: {}", e))
        })?;

        files.push(GeneratedFile {
            path: "store.ts".to_string(),
            content: store_content,
            file_type: "typescript".to_string(),
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
            framework: "vue".to_string(),
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

    /// Generate package.json for the Vue client
    fn generate_package_json(
        &self,
        spec: &OpenApiSpec,
        _config: &ClientGeneratorConfig,
    ) -> Result<String> {
        let package_name = format!("{}-client", spec.info.title.to_lowercase().replace(' ', "-"));

        let package_json = json!({
            "name": package_name,
            "version": "1.0.0",
            "description": format!("Vue 3 client for {}", spec.info.title),
            "main": "composables.ts",
            "types": "types.ts",
            "scripts": {
                "build": "tsc",
                "dev": "tsc --watch"
            },
            "dependencies": {
                "vue": "^3.3.0",
                "pinia": "^2.1.0"
            },
            "devDependencies": {
                "@types/node": "^20.0.0",
                "typescript": "^5.0.0"
            },
            "peerDependencies": {
                "vue": ">=3.0.0"
            }
        });

        serde_json::to_string_pretty(&package_json)
            .map_err(|e| PluginError::execution(format!("Failed to serialize package.json: {}", e)))
    }

    /// Generate README for the Vue client
    fn generate_readme(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<String> {
        let readme = format!(
            r#"# {} Vue Client

Generated Vue 3 client for {} API (v{}).

## Installation

```bash
npm install
```

## Usage

### Using Vue Composables

```vue
<template>
  <div>
    <div v-if="loading">Loading...</div>
    <div v-else-if="error">Error: {{error.message}}</div>
    <div v-else>
      <div v-for="user in data" :key="user.id">
        {{user.name}}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import {{ useGetUsers }} from './composables';

const {{ data, loading, error }} = useGetUsers();
</script>
```

### Using Pinia Store

```vue
<template>
  <div>
    <div v-if="loading">Loading...</div>
    <div v-else-if="error">Error: {{errorMessage}}</div>
    <div v-else>
      <div v-for="user in data" :key="user.id">
        {{user.name}}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import {{ use{}Store }} from './store';

const store = use{}Store();
const {{ data, loading, error, errorMessage }} = storeToRefs(store);

// Execute operations
await store.getUsers();
</script>
```

### Using API Client Directly

```typescript
import {{ apiClient }} from './composables';

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

You can modify the configuration by updating the `defaultConfig` object in `composables.ts`.

## Generated Files

- `types.ts` - TypeScript type definitions
- `composables.ts` - Vue 3 composables and API client
- `store.ts` - Pinia store for state management
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
            spec.info.title,
            spec.info.title,
            config.base_url.as_ref().unwrap_or(&"http://localhost:3000".to_string()),
            self.generate_operations_list(spec)
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
impl ClientGeneratorPlugin for VueClientGenerator {
    fn framework_name(&self) -> &str {
        "vue"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["ts", "vue", "js"]
    }

    async fn generate_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult> {
        self.generate_vue_client(spec, config)
    }

    async fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Vue Client Generator").with_capability("client_generator")
    }
}

impl Default for VueClientGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create VueClientGenerator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_generator::{ApiInfo, OpenApiSpec};

    #[test]
    fn test_vue_client_generator_creation() {
        let generator = VueClientGenerator::new();
        assert!(generator.is_ok());
    }

    #[test]
    fn test_framework_name() {
        let generator = VueClientGenerator::new().unwrap();
        assert_eq!(generator.framework_name(), "vue");
    }

    #[test]
    fn test_supported_extensions() {
        let generator = VueClientGenerator::new().unwrap();
        let extensions = generator.supported_extensions();
        assert!(extensions.contains(&"ts"));
        assert!(extensions.contains(&"vue"));
        assert!(extensions.contains(&"js"));
    }

    #[tokio::test]
    async fn test_generate_client() {
        let generator = VueClientGenerator::new().unwrap();

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
        assert_eq!(result.metadata.framework, "vue");
    }
}
