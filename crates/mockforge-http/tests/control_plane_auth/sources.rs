use super::assembly::{manifest_dir, production_source, read};
use std::path::{Path, PathBuf};

/// Production source of every Rust file in the workspace's crates, read once
pub(super) fn workspace_sources() -> &'static Vec<(PathBuf, String)> {
    static SOURCES: std::sync::OnceLock<Vec<(PathBuf, String)>> = std::sync::OnceLock::new();
    SOURCES.get_or_init(|| {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                let name = entry.file_name();
                if path.is_dir() {
                    if name != "target" && name != "node_modules" && name != "tests" {
                        walk(&path, out);
                    }
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(path);
                }
            }
        }
        let mut files = Vec::new();
        // This crate first, so its definitions win over same-named ones elsewhere.
        walk(&manifest_dir().join("src"), &mut files);
        walk(&manifest_dir().join(".."), &mut files);
        files
            .into_iter()
            .map(|f| {
                let source = production_source(&read(&f));
                (f, source)
            })
            .collect()
    })
}

/// The body of `fn name`, looked up in this crate first, then the workspace
pub(super) fn router_fn_body(name: &str) -> Option<String> {
    let needles = [format!("fn {name}("), format!("fn {name}<")];
    for (_, source) in workspace_sources() {
        let Some(start) = needles.iter().find_map(|n| source.find(n.as_str())) else {
            continue;
        };
        let body_open = start + source[start..].find('{')?;
        let mut depth = 0usize;
        for (i, b) in source.bytes().enumerate().skip(body_open) {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(source[body_open..=i].to_string());
                    }
                }
                _ => {}
            }
        }
    }
    None
}
