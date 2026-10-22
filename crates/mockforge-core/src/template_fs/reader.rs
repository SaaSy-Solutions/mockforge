//! Capability-backed opening and bounded reads of policy-approved files.
use super::{
    path_policy::{is_denied, validate_requested},
    TemplateFsError,
};
use cap_fs_ext::{
    DirExt, FollowSymlinks, OpenOptionsFollowExt, OpenOptionsMaybeDirExt, OpenOptionsSyncExt,
};
use cap_std::fs::{Dir, OpenOptions};
use std::{
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

/// Open a previously resolved path through pinned directory handles. A concurrent
/// symlink replacement must not redirect the read after its policy checks.
pub(super) fn open_confined_file(root: &Path, resolved: &Path) -> Result<File, TemplateFsError> {
    let relative = resolved.strip_prefix(root).map_err(|_| TemplateFsError::OutsideRoot)?;
    let name = relative.file_name().ok_or(TemplateFsError::NotFound)?;
    let mut directory = Dir::open_ambient_dir(root, cap_std::ambient_authority())
        .map_err(|_| TemplateFsError::InvalidRoot)?;
    if let Some(parent) = relative.parent() {
        for component in parent.components() {
            let Component::Normal(name) = component else {
                return Err(TemplateFsError::OutsideRoot);
            };
            directory = directory.open_dir_nofollow(name).map_err(|_| TemplateFsError::Denied)?;
        }
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No).nonblock(true).maybe_dir(true);
    directory
        .open_with(name, &options)
        .map(|file| file.into_std())
        .map_err(|_| TemplateFsError::Denied)
}

/// Resolve an allowed token under one usable canonical root.
fn resolve_path(root: &Path, requested: &Path) -> Result<(PathBuf, PathBuf), TemplateFsError> {
    let root = root.canonicalize().map_err(|_| TemplateFsError::InvalidRoot)?;
    if !root.is_dir() || root.parent().is_none() {
        // A missing root or `/` itself would make the sandbox meaningless.
        return Err(TemplateFsError::InvalidRoot);
    }

    let joined = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };
    if is_denied(&joined, &root) {
        return Err(TemplateFsError::Denied);
    }
    let resolved = joined.canonicalize().map_err(|_| {
        if joined.starts_with(&root) {
            TemplateFsError::NotFound
        } else {
            TemplateFsError::OutsideRoot
        }
    })?;
    if !resolved.starts_with(&root) {
        return Err(TemplateFsError::OutsideRoot);
    }
    if is_denied(&resolved, &root) {
        return Err(TemplateFsError::Denied);
    }

    Ok((root, resolved))
}

/// Read `raw` relative to `root`, enforcing every sandbox rule.
pub fn read_file_in_root(root: &Path, raw: &str, max: u64) -> Result<String, TemplateFsError> {
    let requested = validate_requested(raw)?;
    let (root, resolved) = resolve_path(root, requested)?;
    let file = open_confined_file(&root, &resolved)?;
    let meta = file.metadata().map_err(|_| TemplateFsError::NotFound)?;
    if !meta.is_file() {
        return Err(TemplateFsError::NotFound);
    }
    if meta.len() > max {
        return Err(TemplateFsError::TooLarge(max));
    }
    // Bound the read as well, in case the file grows after the metadata check.
    let mut buf = Vec::new();
    file.take(max.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(|_| TemplateFsError::ReadFailed)?;
    if buf.len() as u64 > max {
        return Err(TemplateFsError::TooLarge(max));
    }
    String::from_utf8(buf).map_err(|_| TemplateFsError::ReadFailed)
}
