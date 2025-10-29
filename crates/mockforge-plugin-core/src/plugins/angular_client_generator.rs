//! Angular Client Generator Plugin
//!
//! Generates Angular services and TypeScript types from OpenAPI specifications
//! for easy integration with Angular applications.

use crate::client_generator::{
    ClientGenerationResult, ClientGeneratorConfig, ClientGeneratorPlugin, GeneratedFile,
    GenerationMetadata, OpenApiSpec,
};
use crate::types::{PluginError, PluginMetadata, Result};
use serde_json::json;

/// Angular client generator plugin
pub struct AngularClientGenerator;

impl AngularClientGenerator {
    /// Create a new Angular client generator
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Generate Angular client code from OpenAPI specification
    fn generate_angular_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult> {
        let mut files = Vec::new();
        let warnings = Vec::new();

        // Generate TypeScript types
        let types_content = self.generate_types(spec)?;
        files.push(GeneratedFile {
            path: "types.ts".to_string(),
            content: types_content,
            file_type: "typescript".to_string(),
        });

        // Generate Angular service
        let service_content = self.generate_service(spec, config)?;
        files.push(GeneratedFile {
            path: format!("{}.service.ts", spec.info.title.to_lowercase().replace(' ', "-")),
            content: service_content,
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
            framework: "angular".to_string(),
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

    /// Generate TypeScript types
    fn generate_types(&self, spec: &OpenApiSpec) -> Result<String> {
        let mut content = format!(
            "// Generated TypeScript types for {}\n// API Version: {}\n\n",
            spec.info.title, spec.info.version
        );

        // Basic types
        content.push_str("// Basic types - customize as needed\n");
        content.push_str("export interface ApiResponse {\n");
        content.push_str("  data: any;\n");
        content.push_str("  status: number;\n");
        content.push_str("  message?: string;\n");
        content.push_str("}\n\n");

        content.push_str("export interface ApiRequest {\n");
        content.push_str("  data: any;\n");
        content.push_str("}\n\n");

        Ok(content)
    }

    /// Generate Angular service
    fn generate_service(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<String> {
        let default_url = "http://localhost:3000".to_string();
        let base_url = config.base_url.as_ref().unwrap_or(&default_url);
        let service_name = spec.info.title.replace(' ', "");

        let mut content = format!(
            "// Generated Angular service for {}\n// API Version: {}\n\n",
            spec.info.title, spec.info.version
        );

        content.push_str("import { Injectable } from '@angular/core';\n");
        content.push_str("import { HttpClient } from '@angular/common/http';\n");
        content.push_str("import { Observable } from 'rxjs';\n\n");

        content.push_str("@Injectable({\n");
        content.push_str("  providedIn: 'root'\n");
        content.push_str("})\n");
        content.push_str(&format!("export class {}Service {{\n", service_name));
        content.push_str(&format!("  private baseUrl = '{}';\n\n", base_url));
        content.push_str("  constructor(private http: HttpClient) {}\n\n");

        content.push_str("  // Example method - customize as needed\n");
        content.push_str("  getData(): Observable<any> {\n");
        content.push_str("    return this.http.get<any>(`${this.baseUrl}/api/data`);\n");
        content.push_str("  }\n");
        content.push_str("}\n");

        Ok(content)
    }

    /// Generate package.json for the Angular client
    fn generate_package_json(
        &self,
        spec: &OpenApiSpec,
        _config: &ClientGeneratorConfig,
    ) -> Result<String> {
        let package_name = format!("{}-client", spec.info.title.to_lowercase().replace(' ', "-"));

        let package_json = json!({
            "name": package_name,
            "version": "1.0.0",
            "description": format!("Angular client for {}", spec.info.title),
            "main": format!("{}.service.ts", spec.info.title.to_lowercase().replace(' ', "-")),
            "types": "types.ts",
            "scripts": {
                "build": "tsc",
                "dev": "tsc --watch"
            },
            "dependencies": {
                "@angular/common": "^17.0.0",
                "@angular/core": "^17.0.0",
                "rxjs": "^7.0.0"
            },
            "devDependencies": {
                "typescript": "^5.0.0"
            },
            "peerDependencies": {
                "@angular/common": ">=17.0.0",
                "@angular/core": ">=17.0.0",
                "rxjs": ">=7.0.0"
            }
        });

        serde_json::to_string_pretty(&package_json)
            .map_err(|e| PluginError::execution(format!("Failed to serialize package.json: {}", e)))
    }

    /// Generate README for the Angular client
    fn generate_readme(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<String> {
        let service_name = spec.info.title.replace(' ', "");
        let default_url = "http://localhost:3000".to_string();
        let base_url = config.base_url.as_ref().unwrap_or(&default_url);

        let readme = format!(
            r#"# {} Angular Client

Generated Angular client for {} API (v{}).

## Installation

```bash
npm install
```

## Usage

### Using Angular Service

```typescript
import {{ Component, OnInit }} from '@angular/core';
import {{ {}Service }} from './{}.service';
import {{ Observable }} from 'rxjs';

@Component({{
  selector: 'app-my-component',
  template: `
    <div *ngIf="loading">Loading...</div>
    <div *ngIf="error">Error: {{error.message}}</div>
    <div *ngIf="data">
      <div *ngFor="let item of data">
        {{item.name}}
      </div>
    </div>
  `,
}})
export class MyComponent implements OnInit {{
  data: any[] | null = null;
  loading: boolean = false;
  error: Error | null = null;

  constructor(private apiService: {}Service) {{}}

  ngOnInit(): void {{
    this.loading = true;
    this.apiService.getData().subscribe({{
      next: (data) => {{
        this.data = data;
        this.loading = false;
      }},
      error: (err) => {{
        this.error = err;
        this.loading = false;
      }}
    }});
  }}
}}
```

## Configuration

The client is configured to use the following base URL: `{}`

You can modify the configuration by updating the `baseUrl` property in `{}.service.ts`.

## Generated Files

- `types.ts` - TypeScript type definitions
- `{}.service.ts` - Angular service
- `package.json` - Package configuration
- `README.md` - This documentation

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
            service_name,
            spec.info.title.to_lowercase().replace(' ', "-"),
            service_name,
            base_url,
            spec.info.title.to_lowercase().replace(' ', "-"),
            spec.info.title.to_lowercase().replace(' ', "-")
        );

        Ok(readme)
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
impl ClientGeneratorPlugin for AngularClientGenerator {
    fn framework_name(&self) -> &str {
        "angular"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["ts"]
    }

    async fn generate_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult> {
        self.generate_angular_client(spec, config)
    }

    async fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Angular Client Generator").with_capability("client_generator")
    }
}

impl Default for AngularClientGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create AngularClientGenerator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_generator::{ApiInfo, OpenApiSpec};
    use std::collections::HashMap;

    #[test]
    fn test_angular_client_generator_creation() {
        let generator = AngularClientGenerator::new();
        assert!(generator.is_ok());
    }

    #[test]
    fn test_framework_name() {
        let generator = AngularClientGenerator::new().unwrap();
        assert_eq!(generator.framework_name(), "angular");
    }

    #[test]
    fn test_supported_extensions() {
        let generator = AngularClientGenerator::new().unwrap();
        let extensions = generator.supported_extensions();
        assert!(extensions.contains(&"ts"));
    }

    #[tokio::test]
    async fn test_generate_client() {
        let generator = AngularClientGenerator::new().unwrap();

        let spec = OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API".to_string()),
            },
            servers: None,
            paths: HashMap::new(),
            components: None,
        };

        let config = ClientGeneratorConfig {
            output_dir: "./output".to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            include_types: true,
            include_mocks: false,
            template_dir: None,
            options: HashMap::new(),
        };

        let result = generator.generate_client(&spec, &config).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(!result.files.is_empty());
        assert_eq!(result.metadata.framework, "angular");
    }
}
