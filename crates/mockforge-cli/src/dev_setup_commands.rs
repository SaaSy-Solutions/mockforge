//! One-command frontend integration setup
//!
//! Provides `mockforge dev-setup <framework>` command that:
//! - Detects existing project structure
//! - Generates typed client
//! - Creates example hooks/composables/services
//! - Adds .env.mockforge.example
//! - Sets up SDK dependencies

use clap::Args;
#[cfg(feature = "vue")]
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Dev setup command arguments
#[derive(Debug, Args)]
pub struct DevSetupArgs {
    /// Framework to set up (react, vue, angular, svelte, next, nuxt)
    pub framework: String,

    /// Base URL for MockForge server
    #[arg(long, default_value = "http://localhost:3000")]
    pub base_url: String,

    /// Reality level (static, light, moderate, high, chaos)
    #[arg(long, default_value = "moderate")]
    pub reality_level: String,

    /// OpenAPI spec file path (optional, for client generation)
    #[arg(short, long)]
    pub spec: Option<PathBuf>,

    /// Output directory for generated files
    #[arg(short, long, default_value = "./src/mockforge")]
    pub output: PathBuf,

    /// Overwrite existing files
    #[arg(long)]
    pub force: bool,
}

/// Supported frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    React,
    Vue,
    Angular,
    Svelte,
    Next,
    Nuxt,
}

impl Framework {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "react" => Some(Self::React),
            "vue" => Some(Self::Vue),
            "angular" => Some(Self::Angular),
            "svelte" => Some(Self::Svelte),
            "next" => Some(Self::Next),
            "nuxt" => Some(Self::Nuxt),
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::React => "react",
            Self::Vue => "vue",
            Self::Angular => "angular",
            Self::Svelte => "svelte",
            Self::Next => "next",
            Self::Nuxt => "nuxt",
        }
    }

    fn sdk_package(&self) -> &'static str {
        match self {
            Self::React | Self::Next => "@mockforge/sdk",
            Self::Vue | Self::Nuxt => "@mockforge/sdk",
            Self::Angular => "@mockforge/sdk",
            Self::Svelte => "@mockforge/sdk",
        }
    }
}

/// Execute dev-setup command
pub async fn execute_dev_setup(args: DevSetupArgs) -> anyhow::Result<()> {
    let framework = Framework::from_str(&args.framework).ok_or_else(|| {
        anyhow::anyhow!(
            "Unsupported framework: {}. Supported: react, vue, angular, svelte, next, nuxt",
            args.framework
        )
    })?;

    println!("🚀 Setting up MockForge for {}...", framework.name());

    // Detect project structure
    let project_root = detect_project_root()?;
    println!("  ✓ Detected project root: {}", project_root.display());

    // Detect existing MockForge workspace
    let (detected_base_url, detected_reality_level) = detect_mockforge_workspace(&project_root)?;

    // Use detected values or fall back to provided/default values
    let base_url = if args.base_url == "http://localhost:3000" && detected_base_url.is_some() {
        detected_base_url.as_ref().unwrap().clone()
    } else {
        args.base_url.clone()
    };

    let reality_level = if args.reality_level == "moderate" && detected_reality_level.is_some() {
        detected_reality_level.as_ref().unwrap().clone()
    } else {
        args.reality_level.clone()
    };

    if detected_base_url.is_some() || detected_reality_level.is_some() {
        println!("  ✓ Detected existing MockForge workspace configuration");
        if let Some(ref url) = detected_base_url {
            println!("    Base URL: {}", url);
        }
        if let Some(ref level) = detected_reality_level {
            println!("    Reality level: {}", level);
        }
    }

    // Check if workspace was created from a blueprint
    let blueprint_spec = detect_blueprint_origin(&project_root)?;

    // Auto-detect OpenAPI spec if not provided
    let spec_path = if args.spec.is_some() {
        args.spec.clone()
    } else if let Some(ref blueprint_spec_path) = blueprint_spec {
        println!("  ✓ Using OpenAPI spec from blueprint: {}", blueprint_spec_path.display());
        Some(blueprint_spec_path.clone())
    } else {
        let auto_detected = auto_detect_openapi_spec(&project_root)?;
        if let Some(ref spec) = auto_detected {
            println!("  ✓ Auto-detected OpenAPI spec: {}", spec.display());
        }
        auto_detected
    };

    // Check for existing client code
    let output_dir = project_root.join(&args.output);
    if output_dir.exists() && !args.force {
        let existing_files = check_existing_client_code(&output_dir)?;
        if !existing_files.is_empty() {
            println!("  ⚠️  Found existing client code in {}", output_dir.display());
            println!("     Files: {}", existing_files.join(", "));
            println!("     Use --force to overwrite");
            return Ok(());
        }
    }

    // Create output directory
    fs::create_dir_all(&output_dir)?;
    println!("  ✓ Created output directory: {}", output_dir.display());

    // Generate client if spec provided or detected
    if let Some(spec_path) = &spec_path {
        println!("  📦 Generating typed client from OpenAPI spec...");
        generate_client(framework, spec_path, &output_dir, &base_url).await?;
        println!("  ✓ Client generated");
    } else {
        println!("  ⚠️  No OpenAPI spec found, skipping client generation");
        println!("     Use --spec <path> to generate typed client");
    }

    // Generate framework-specific hooks/composables/services
    println!("  📝 Generating {} examples...", framework.name());
    generate_framework_examples(framework, &output_dir, &base_url)?;
    println!("  ✓ Examples generated");

    // Create .env.mockforge.example
    println!("  🔧 Creating environment configuration...");
    create_env_example(&project_root, &base_url, &reality_level)?;
    println!("  ✓ Environment configuration created");

    // Update package.json if it exists
    if let Some(package_json_path) = find_package_json(&project_root) {
        println!("  📦 Updating package.json...");
        update_package_json(&package_json_path, framework)?;
        println!("  ✓ package.json updated");
    }

    // Verify TypeScript compilation if tsc is available
    if let Some(tsconfig_path) = find_tsconfig(&project_root) {
        println!("  🔍 Verifying TypeScript compilation...");
        if verify_typescript_compilation(&tsconfig_path).is_ok() {
            println!("  ✓ TypeScript compilation verified");
        } else {
            println!("  ⚠️  TypeScript compilation check skipped (tsc not found)");
        }
    }

    println!("\n✅ MockForge setup complete!");
    println!("\nNext steps:");
    println!("  1. Copy .env.mockforge.example to .env.mockforge");
    println!("  2. Review generated files in {}", output_dir.display());
    println!("  3. Import and use the generated hooks/composables in your app");
    println!(
        "  4. Check out the example component: {}",
        output_dir.join("UserList.example.tsx").display()
    );
    println!("  5. Start MockForge server: mockforge serve");

    Ok(())
}

