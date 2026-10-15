//! Local workspace configuration transfer without exporting credential values.

use clap::ValueEnum;
use mockforge_core::workspace_export::WorkspaceExportDocument;
use std::{error::Error, io::Write, path::Path};

pub type TransferResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum WorkspaceFileFormat {
    Json,
    Yaml,
}

fn local_endpoint(admin_url: &str, workspace: Option<&str>) -> TransferResult<url::Url> {
    let mut url = url::Url::parse(admin_url).map_err(|_| "Invalid local admin URL")?;
    let local = match url.host() {
        Some(url::Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };
    if !local
        || !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return Err("Workspace transfer requires a loopback admin URL without credentials, query, fragment, or path".into());
    }
    {
        let mut segments = url.path_segments_mut().map_err(|_| "Invalid local admin URL")?;
        segments.clear().push("__mockforge").push("workspaces");
        if let Some(workspace) = workspace {
            segments.push(workspace).push("export");
        } else {
            segments.push("import");
        }
    }
    Ok(url)
}

fn client() -> TransferResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|_| "Cannot create workspace transfer client")?)
}

fn token(name: &str) -> TransferResult<String> {
    std::env::var(name)
        .ok()
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| {
            "Set the selected token environment variable to a valid local admin JWT".into()
        })
}

pub async fn export_workspace(
    workspace_id: &str,
    admin_url: &str,
    token_env: &str,
    output: &Path,
    format: WorkspaceFileFormat,
    force: bool,
) -> TransferResult<()> {
    if !force && output.symlink_metadata().is_ok() {
        return Err("Output already exists; use --force to replace it".into());
    }
    let endpoint = local_endpoint(admin_url, Some(workspace_id))?;
    let response = client()?
        .get(endpoint)
        .bearer_auth(token(token_env)?)
        .send()
        .await
        .map_err(|_| "Local workspace export request failed")?;
    if !response.status().is_success() {
        return Err(format!("Workspace export failed: HTTP {}", response.status()).into());
    }
    let document: WorkspaceExportDocument =
        response.json().await.map_err(|_| "Invalid workspace export response")?;
    if document.format_version != 1 {
        return Err("Unsupported workspace export version".into());
    }
    let contents = match format {
        WorkspaceFileFormat::Json => {
            serde_json::to_string_pretty(&document)
                .map_err(|_| "Cannot serialize workspace JSON")?
                + "\n"
        }
        WorkspaceFileFormat::Yaml => {
            serde_yaml::to_string(&document).map_err(|_| "Cannot serialize workspace YAML")?
        }
    };
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| "Cannot create export file in output directory")?;
    temporary
        .write_all(contents.as_bytes())
        .map_err(|_| "Cannot write workspace export")?;
    temporary.as_file().sync_all().map_err(|_| "Cannot flush workspace export")?;
    if force {
        temporary.persist(output).map_err(|_| "Cannot replace workspace export file")?;
    } else {
        temporary
            .persist_noclobber(output)
            .map_err(|_| "Output exists or cannot be created")?;
    }
    println!("Workspace exported.");
    Ok(())
}

pub async fn import_workspace(
    input: &Path,
    workspace_id: &str,
    admin_url: &str,
    token_env: &str,
    resolve_env: bool,
) -> TransferResult<()> {
    let endpoint = local_endpoint(admin_url, None)?;
    let contents =
        std::fs::read_to_string(input).map_err(|_| "Cannot read workspace export file")?;
    let document: WorkspaceExportDocument = match input.extension().and_then(|value| value.to_str())
    {
        Some("json") => {
            serde_json::from_str(&contents).map_err(|_| "Invalid workspace JSON document")?
        }
        Some("yaml" | "yml") => {
            serde_yaml::from_str(&contents).map_err(|_| "Invalid workspace YAML document")?
        }
        _ => return Err("Import file must have a .json, .yaml, or .yml extension".into()),
    };
    let workspace = document.restore(|name| {
        let suffix = name.strip_prefix("MOCKFORGE_WORKSPACE_EXPORT_VALUE_")?;
        if !resolve_env || suffix.len() != 6 || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        std::env::var(name).ok()
    })?;
    let response = client()?
        .post(endpoint)
        .bearer_auth(token(token_env)?)
        .json(&serde_json::json!({"id":workspace_id,"workspace":workspace}))
        .send()
        .await
        .map_err(|_| "Local workspace import request failed")?;
    if !response.status().is_success() {
        return Err(format!("Workspace import failed: HTTP {}", response.status()).into());
    }
    println!("Workspace imported.");
    Ok(())
}

pub async fn activate_workspace(
    workspace_id: &str,
    admin_url: &str,
    token_env: &str,
) -> TransferResult<()> {
    let mut endpoint = local_endpoint(admin_url, Some(workspace_id))?;
    endpoint
        .path_segments_mut()
        .map_err(|_| "Invalid local admin URL")?
        .pop()
        .push("activate");
    let response = client()?
        .post(endpoint)
        .bearer_auth(token(token_env)?)
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|_| "Local workspace activation request failed")?;
    if !response.status().is_success() {
        return Err(format!("Workspace activation failed: HTTP {}", response.status()).into());
    }
    println!("Workspace activated under its /workspace/<id>/ path.");
    Ok(())
}
