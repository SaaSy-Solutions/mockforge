use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Deserialize, Debug)]
struct ManifestEntry {
    file: String,
    css: Option<Vec<String>>,
}

fn main() {
    // Generate version information using vergen
    if let Err(e) = vergen::EmitBuilder::builder().build_timestamp().git_sha(true).emit() {
        println!("cargo:warning=Failed to generate version info: {}", e);
    }
    println!("cargo:rerun-if-changed=ui/build.rs");
    println!("cargo:rerun-if-changed=ui/src/");

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ui_build_script = Path::new(&crate_dir).join("build_ui.sh");

    // Run the UI build script
    let status = Command::new("bash")
        .arg(ui_build_script)
        .status()
        .expect("Failed to run UI build script");

    if !status.success() {
        panic!("UI build script failed");
    }

    let ui_dist_path = Path::new(&crate_dir).join("ui/dist");

    println!("cargo:rerun-if-changed={}", ui_dist_path.join("manifest.json").display());

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("asset_paths.rs");

    let manifest_path = ui_dist_path.join("manifest.json");

    if manifest_path.exists() {
        let manifest_content = fs::read_to_string(&manifest_path).unwrap();
        let manifest: HashMap<String, ManifestEntry> =
            serde_json::from_str(&manifest_content).unwrap();
        let entry = manifest
            .get("index.html")
            .expect("Could not find index.html entry in manifest.json");

        let js_path = ui_dist_path.join(&entry.file);
        let css_path = entry
            .css
            .as_ref()
            .and_then(|files| files.first())
            .map(|file| ui_dist_path.join(file));

        let css_content = if let Some(path) = css_path {
            format!(
                "pub fn get_admin_css() -> &'static str {{    include_str!(r\"{}\")\n}}",
                path.display()
            )
        } else {
            "pub fn get_admin_css() -> &'static str { \"\" }\n".to_string()
        };

        let js_content = format!(
            "pub fn get_admin_js() -> &'static str {{    include_str!(r\"{}\")\n}}",
            js_path.display()
        );

        fs::write(&dest_path, format!("{}\n\n{}", css_content, js_content)).unwrap();
    } else {
        // UI not built, create dummy functions
        let content = "
            pub fn get_admin_css() -> &'static str { \"\" }
            pub fn get_admin_js() -> &'static str { \"\" }
        ";
        fs::write(&dest_path, content).unwrap();
    }
}
