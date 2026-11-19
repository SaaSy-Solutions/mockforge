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

/// Bootstrap run by `python3 -I -c`: its only argument is the directory the
/// bundled modules were written to. The module sources never travel on the
/// command line, so the bundle's size cannot hit the kernel's per-argument
/// limit (MAX_ARG_STRLEN, 128 KiB), which a single JSON argument did.
const CVE_BOOTSTRAP: &str = r#"import pathlib, runpy, sys
root = pathlib.Path(sys.argv.pop(1))
sys.path.insert(0, str(root))
runpy.run_path(str(root / "cve_lab.py"), run_name="__main__")
"#;

/// The Python invocation for the CVE runner, given the directory that holds
/// the bundled `cve_lab*.py` modules.
fn cve_python_command(modules_dir: &std::path::Path) -> std::process::Command {
    let mut command = std::process::Command::new("python3");
    command.args(["-I", "-c", CVE_BOOTSTRAP]).arg(modules_dir);
    command
}

/// Run the shipped, isolated standard-library runner without a checkout or pip.
pub fn cve(args: Vec<std::ffi::OsString>) -> anyhow::Result<()> {
    let files: BTreeMap<String, String> = serde_json::from_str(include_str!("bundled_tools.json"))?;
    let modules: BTreeMap<_, _> = files
        .into_iter()
        .filter(|(name, _)| name.starts_with("scripts/cve_lab") && name.ends_with(".py"))
        .collect();
    anyhow::ensure!(modules.contains_key("scripts/cve_lab.py"), "Missing bundled CVE runner");
    // A private (0700) directory, removed when this function returns.
    let directory = tempfile::Builder::new()
        .prefix("mockforge-cve-")
        .tempdir()
        .context("Could not create a temporary directory for the CVE runner")?;
    for (name, source) in &modules {
        let file_name = std::path::Path::new(name)
            .file_name()
            .with_context(|| format!("Bundled module has no file name: {name}"))?;
        fs::write(directory.path().join(file_name), source)?;
    }
    let status = cve_python_command(directory.path()).args(args).status().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!("Managed CVE labs require Python 3.11+ (python3 on PATH): {error}")
        } else {
            anyhow::anyhow!("Could not start python3 for the CVE runner: {error}")
        }
    })?;
    anyhow::ensure!(status.success(), "CVE command failed ({status}); see diagnostics above");
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

    #[test]
    fn cve_command_line_does_not_grow_with_the_bundle() {
        // The bundled CVE modules once travelled as one JSON argument; past
        // 128 KiB (MAX_ARG_STRLEN) every `mockforge cve` failed with E2BIG.
        let files: BTreeMap<String, String> =
            serde_json::from_str(include_str!("bundled_tools.json")).unwrap();
        let bundle_bytes: usize = files
            .iter()
            .filter(|(name, _)| name.starts_with("scripts/cve_lab") && name.ends_with(".py"))
            .map(|(_, source)| source.len())
            .sum();
        assert!(bundle_bytes > 0);
        let dir = tempfile::tempdir().unwrap();
        let command = cve_python_command(dir.path());
        let longest = command.get_args().map(|arg| arg.len()).max().unwrap();
        assert!(longest < 4096, "a CVE runner argument is {longest} bytes");
        assert_eq!(command.get_args().last().unwrap(), dir.path().as_os_str());
    }

    #[test]
    fn cve_list_runs_through_the_real_python_path() {
        // Needs a Python 3.11+ on PATH, as `mockforge cve` itself does.
        let usable = std::process::Command::new("python3")
            .args(["-c", "import sys; sys.exit(sys.version_info < (3, 11))"])
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if !usable {
            eprintln!("skipping: no python3 >= 3.11 on PATH");
            return;
        }
        cve(vec!["list".into()]).unwrap();
    }
}
