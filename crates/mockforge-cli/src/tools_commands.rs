use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::Context;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ToolsCommands {
    /// Export bundled discussion and CVE tools (requires Python 3.11+ to run them)
    Export {
        /// New directory for the scripts, examples and instructions; never overwritten
        #[arg(long)]
        output_dir: PathBuf,
    },
}

pub fn handle(command: ToolsCommands) -> anyhow::Result<()> {
    match command {
        ToolsCommands::Export { output_dir } => {
            let files: BTreeMap<String, String> =
                serde_json::from_str(include_str!("bundled_tools.json"))?;
            fs::create_dir(&output_dir).with_context(|| {
                format!("Choose a new output directory: {}", output_dir.display())
            })?;
            for (name, contents) in files {
                let path = output_dir.join(name);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, contents)?;
            }
            fs::write(output_dir.join("VERSION"), concat!(env!("CARGO_PKG_VERSION"), "\n"))?;
            println!("Exported tools to {}. See README.md for commands.", output_dir.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_includes_tools_and_preserves_existing_directory() {
        let root = tempfile::tempdir().unwrap();
        let output_dir = root.path().join("tools");
        handle(ToolsCommands::Export {
            output_dir: output_dir.clone(),
        })
        .unwrap();
        assert!(output_dir.join("scripts/export_github_issue.py").is_file());
        assert!(output_dir.join("scripts/cve_pack.py").is_file());
        assert!(output_dir.join("examples/cve-packs/CVE-2025-29927/profile.json").is_file());
        fs::write(output_dir.join("README.md"), "user edits").unwrap();
        assert!(handle(ToolsCommands::Export {
            output_dir: output_dir.clone()
        })
        .is_err());
        assert_eq!(fs::read_to_string(output_dir.join("README.md")).unwrap(), "user edits");
    }
}