/// Detect project root directory
fn detect_project_root() -> anyhow::Result<PathBuf> {
    let current_dir = std::env::current_dir()?;

    // Look for common project indicators
    let indicators = [
        "package.json",
        "Cargo.toml",
        "go.mod",
        "pom.xml",
        "build.gradle",
    ];

    let mut dir = current_dir.clone();
    loop {
        for indicator in &indicators {
            if dir.join(indicator).exists() {
                return Ok(dir);
            }
        }

        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Fallback to current directory
    Ok(current_dir)
}

/// Find package.json in project
fn find_package_json(project_root: &Path) -> Option<PathBuf> {
    let path = project_root.join("package.json");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Detect existing MockForge workspace and extract configuration
fn detect_mockforge_workspace(
    project_root: &Path,
) -> anyhow::Result<(Option<String>, Option<String>)> {
    // Look for mockforge.yaml or mockforge.yml
    let config_paths = [
        project_root.join("mockforge.yaml"),
        project_root.join("mockforge.yml"),
        project_root.join(".mockforge.yaml"),
        project_root.join(".mockforge.yml"),
    ];

    for config_path in &config_paths {
        if config_path.exists() {
            return load_config_values(config_path);
        }
    }

    Ok((None, None))
}

/// Load base URL and reality level from config file
fn load_config_values(config_path: &Path) -> anyhow::Result<(Option<String>, Option<String>)> {
    use serde_yaml::Value;

    let content = fs::read_to_string(config_path)?;
    let config: Value = serde_yaml::from_str(&content)?;

    // Extract base URL from http.port (construct URL)
    let base_url = if let Some(http) = config.get("http") {
        let port = http.get("port").and_then(|v| v.as_u64()).unwrap_or(3000);
        let host = http.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");

        // Convert 0.0.0.0 to localhost for client usage
        let host_str = if host == "0.0.0.0" { "localhost" } else { host };
        Some(format!("http://{}:{}", host_str, port))
    } else {
        None
    };

    // Extract reality level
    let reality_level = config
        .get("reality")
        .and_then(|r| r.get("level"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok((base_url, reality_level))
}

/// Auto-detect OpenAPI specification files in workspace
fn auto_detect_openapi_spec(project_root: &Path) -> anyhow::Result<Option<PathBuf>> {
    // Common OpenAPI spec file names and locations
    let spec_candidates = [
        // Root directory
        project_root.join("openapi.json"),
        project_root.join("openapi.yaml"),
        project_root.join("openapi.yml"),
        project_root.join("api-spec.json"),
        project_root.join("api-spec.yaml"),
        project_root.join("api.json"),
        project_root.join("api.yaml"),
        // Examples directory
        project_root.join("examples").join("openapi.json"),
        project_root.join("examples").join("openapi.yaml"),
        // Docs directory
        project_root.join("docs").join("openapi.json"),
        project_root.join("docs").join("openapi.yaml"),
        // API directory
        project_root.join("api").join("openapi.json"),
        project_root.join("api").join("openapi.yaml"),
        // Spec directory
        project_root.join("spec").join("openapi.json"),
        project_root.join("spec").join("openapi.yaml"),
    ];

    for candidate in &spec_candidates {
        if candidate.exists() {
            // Quick validation: check if it's a valid OpenAPI spec
            if is_valid_openapi_spec(candidate)? {
                return Ok(Some(candidate.clone()));
            }
        }
    }

    // Also check config file for openapi_spec reference
    let config_paths = [
        project_root.join("mockforge.yaml"),
        project_root.join("mockforge.yml"),
    ];

    for config_path in &config_paths {
        if config_path.exists() {
            if let Ok(Some(spec_path)) = extract_spec_from_config(config_path, project_root) {
                if spec_path.exists() && is_valid_openapi_spec(&spec_path)? {
                    return Ok(Some(spec_path));
                }
            }
        }
    }

    Ok(None)
}

/// Check if file is a valid OpenAPI specification
fn is_valid_openapi_spec(path: &Path) -> anyhow::Result<bool> {
    let content = fs::read_to_string(path)?;

    // Try parsing as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
        if json.get("openapi").is_some() || json.get("swagger").is_some() {
            return Ok(true);
        }
    }

    // Try parsing as YAML
    if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(&content) {
        if yaml.get("openapi").is_some() || yaml.get("swagger").is_some() {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Extract OpenAPI spec path from mockforge.yaml config
fn extract_spec_from_config(
    config_path: &Path,
    project_root: &Path,
) -> anyhow::Result<Option<PathBuf>> {
    use serde_yaml::Value;

    let content = fs::read_to_string(config_path)?;
    let config: Value = serde_yaml::from_str(&content)?;

    // Check http.openapi_spec
    if let Some(spec_path_str) =
        config.get("http").and_then(|h| h.get("openapi_spec")).and_then(|v| v.as_str())
    {
        let spec_path = if spec_path_str.starts_with('/') {
            PathBuf::from(spec_path_str)
        } else {
            project_root.join(spec_path_str)
        };
        return Ok(Some(spec_path));
    }

    Ok(None)
}

/// Check for existing client code in output directory
fn check_existing_client_code(output_dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut existing_files = Vec::new();

    if !output_dir.exists() {
        return Ok(existing_files);
    }

    // Check for common generated files
    let common_files = [
        "client.ts",
        "client.js",
        "hooks.ts",
        "hooks.js",
        "composables.ts",
        "composables.js",
        "types.ts",
        "types.js",
        "index.ts",
        "index.js",
    ];

    for file in &common_files {
        let file_path = output_dir.join(file);
        if file_path.exists() {
            existing_files.push(file.to_string());
        }
    }

    Ok(existing_files)
}

/// Detect if workspace was created from a blueprint and return blueprint's OpenAPI spec path
fn detect_blueprint_origin(project_root: &Path) -> anyhow::Result<Option<PathBuf>> {
    // Check for blueprint indicators:
    // 1. README mentions blueprint
    // 2. scenarios/ directory exists (blueprint-specific)
    // 3. contracts/ directory exists (blueprint-specific)

    let has_scenarios = project_root.join("scenarios").exists();
    let has_contracts = project_root.join("contracts").exists();

    if has_scenarios || has_contracts {
        // This looks like a blueprint-based workspace
        // Check config for openapi_spec reference
        let config_paths = [
            project_root.join("mockforge.yaml"),
            project_root.join("mockforge.yml"),
        ];

        for config_path in &config_paths {
            if config_path.exists() {
                if let Ok(Some(spec_path)) = extract_spec_from_config(config_path, project_root) {
                    if spec_path.exists() {
                        return Ok(Some(spec_path));
                    }
                }
            }
        }

        // Also check for openapi.yaml in root (common blueprint location)
        let openapi_candidates = [
            project_root.join("openapi.yaml"),
            project_root.join("openapi.json"),
            project_root.join("openapi.yml"),
        ];

        for candidate in &openapi_candidates {
            if candidate.exists() && is_valid_openapi_spec(candidate)? {
                return Ok(Some(candidate.clone()));
            }
        }
    }

    Ok(None)
}

/// Generate typed client from OpenAPI spec
async fn generate_client(
    framework: Framework,
    spec_path: &Path,
    output_dir: &Path,
    base_url: &str,
) -> anyhow::Result<()> {
    use crate::client_generator::{ClientGeneratorManager, GenerateArgs};

    let manager = ClientGeneratorManager::new()?;

    let framework_name = match framework {
        Framework::React | Framework::Next => "react",
        Framework::Vue | Framework::Nuxt => "vue",
        Framework::Angular => "angular",
        Framework::Svelte => "svelte",
    };

    let args = GenerateArgs {
        spec: spec_path.to_string_lossy().to_string(),
        framework: framework_name.to_string(),
        output: output_dir.to_string_lossy().to_string(),
        base_url: Some(base_url.to_string()),
        include_types: true,
        include_mocks: false,
        template_dir: None,
        options: None,
    };

    manager.generate_client(&args).await?;

    Ok(())
}

/// Generate framework-specific examples
fn generate_framework_examples(
    framework: Framework,
    output_dir: &Path,
    base_url: &str,
) -> anyhow::Result<()> {
    match framework {
        Framework::React | Framework::Next => generate_react_examples(output_dir, base_url)?,
        Framework::Vue | Framework::Nuxt => generate_vue_examples(output_dir, base_url)?,
        Framework::Angular => generate_angular_examples(output_dir, base_url)?,
        Framework::Svelte => generate_svelte_examples(output_dir, base_url)?,
    }

    Ok(())
}

/// Generate React/Next.js examples
fn generate_react_examples(output_dir: &Path, base_url: &str) -> anyhow::Result<()> {
    // React Query hooks example with comprehensive error handling
    let react_query_hooks = format!(
        r#"// React Query hooks for MockForge API
// Generated with comprehensive error handling and TypeScript types

import {{ useQuery, useMutation, useQueryClient }} from '@tanstack/react-query';
import type {{ ApiError }} from './client';

const MOCKFORGE_BASE_URL = '{}';

// ============================================================================
// Error Handling Utilities
// ============================================================================

/**
 * Format API error for display to users
 */
export function formatApiError(error: unknown): string {{
  if (error instanceof Error) {{
    // Check if it's an ApiError with detailed information
    if ('status' in error && 'statusText' in error) {{
      const apiError = error as ApiError;
      if (apiError.body && typeof apiError.body === 'object') {{
        // Extract user-friendly error message
        if ('message' in apiError.body) {{
          return String(apiError.body.message);
        }}
        if ('error' in apiError.body) {{
          return String(apiError.body.error);
        }}
      }}
      return `${{apiError.status}} ${{apiError.statusText}}`;
    }}
    return error.message;
  }}
  return 'An unexpected error occurred';
}}

/**
 * Check if error is a network error (can be retried)
 */
export function isNetworkError(error: unknown): boolean {{
  if (error instanceof Error) {{
    return error.message.includes('fetch') ||
           error.message.includes('network') ||
           error.message.includes('Failed to fetch');
  }}
  return false;
}}

/**
 * Check if error is a client error (4xx - user input issue)
 */
export function isClientError(error: unknown): boolean {{
  if (error && typeof error === 'object' && 'status' in error) {{
    const status = (error as {{ status: number }}).status;
    return status >= 400 && status < 500;
  }}
  return false;
}}

/**
 * Check if error is a server error (5xx - server issue)
 */
export function isServerError(error: unknown): boolean {{
  if (error && typeof error === 'object' && 'status' in error) {{
    const status = (error as {{ status: number }}).status;
    return status >= 500;
  }}
  return false;
}}

// ============================================================================
// Type Definitions
// ============================================================================

export interface User {{
  id: string;
  name: string;
  email: string;
  createdAt?: string;
}}

export interface CreateUserRequest {{
  name: string;
  email: string;
}}

export interface UpdateUserRequest {{
  name?: string;
  email?: string;
}}

// ============================================================================
// React Query Hooks
// ============================================================================

/**
 * Hook to fetch all users
 *
 * @example
 * ```tsx
 * function UserList() {{
 *   const {{ data: users, isLoading, error }} = useUsers();
 *
 *   if (isLoading) return <div>Loading...</div>;
 *   if (error) return <div>Error: {{formatApiError(error)}}</div>;
 *
 *   return (
 *     <ul>
 *       {{users?.map(user => (
 *         <li key={{user.id}}>{{user.name}}</li>
 *       ))}}
 *     </ul>
 *   );
 * }}
 * ```
 */
export function useUsers() {{
  return useQuery<User[], ApiError>({{
    queryKey: ['users'],
    queryFn: async () => {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users`);
      if (!response.ok) {{
        const errorBody = await response.json().catch(() => ({{}}));
        throw new Error(`Failed to fetch users: ${{response.status}} ${{response.statusText}}`);
      }}
      const data = await response.json();
      return Array.isArray(data) ? data : (data.users || []);
    }},
    retry: (failureCount, error) => {{
      // Retry network errors up to 3 times
      if (isNetworkError(error)) {{
        return failureCount < 3;
      }}
      // Don't retry client errors (4xx)
      if (isClientError(error)) {{
        return false;
      }}
      // Retry server errors (5xx) up to 2 times
      return failureCount < 2;
    }},
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
  }});
}}

/**
 * Hook to fetch a single user by ID
 *
 * @param id - User ID
 * @example
 * ```tsx
 * function UserProfile({{ userId }}: {{ userId: string }}) {{
 *   const {{ data: user, isLoading, error }} = useUser(userId);
 *
 *   if (isLoading) return <div>Loading user...</div>;
 *   if (error) return <div>Error: {{formatApiError(error)}}</div>;
 *   if (!user) return <div>User not found</div>;
 *
 *   return (
 *     <div>
 *       <h1>{{user.name}}</h1>
 *       <p>{{user.email}}</p>
 *     </div>
 *   );
 * }}
 * ```
 */
export function useUser(id: string) {{
  return useQuery<User, ApiError>({{
    queryKey: ['users', id],
    queryFn: async () => {{
      if (!id) throw new Error('User ID is required');
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users/${{id}}`);
      if (!response.ok) {{
        if (response.status === 404) {{
          throw new Error('User not found');
        }}
        const errorBody = await response.json().catch(() => ({{}}));
        throw new Error(`Failed to fetch user: ${{response.status}} ${{response.statusText}}`);
      }}
      return response.json();
    }},
    enabled: !!id,
    retry: (failureCount, error) => {{
      if (isNetworkError(error)) return failureCount < 3;
      if (isClientError(error)) return false;
      return failureCount < 2;
    }},
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
  }});
}}

/**
 * Hook to create a new user
 *
 * @example
 * ```tsx
 * function CreateUserForm() {{
 *   const createUser = useCreateUser();
 *
 *   const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {{
 *     e.preventDefault();
 *     const formData = new FormData(e.currentTarget);
 *
 *     try {{
 *       await createUser.mutateAsync({{
 *         name: formData.get('name') as string,
 *         email: formData.get('email') as string,
 *       }});
 *       alert('User created successfully!');
 *     }} catch (error) {{
 *       alert(`Failed to create user: ${{formatApiError(error)}}`);
 *     }}
 *   }};
 *
 *   return (
 *     <form onSubmit={{handleSubmit}}>
 *       <input name="name" placeholder="Name" required />
 *       <input name="email" type="email" placeholder="Email" required />
 *       <button type="submit" disabled={{createUser.isPending}}>
 *         {{createUser.isPending ? 'Creating...' : 'Create User'}}
 *       </button>
 *       {{createUser.error && (
 *         <div style={{color: 'red'}}>
 *           Error: {{formatApiError(createUser.error)}}
 *         </div>
 *       )}}
 *     </form>
 *   );
 * }}
 * ```
 */
export function useCreateUser() {{
  const queryClient = useQueryClient();

  return useMutation<User, ApiError, CreateUserRequest>({{
    mutationFn: async (userData) => {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users`, {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify(userData),
      }});

      if (!response.ok) {{
        const errorBody = await response.json().catch(() => ({{}}));
        throw new Error(`Failed to create user: ${{response.status}} ${{response.statusText}}`);
      }}

      return response.json();
    }},
    onSuccess: () => {{
      // Invalidate and refetch users list
      queryClient.invalidateQueries({{ queryKey: ['users'] }});
    }},
    onError: (error) => {{
      // Log error for debugging
      console.error('Failed to create user:', error);
    }},
  }});
}}

/**
 * Hook to update an existing user
 *
 * @example
 * ```tsx
 * function EditUserForm({{ userId, initialData }}: {{
 *   userId: string;
 *   initialData: User
 * }}) {{
 *   const updateUser = useUpdateUser();
 *
 *   const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {{
 *     e.preventDefault();
 *     const formData = new FormData(e.currentTarget);
 *
 *     try {{
 *       await updateUser.mutateAsync({{
 *         id: userId,
 *         name: formData.get('name') as string,
 *         email: formData.get('email') as string,
 *       }});
 *       alert('User updated successfully!');
 *     }} catch (error) {{
 *       alert(`Failed to update user: ${{formatApiError(error)}}`);
 *     }}
 *   }};
 *
 *   return (
 *     <form onSubmit={{handleSubmit}}>
 *       <input name="name" defaultValue={{initialData.name}} required />
 *       <input name="email" type="email" defaultValue={{initialData.email}} required />
 *       <button type="submit" disabled={{updateUser.isPending}}>
 *         {{updateUser.isPending ? 'Updating...' : 'Update User'}}
 *       </button>
 *     </form>
 *   );
 * }}
 * ```
 */
export function useUpdateUser() {{
  const queryClient = useQueryClient();

  return useMutation<User, ApiError, {{ id: string }} & UpdateUserRequest>({{
    mutationFn: async ({{ id, ...userData }}) => {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users/${{id}}`, {{
        method: 'PATCH',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify(userData),
      }});

      if (!response.ok) {{
        const errorBody = await response.json().catch(() => ({{}}));
        throw new Error(`Failed to update user: ${{response.status}} ${{response.statusText}}`);
      }}

      return response.json();
    }},
    onSuccess: (_, variables) => {{
      // Invalidate both the specific user and the users list
      queryClient.invalidateQueries({{ queryKey: ['users', variables.id] }});
      queryClient.invalidateQueries({{ queryKey: ['users'] }});
    }},
    onError: (error) => {{
      console.error('Failed to update user:', error);
    }},
  }});
}}

/**
 * Hook to delete a user
 */
export function useDeleteUser() {{
  const queryClient = useQueryClient();

  return useMutation<void, ApiError, string>({{
    mutationFn: async (id) => {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users/${{id}}`, {{
        method: 'DELETE',
      }});

      if (!response.ok) {{
        const errorBody = await response.json().catch(() => ({{}}));
        throw new Error(`Failed to delete user: ${{response.status}} ${{response.statusText}}`);
      }}
    }},
    onSuccess: () => {{
      queryClient.invalidateQueries({{ queryKey: ['users'] }});
    }},
  }});
}}
"#,
        base_url
    );

    fs::write(output_dir.join("hooks.ts"), react_query_hooks)?;

    // SWR hooks example
    let swr_hooks = format!(
        r#"// SWR hooks for MockForge API
import useSWR from 'swr';
import useSWRMutation from 'swr/mutation';

const MOCKFORGE_BASE_URL = '{}';

const fetcher = (url: string) => fetch(url).then(res => res.json());

// Example: Get users with SWR
export function useUsersSWR() {{
  const {{ data, error, isLoading }} = useSWR(
    `${{MOCKFORGE_BASE_URL}}/api/users`,
    fetcher
  );

  return {{ users: data, error, isLoading }};
}}

// Example: Get user by ID with SWR
export function useUserSWR(id: string) {{
  const {{ data, error, isLoading }} = useSWR(
    id ? `${{MOCKFORGE_BASE_URL}}/api/users/${{id}}` : null,
    fetcher
  );

  return {{ user: data, error, isLoading }};
}}

// Example: Create user mutation with SWR
export function useCreateUserSWR() {{
  const {{ trigger, isMutating }} = useSWRMutation(
    `${{MOCKFORGE_BASE_URL}}/api/users`,
    async (url, {{ arg }}: {{ name: string; email: string }}) => {{
      const response = await fetch(url, {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify(arg),
      }});
      if (!response.ok) throw new Error('Failed to create user');
      return response.json();
    }}
  );

  return {{ createUser: trigger, isCreating: isMutating }};
}}
"#,
        base_url
    );

    fs::write(output_dir.join("hooks-swr.ts"), swr_hooks)?;

    // Generate example component
    let example_component = r#"// Example React component using MockForge hooks
// This demonstrates how to use the generated hooks in a real component

import React from 'react';
import { useUsers, useCreateUser, formatApiError } from './hooks';

/**
 * Example UserList component
 *
 * This component demonstrates:
 * - Fetching data with useUsers hook
 * - Creating new users with useCreateUser hook
 * - Error handling and loading states
 * - TypeScript type safety
 */
export function UserList() {
  const { data: users, isLoading, error, refetch } = useUsers();
  const createUser = useCreateUser();

  const handleCreateUser = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);

    try {
      await createUser.mutateAsync({
        name: formData.get('name') as string,
        email: formData.get('email') as string,
      });
      // Form will be reset by the form's reset() method
      e.currentTarget.reset();
    } catch (error) {
      // Error is already handled by the mutation's onError
      console.error('Failed to create user:', error);
    }
  };

  if (isLoading) {
    return (
      <div style={ padding: '20px' }>
        <div>Loading users...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div style={ padding: '20px', color: 'red' }>
        <h2>Error loading users</h2>
        <p>{formatApiError(error)}</p>
        <button onClick={() => refetch()}>Retry</button>
      </div>
    );
  }

  return (
    <div style={ padding: '20px' }>
      <h1>Users</h1>

      <form onSubmit={handleCreateUser} style={ marginBottom: '20px', padding: '10px', border: '1px solid #ccc' }>
        <h3>Create New User</h3>
        <div style={ marginBottom: '10px' }>
          <input
            name="name"
            placeholder="Name"
            required
            style={ padding: '8px', width: '200px', marginRight: '10px' }
          />
          <input
            name="email"
            type="email"
            placeholder="Email"
            required
            style={ padding: '8px', width: '200px', marginRight: '10px' }
          />
          <button
            type="submit"
            disabled={createUser.isPending}
            style={ padding: '8px 16px' }
          >
            {createUser.isPending ? 'Creating...' : 'Create User'}
          </button>
        </div>
        {createUser.error && (
          <div style={ color: 'red', fontSize: '14px' }>
            Error: {formatApiError(createUser.error)}
          </div>
        )}
      </form>

      <div>
        <h3>User List</h3>
        {users && users.length > 0 ? (
          <ul style={ listStyle: 'none', padding: 0 }>
            {{users.map((user) => (
              <li
                key={{user.id}}
                style={{ padding: '10px', marginBottom: '10px', border: '1px solid #ddd', borderRadius: '4px' }}
              >
                <strong>{{user.name}}</strong> - {{user.email}}
                {{user.createdAt && (
                  <span style={{ color: '#666', fontSize: '12px', marginLeft: '10px' }}>
                    (Created: {{new Date(user.createdAt).toLocaleDateString()}})
                  </span>
                )}}
              </li>
            )}}
          </ul>
        ) : (
          <p>No users found. Create one above!</p>
        )}
      </div>
    </div>
  );
}

export default UserList;
"#.to_string();

    fs::write(output_dir.join("UserList.example.tsx"), example_component)?;

    // Generate TypeScript types file (basic structure)
    let types_file = r#"// TypeScript type definitions for MockForge API
// These types are generated from your OpenAPI specification
// Update this file if your API schema changes

/**
 * Base API error response
 */
export interface ApiError {
  status: number;
  statusText: string;
  body?: any;
  message?: string;
}

/**
 * User entity
 */
export interface User {
  id: string;
  name: string;
  email: string;
  createdAt?: string;
  updatedAt?: string;
}

/**
 * Request to create a new user
 */
export interface CreateUserRequest {
  name: string;
  email: string;
}

/**
 * Request to update an existing user
 */
export interface UpdateUserRequest {
  name?: string;
  email?: string;
}

/**
 * API response wrapper (if your API uses this format)
 */
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
    details?: any;
  };
}

/**
 * Paginated response (if your API supports pagination)
 */
export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}
"#
    .to_string();

    fs::write(output_dir.join("types.ts"), types_file)?;

    // Generate index file for easy imports
    let index_file = r#"// MockForge API Client - Main Export
// Import hooks and utilities from this file

export * from './hooks';
export * from './types';

// Re-export error utilities for convenience
export { formatApiError, isNetworkError, isClientError, isServerError } from './hooks';
"#;

    fs::write(output_dir.join("index.ts"), index_file)?;

    Ok(())
}

/// Generate Vue/Nuxt examples
fn generate_vue_examples(output_dir: &Path, base_url: &str) -> anyhow::Result<()> {
    // Vue 3 composables (works with both Vue 3 and Nuxt 3)
    let composables = format!(
        r#"// Vue composables for MockForge API
import {{ ref, computed }} from 'vue';
import type {{ Ref }} from 'vue';

const MOCKFORGE_BASE_URL = '{}';

// Example: Get users composable
export function useUsers() {{
  const users = ref([]);
  const loading = ref(false);
  const error = ref<Error | null>(null);

  const fetchUsers = async () => {{
    loading.value = true;
    error.value = null;

    try {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users`);
      if (!response.ok) throw new Error('Failed to fetch users');
      users.value = await response.json();
    }} catch (e) {{
      error.value = e as Error;
    }} finally {{
      loading.value = false;
    }}
  }};

  // Auto-fetch on first use
  fetchUsers();

  return {{
    users: computed(() => users.value),
    loading: computed(() => loading.value),
    error: computed(() => error.value),
    refresh: fetchUsers,
  }};
}}

// Example: Get user by ID composable
export function useUser(id: Ref<string> | string) {{
  const userId = typeof id === 'string' ? ref(id) : id;
  const user = ref(null);
  const loading = ref(false);
  const error = ref<Error | null>(null);

  const fetchUser = async () => {{
    if (!userId.value) return;

    loading.value = true;
    error.value = null;

    try {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users/${{userId.value}}`);
      if (!response.ok) throw new Error('Failed to fetch user');
      user.value = await response.json();
    }} catch (e) {{
      error.value = e as Error;
    }} finally {{
      loading.value = false;
    }}
  }};

  // Auto-fetch when id changes (requires watch import)
  // import {{ watch }} from 'vue';
  // watch(userId, fetchUser, {{ immediate: true }});

  // For now, manually call fetchUser() when needed
  fetchUser();

  return {{
    user: computed(() => user.value),
    loading: computed(() => loading.value),
    error: computed(() => error.value),
    refresh: fetchUser,
  }};
}}

// Example: Create user composable
export function useCreateUser() {{
  const creating = ref(false);
  const error = ref<Error | null>(null);

  const createUser = async (userData: {{ name: string; email: string }}) => {{
    creating.value = true;
    error.value = null;

    try {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users`, {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify(userData),
      }});
      if (!response.ok) throw new Error('Failed to create user');
      return await response.json();
    }} catch (e) {{
      error.value = e as Error;
      throw e;
    }} finally {{
      creating.value = false;
    }}
  }};

  return {{
    createUser,
    creating: computed(() => creating.value),
    error: computed(() => error.value),
  }};
}}
"#,
        base_url
    );

    fs::write(output_dir.join("composables.ts"), composables)?;

    // Nuxt 3 specific composables (using useFetch)
    let nuxt_composables = format!(
        r#"// Nuxt 3 composables for MockForge API (using useFetch)
// Note: This file is for Nuxt 3 projects. For Vue 3, use composables.ts

import {{ ref, computed }} from 'vue';
import type {{ Ref }} from 'vue';

const MOCKFORGE_BASE_URL = '{}';

// Example: Get users composable (Nuxt 3)
export function useUsersNuxt() {{
    const {{ data, error, pending, refresh }} = useLazyFetch(
        `${{MOCKFORGE_BASE_URL}}/api/users`
    );

    return {{
        users: computed(() => data.value),
        error,
        loading: pending,
        refresh,
    }};
}}

// Example: Get user by ID composable (Nuxt 3)
export function useUserNuxt(id: Ref<string> | string) {{
    const userId = typeof id === 'string' ? ref(id) : id;

  const {{ data, error, pending, refresh }} = useLazyFetch(
    computed(() => `${{MOCKFORGE_BASE_URL}}/api/users/${{userId.value}}`)
  );

  return {{
    user: computed(() => data.value),
    error,
    loading: pending,
    refresh,
  }};
}}

// Example: Create user composable (Nuxt 3)
export function useCreateUserNuxt() {{
  const creating = ref(false);
  const error = ref<Error | null>(null);

  const createUser = async (userData: {{ name: string; email: string }}) => {{
    creating.value = true;
    error.value = null;

    try {{
      const response = await $fetch(`${{MOCKFORGE_BASE_URL}}/api/users`, {{
        method: 'POST',
        body: userData,
      }});
      return response;
    }} catch (e) {{
      error.value = e as Error;
      throw e;
    }} finally {{
      creating.value = false;
    }}
  }};

  return {{
    createUser,
    creating: computed(() => creating.value),
    error: computed(() => error.value),
  }};
}}
"#,
        base_url
    );

    fs::write(output_dir.join("composables-nuxt.ts"), nuxt_composables)?;

    Ok(())
}

