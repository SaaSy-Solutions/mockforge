//! Bound file input and publish complete artifacts without overwriting a winner.
use super::{document, Result};
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

const MAX_BYTES: u64 = 8 * 1024 * 1024;

pub(super) fn format(path: &Path) -> Result<&'static str> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => Ok("json"),
        Some("yaml" | "yml") => Ok("yaml"),
        _ => Err("Output must have a .json, .yaml or .yml extension".into()),
    }
}

fn read(path: &Path) -> Result<String> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err("AI bake input/output must be a regular file, not a symlink or device".into());
    }
    let mut text = String::new();
    fs::File::open(path)?.take(MAX_BYTES + 1).read_to_string(&mut text)?;
    if text.len() as u64 > MAX_BYTES {
        return Err("AI bake file exceeds the 8 MiB limit".into());
    }
    Ok(text)
}

pub(super) fn read_prompt(path: &Path) -> Result<String> {
    let text = read(path)?;
    if text.trim().is_empty() {
        return Err("The prompt file is empty".into());
    }
    Ok(text)
}

pub(super) fn load(path: &Path, identity: &str) -> Result<Option<Value>> {
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let text = read(path)?;
    let value = if format(path)? == "json" {
        serde_json::from_str(&text)?
    } else {
        serde_yaml::from_str(&text)?
    };
    Ok(Some(document::verify(value, identity)?))
}

pub(super) fn publish(path: &Path, value: &Value, identity: &str) -> Result<()> {
    let bytes = if format(path)? == "json" {
        serde_json::to_vec_pretty(value)?
    } else {
        serde_yaml::to_string(value)?.into_bytes()
    };
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Generated fixture exceeds the 8 MiB limit; no fixture was written".into());
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    if let Err(error) = temporary.persist_noclobber(path) {
        if error.error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(error.error.into());
        }
        load(path, identity)?.ok_or("Concurrent fixture publication disappeared")?;
    }
    Ok(())
}
