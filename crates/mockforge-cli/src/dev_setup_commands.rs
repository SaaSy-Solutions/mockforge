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

    // Create output directory
    let output_dir = project_root.join(&args.output);
    fs::create_dir_all(&output_dir)?;
    println!("  ✓ Created output directory: {}", output_dir.display());

    // Generate client if spec provided
    if let Some(spec_path) = &args.spec {
        println!("  📦 Generating typed client from OpenAPI spec...");
        generate_client(framework, spec_path, &output_dir, &args.base_url).await?;
        println!("  ✓ Client generated");
    } else {
        println!("  ⚠️  No OpenAPI spec provided, skipping client generation");
        println!("     Use --spec <path> to generate typed client");
    }

    // Generate framework-specific hooks/composables/services
    println!("  📝 Generating {} examples...", framework.name());
    generate_framework_examples(framework, &output_dir, &args.base_url)?;
    println!("  ✓ Examples generated");

    // Create .env.mockforge.example
    println!("  🔧 Creating environment configuration...");
    create_env_example(&project_root, &args.base_url, &args.reality_level)?;
    println!("  ✓ Environment configuration created");

    // Update package.json if it exists
    if let Some(package_json_path) = find_package_json(&project_root) {
        println!("  📦 Updating package.json...");
        update_package_json(&package_json_path, framework)?;
        println!("  ✓ package.json updated");
    }

    println!("\n✅ MockForge setup complete!");
    println!("\nNext steps:");
    println!("  1. Copy .env.mockforge.example to .env.mockforge");
    println!("  2. Review generated files in {}", output_dir.display());
    println!("  3. Import and use the generated hooks/composables in your app");
    println!("  4. Start MockForge server: mockforge serve");

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
    // React Query hooks example
    let react_query_hooks = format!(
        r#"// React Query hooks for MockForge API
import {{ useQuery, useMutation, useQueryClient }} from '@tanstack/react-query';
import {{ mockforgeClient }} from './client';

const MOCKFORGE_BASE_URL = '{}';

// Example: Get users
export function useUsers() {{
  return useQuery({{
    queryKey: ['users'],
    queryFn: async () => {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users`);
      if (!response.ok) throw new Error('Failed to fetch users');
      return response.json();
    }},
  }});
}}

// Example: Get user by ID
export function useUser(id: string) {{
  return useQuery({{
    queryKey: ['users', id],
    queryFn: async () => {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users/${{id}}`);
      if (!response.ok) throw new Error('Failed to fetch user');
      return response.json();
    }},
    enabled: !!id,
  }});
}}

// Example: Create user mutation
export function useCreateUser() {{
  const queryClient = useQueryClient();

  return useMutation({{
    mutationFn: async (userData: {{ name: string; email: string }}) => {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users`, {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify(userData),
      }});
      if (!response.ok) throw new Error('Failed to create user');
      return response.json();
    }},
    onSuccess: () => {{
      queryClient.invalidateQueries({{ queryKey: ['users'] }});
    }},
  }});
}}

// Example: Update user mutation
export function useUpdateUser() {{
  const queryClient = useQueryClient();

  return useMutation({{
    mutationFn: async ({{ id, ...userData }}: {{ id: string; name?: string; email?: string }}) => {{
      const response = await fetch(`${{MOCKFORGE_BASE_URL}}/api/users/${{id}}`, {{
        method: 'PATCH',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify(userData),
      }});
      if (!response.ok) throw new Error('Failed to update user');
      return response.json();
    }},
    onSuccess: (_, variables) => {{
      queryClient.invalidateQueries({{ queryKey: ['users', variables.id] }});
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