/// Generate Angular examples
fn generate_angular_examples(output_dir: &Path, base_url: &str) -> anyhow::Result<()> {
    let service = format!(
        r#"// Angular service for MockForge API
import {{ Injectable }} from '@angular/core';
import {{ HttpClient, HttpParams }} from '@angular/common/http';
import {{ Observable }} from 'rxjs';
import {{ map }} from 'rxjs/operators';

const MOCKFORGE_BASE_URL = '{}';

export interface User {{
  id: string;
  name: string;
  email: string;
}}

@Injectable({{
  providedIn: 'root'
}})
export class MockForgeService {{
  constructor(private http: HttpClient) {{}}

  // Example: Get users
  getUsers(): Observable<User[]> {{
    return this.http.get<User[]>(`${{MOCKFORGE_BASE_URL}}/api/users`);
  }}

  // Example: Get user by ID
  getUser(id: string): Observable<User> {{
    return this.http.get<User>(`${{MOCKFORGE_BASE_URL}}/api/users/${{id}}`);
  }}

  // Example: Create user
  createUser(userData: {{ name: string; email: string }}): Observable<User> {{
    return this.http.post<User>(`${{MOCKFORGE_BASE_URL}}/api/users`, userData);
  }}

  // Example: Update user
  updateUser(id: string, userData: Partial<User>): Observable<User> {{
    return this.http.patch<User>(`${{MOCKFORGE_BASE_URL}}/api/users/${{id}}`, userData);
  }}

  // Example: Delete user
  deleteUser(id: string): Observable<void> {{
    return this.http.delete<void>(`${{MOCKFORGE_BASE_URL}}/api/users/${{id}}`);
  }}
}}
"#,
        base_url
    );

    fs::write(output_dir.join("mockforge.service.ts"), service)?;

    Ok(())
}

