//! Deterministic, bounded, read-only loading of the existing HTTP replay format.

use mockforge_core::{record_replay::RecordedRequest, Error, Result};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_FILES: usize = 10_000;

pub(super) fn load(path: &Path) -> Result<Vec<(String, RecordedRequest)>> {
    let root = if path.join("http").is_dir() {
        path.join("http")
    } else {
        path.to_path_buf()
    };
    let mut files = Vec::new();
    discover(&root, 0, &mut files)?;
    if files.is_empty() {
        return Err(Error::config("no HTTP JSON fixtures found"));
    }
    files.sort();
    files.into_iter().map(|path| read(&path)).collect()
}

pub(in crate::verify_mocks_commands) fn discover(
    path: &Path,
    depth: usize,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    if depth > 64 {
        return Err(Error::config("fixture directory depth exceeds 64"));
    }
    let metadata = fs::symlink_metadata(path).map_err(|e| {
        Error::config(format!("cannot inspect fixture path {}: {e}", path.display()))
    })?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            discover(&entry?.path(), depth + 1, files)?;
        }
    } else if metadata.is_file() {
        if path.extension().and_then(|x| x.to_str()) == Some("json") {
            if files.len() >= MAX_FILES {
                return Err(Error::config("fixture set exceeds 10000 JSON files"));
            }
            files.push(path.to_path_buf());
        }
    } else {
        return Err(Error::config(format!(
            "fixture path must be a regular file or directory: {}",
            path.display()
        )));
    }
    Ok(())
}

fn read(path: &Path) -> Result<(String, RecordedRequest)> {
    let data = read_bytes(path)?;
    let fixture = serde_json::from_slice(&data).map_err(|e| {
        Error::config(format!("invalid HTTP replay fixture {}: {e}", path.display()))
    })?;
    Ok((path.display().to_string(), fixture))
}

pub(in crate::verify_mocks_commands) fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(Error::config("fixture must be a regular file"));
    }
    let mut data = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut data)?;
    if data.len() as u64 > MAX_FILE_BYTES {
        return Err(Error::config(format!("fixture exceeds 8 MiB: {}", path.display())));
    }
    Ok(data)
}