/// Generate Svelte examples
fn generate_svelte_examples(output_dir: &Path, base_url: &str) -> anyhow::Result<()> {
    let stores = format!(
        r#"// Svelte stores for MockForge API
import {{ writable, derived }} from 'svelte/store';

const MOCKFORGE_BASE_URL = '{}';

// Example: Users store
export const users = writable([]);
export const usersLoading = writable(false);
export const usersError = writable(null);

export async function fetchUsers() {{
  usersLoading.set(true);
  usersError.set(null);

  try {{
    const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users`);
    if (!response.ok) throw new Error('Failed to fetch users');
    const data = await response.json();
    users.set(data);
  }} catch (error) {{
    usersError.set(error);
  }} finally {{
    usersLoading.set(false);
  }}
}}

// Example: User store
export const user = writable(null);
export const userLoading = writable(false);
export const userError = writable(null);

export async function fetchUser(id: string) {{
  userLoading.set(true);
  userError.set(null);

  try {{
    const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users/${{id}}`);
    if (!response.ok) throw new Error('Failed to fetch user');
    const data = await response.json();
    user.set(data);
  }} catch (error) {{
    userError.set(error);
  }} finally {{
    userLoading.set(false);
  }}
}}

// Example: Create user function
export async function createUser(userData: {{ name: string; email: string }}) {{
  try {{
    const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users`, {{
      method: 'POST',
      headers: {{ 'Content-Type': 'application/json' }},
      body: JSON.stringify(userData),
    }});
    if (!response.ok) throw new Error('Failed to create user');
    const data = await response.json();
    // Refresh users list
    await fetchUsers();
    return data;
  }} catch (error) {{
    console.error('Error creating user:', error);
    throw error;
  }}
}}
"#,
        base_url
    );

    fs::write(output_dir.join("stores.ts"), stores)?;

    Ok(())
}

/// Create .env.mockforge.example file
fn create_env_example(
    project_root: &Path,
    base_url: &str,
    reality_level: &str,
) -> anyhow::Result<()> {
    let env_content = format!(
        r#"# MockForge Configuration
# Copy this file to .env.mockforge and customize as needed

# Base URL for MockForge server
MOCKFORGE_BASE_URL={}

# Reality level: static, light, moderate, high, chaos
MOCKFORGE_REALITY_LEVEL={}

# Optional: API key if authentication is enabled
# MOCKFORGE_API_KEY=your-api-key-here

# Optional: Workspace ID
# MOCKFORGE_WORKSPACE_ID=default
"#,
        base_url, reality_level
    );

    let env_path = project_root.join(".env.mockforge.example");
    fs::write(&env_path, env_content)?;

    Ok(())
}

/// Update package.json with SDK dependency
fn update_package_json(package_json_path: &Path, framework: Framework) -> anyhow::Result<()> {
    let content = fs::read_to_string(package_json_path)?;
    let mut package_json: serde_json::Value = serde_json::from_str(&content)?;

    // Add SDK dependency
    if let Some(deps) = package_json.get_mut("dependencies").and_then(|d| d.as_object_mut()) {
        deps.insert(
            framework.sdk_package().to_string(),
            serde_json::Value::String("latest".to_string()),
        );
    } else {
        let mut deps = serde_json::Map::new();
        deps.insert(
            framework.sdk_package().to_string(),
            serde_json::Value::String("latest".to_string()),
        );
        package_json["dependencies"] = serde_json::Value::Object(deps);
    }

    // Add React Query for React/Next
    if matches!(framework, Framework::React | Framework::Next) {
        if let Some(deps) = package_json.get_mut("dependencies").and_then(|d| d.as_object_mut()) {
            deps.insert(
                "@tanstack/react-query".to_string(),
                serde_json::Value::String("^5.0.0".to_string()),
            );
        }
    }

    // Add SWR for React/Next (optional)
    if matches!(framework, Framework::React | Framework::Next) {
        if let Some(deps) = package_json.get_mut("dependencies").and_then(|d| d.as_object_mut()) {
            deps.insert("swr".to_string(), serde_json::Value::String("^2.0.0".to_string()));
        }
    }

    let updated_content = serde_json::to_string_pretty(&package_json)?;
    fs::write(package_json_path, updated_content)?;

    Ok(())
}

/// Find tsconfig.json in project
fn find_tsconfig(project_root: &Path) -> Option<PathBuf> {
    let paths = [
        project_root.join("tsconfig.json"),
        project_root.join("tsconfig.app.json"),
        project_root.join("tsconfig.base.json"),
    ];

    for path in &paths {
        if path.exists() {
            return Some(path.clone());
        }
    }

    None
}

/// Verify TypeScript compilation (if tsc is available)
fn verify_typescript_compilation(tsconfig_path: &Path) -> anyhow::Result<()> {
    use std::process::Command;

    // Check if tsc is available
    let tsc_check = Command::new("tsc").arg("--version").output();

    if tsc_check.is_err() {
        return Err(anyhow::anyhow!("tsc not found"));
    }

    // Try to compile the generated files
    let project_dir = tsconfig_path.parent().unwrap_or(Path::new("."));
    let compile_result = Command::new("tsc")
        .arg("--noEmit")
        .arg("--project")
        .arg(tsconfig_path)
        .current_dir(project_dir)
        .output();

    match compile_result {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Don't fail the whole setup, just warn
                println!("  ⚠️  TypeScript compilation warnings:");
                println!("     {}", stderr.lines().take(5).collect::<Vec<_>>().join("\n     "));
                Ok(())
            }
        }
        Err(_) => Err(anyhow::anyhow!("Failed to run tsc")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Framework tests
    #[test]
    fn test_framework_from_str_react() {
        assert_eq!(Framework::from_str("react"), Some(Framework::React));
        assert_eq!(Framework::from_str("React"), Some(Framework::React));
        assert_eq!(Framework::from_str("REACT"), Some(Framework::React));
    }

    #[test]
    fn test_framework_from_str_vue() {
        assert_eq!(Framework::from_str("vue"), Some(Framework::Vue));
        assert_eq!(Framework::from_str("Vue"), Some(Framework::Vue));
    }

    #[test]
    fn test_framework_from_str_angular() {
        assert_eq!(Framework::from_str("angular"), Some(Framework::Angular));
        assert_eq!(Framework::from_str("Angular"), Some(Framework::Angular));
    }

    #[test]
    fn test_framework_from_str_svelte() {
        assert_eq!(Framework::from_str("svelte"), Some(Framework::Svelte));
        assert_eq!(Framework::from_str("Svelte"), Some(Framework::Svelte));
    }

    #[test]
    fn test_framework_from_str_next() {
        assert_eq!(Framework::from_str("next"), Some(Framework::Next));
        assert_eq!(Framework::from_str("Next"), Some(Framework::Next));
    }

    #[test]
    fn test_framework_from_str_nuxt() {
        assert_eq!(Framework::from_str("nuxt"), Some(Framework::Nuxt));
        assert_eq!(Framework::from_str("Nuxt"), Some(Framework::Nuxt));
    }

    #[test]
    fn test_framework_from_str_invalid() {
        assert_eq!(Framework::from_str("invalid"), None);
        assert_eq!(Framework::from_str(""), None);
        assert_eq!(Framework::from_str("reactjs"), None);
    }

    #[test]
    fn test_framework_name() {
        assert_eq!(Framework::React.name(), "react");
        assert_eq!(Framework::Vue.name(), "vue");
        assert_eq!(Framework::Angular.name(), "angular");
        assert_eq!(Framework::Svelte.name(), "svelte");
        assert_eq!(Framework::Next.name(), "next");
        assert_eq!(Framework::Nuxt.name(), "nuxt");
    }

    #[test]
    fn test_framework_sdk_package() {
        assert_eq!(Framework::React.sdk_package(), "@mockforge/sdk");
        assert_eq!(Framework::Vue.sdk_package(), "@mockforge/sdk");
        assert_eq!(Framework::Angular.sdk_package(), "@mockforge/sdk");
        assert_eq!(Framework::Svelte.sdk_package(), "@mockforge/sdk");
        assert_eq!(Framework::Next.sdk_package(), "@mockforge/sdk");
        assert_eq!(Framework::Nuxt.sdk_package(), "@mockforge/sdk");
    }

    // detect_project_root tests
    #[test]
    fn test_detect_project_root_with_package_json() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("package.json"), "{}").unwrap();

        // Change to temp dir and test
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).ok();
        let result = detect_project_root();
        std::env::set_current_dir(original_dir).ok();

        assert!(result.is_ok());
        let root = result.unwrap();
        assert!(root.join("package.json").exists());
    }

    #[test]
    fn test_detect_project_root_returns_pathbuf() {
        let result = detect_project_root();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.is_absolute() || path.is_relative());
    }

    // find_package_json tests
    #[test]
    fn test_find_package_json_exists() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("package.json"), "{}").unwrap();

        let result = find_package_json(temp_dir.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir.path().join("package.json"));
    }

    #[test]
    fn test_find_package_json_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_package_json(temp_dir.path());
        assert!(result.is_none());
    }

    // find_tsconfig tests
    #[test]
    fn test_find_tsconfig_exists() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("tsconfig.json"), "{}").unwrap();

        let result = find_tsconfig(temp_dir.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir.path().join("tsconfig.json"));
    }

    #[test]
    fn test_find_tsconfig_app_json() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("tsconfig.app.json"), "{}").unwrap();

        let result = find_tsconfig(temp_dir.path());
        assert!(result.is_some());
    }

    #[test]
    fn test_find_tsconfig_base_json() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("tsconfig.base.json"), "{}").unwrap();

        let result = find_tsconfig(temp_dir.path());
        assert!(result.is_some());
    }

    #[test]
    fn test_find_tsconfig_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_tsconfig(temp_dir.path());
        assert!(result.is_none());
    }

    // is_valid_openapi_spec tests
    #[test]
    fn test_is_valid_openapi_spec_json() {
        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("openapi.json");
        fs::write(
            &spec_path,
            r#"{"openapi": "3.0.0", "info": {"title": "Test", "version": "1.0.0"}, "paths": {}}"#,
        )
        .unwrap();

        let result = is_valid_openapi_spec(&spec_path);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_is_valid_openapi_spec_swagger() {
        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("swagger.json");
        fs::write(
            &spec_path,
            r#"{"swagger": "2.0", "info": {"title": "Test", "version": "1.0.0"}, "paths": {}}"#,
        )
        .unwrap();

        let result = is_valid_openapi_spec(&spec_path);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_is_valid_openapi_spec_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("openapi.yaml");
        fs::write(&spec_path, "openapi: 3.0.0\ninfo:\n  title: Test\n  version: 1.0.0\npaths: {}")
            .unwrap();

        let result = is_valid_openapi_spec(&spec_path);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_is_valid_openapi_spec_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("invalid.json");
        fs::write(&spec_path, r#"{"not": "openapi"}"#).unwrap();

        let result = is_valid_openapi_spec(&spec_path);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_is_valid_openapi_spec_file_not_found() {
        let path = PathBuf::from("/nonexistent/spec.yaml");
        let result = is_valid_openapi_spec(&path);
        assert!(result.is_err());
    }

    // check_existing_client_code tests
    #[test]
    fn test_check_existing_client_code_none() {
        let temp_dir = TempDir::new().unwrap();
        let result = check_existing_client_code(temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_check_existing_client_code_with_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("client.ts"), "// client code").unwrap();
        fs::write(temp_dir.path().join("hooks.ts"), "// hooks code").unwrap();

        let result = check_existing_client_code(temp_dir.path());
        assert!(result.is_ok());
        let files = result.unwrap();
        assert!(files.contains(&"client.ts".to_string()));
        assert!(files.contains(&"hooks.ts".to_string()));
    }

    #[test]
    fn test_check_existing_client_code_directory_not_exists() {
        let path = PathBuf::from("/nonexistent/directory");
        let result = check_existing_client_code(&path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // DevSetupArgs tests
    #[test]
    fn test_dev_setup_args_default_values() {
        // Test that default values are correct
        let args = DevSetupArgs {
            framework: "react".to_string(),
            base_url: "http://localhost:3000".to_string(),
            reality_level: "moderate".to_string(),
            spec: None,
            output: PathBuf::from("./src/mockforge"),
            force: false,
        };

        assert_eq!(args.framework, "react");
        assert_eq!(args.base_url, "http://localhost:3000");
        assert_eq!(args.reality_level, "moderate");
        assert!(!args.force);
    }

    #[test]
    fn test_dev_setup_args_with_spec() {
        let args = DevSetupArgs {
            framework: "vue".to_string(),
            base_url: "http://localhost:3000".to_string(),
            reality_level: "moderate".to_string(),
            spec: Some(PathBuf::from("./openapi.yaml")),
            output: PathBuf::from("./src/mockforge"),
            force: false,
        };

        assert!(args.spec.is_some());
        assert_eq!(args.spec.unwrap(), PathBuf::from("./openapi.yaml"));
    }

    #[test]
    fn test_dev_setup_args_with_force() {
        let args = DevSetupArgs {
            framework: "angular".to_string(),
            base_url: "http://localhost:3000".to_string(),
            reality_level: "moderate".to_string(),
            spec: None,
            output: PathBuf::from("./src/mockforge"),
            force: true,
        };

        assert!(args.force);
    }

    // Framework enum tests
    #[test]
    fn test_framework_debug() {
        let framework = Framework::React;
        let debug = format!("{:?}", framework);
        assert!(debug.contains("React"));
    }

    #[test]
    fn test_framework_clone() {
        let framework = Framework::Vue;
        let cloned = framework;
        assert_eq!(framework as i32, cloned as i32);
    }

    #[test]
    fn test_framework_copy() {
        let framework = Framework::Angular;
        let copied = framework;
        // Both should be usable
        assert_eq!(framework.name(), "angular");
        assert_eq!(copied.name(), "angular");
    }

    #[test]
    fn test_framework_partial_eq() {
        assert_eq!(Framework::React, Framework::React);
        assert_ne!(Framework::React, Framework::Vue);
        assert_ne!(Framework::Vue, Framework::Angular);
    }

    #[test]
    fn test_framework_eq() {
        let f1 = Framework::Svelte;
        let f2 = Framework::Svelte;
        assert_eq!(f1, f2);
    }

    // Integration tests for detect_mockforge_workspace
    #[test]
    fn test_detect_mockforge_workspace_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let result = detect_mockforge_workspace(temp_dir.path());
        assert!(result.is_ok());
        let (base_url, reality_level) = result.unwrap();
        assert!(base_url.is_none());
        assert!(reality_level.is_none());
    }

    #[test]
    fn test_detect_mockforge_workspace_with_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
http:
  host: localhost
  port: 8080
reality:
  level: high
"#;
        fs::write(temp_dir.path().join("mockforge.yaml"), config_content).unwrap();

        let result = detect_mockforge_workspace(temp_dir.path());
        assert!(result.is_ok());
        let (base_url, reality_level) = result.unwrap();
        assert!(base_url.is_some());
        assert_eq!(base_url.unwrap(), "http://localhost:8080");
        assert!(reality_level.is_some());
        assert_eq!(reality_level.unwrap(), "high");
    }

    #[test]
    fn test_detect_mockforge_workspace_converts_0_0_0_0() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
http:
  host: 0.0.0.0
  port: 3000
"#;
        fs::write(temp_dir.path().join("mockforge.yaml"), config_content).unwrap();

        let result = detect_mockforge_workspace(temp_dir.path());
        assert!(result.is_ok());
        let (base_url, _) = result.unwrap();
        assert!(base_url.is_some());
        // 0.0.0.0 should be converted to localhost
        assert_eq!(base_url.unwrap(), "http://localhost:3000");
    }
}
