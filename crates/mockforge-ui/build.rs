use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize, Debug)]
struct ManifestEntry {
    file: String,
    css: Option<Vec<String>>,
}

// The published mockforge-ui crate deliberately omits the compiled Vite
// bundle (see `include = [...]` in Cargo.toml) because it would push the
// tarball well past crates.io's 10 MiB cap. When `ui/dist/index.html`
// isn't present at compile time, this placeholder is embedded instead.
// The mock server itself is fully functional — only the React dashboard
// is a stub. Users who want the full dashboard should run the Docker
// image or build from source.
const PLACEHOLDER_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>MockForge — Admin UI not bundled</title>
<style>
  body{font-family:system-ui,-apple-system,sans-serif;max-width:640px;margin:60px auto;padding:0 24px;color:#1a1a1a;line-height:1.55}
  h1{margin-bottom:4px}
  p.muted{color:#666;margin-top:0}
  pre{background:#f4f4f4;padding:12px 14px;border-radius:6px;overflow-x:auto;font-size:13px}
  code{background:#f4f4f4;padding:2px 5px;border-radius:3px;font-size:90%}
  a{color:#2962ff}
  ul{padding-left:20px}
</style>
</head>
<body>
<h1>MockForge Admin UI</h1>
<p class="muted">The mock server is running. The web dashboard is not bundled in this build.</p>

<p>This <code>mockforge-cli</code> was installed from crates.io, where the full React admin UI can't be shipped (it would exceed the 10 MiB package cap). The mock server, its OpenAPI routes, dynamic stubs, and management API at <code>/__mockforge/api/*</code> all work normally — you just don't get the dashboard UI on this port.</p>

<h2>To get the dashboard</h2>
<ul>
  <li><strong>Docker</strong> (recommended): <code>docker run -p 3000:3000 -p 9080:9080 ghcr.io/saasy-solutions/mockforge:latest</code></li>
  <li><strong>From source</strong>: <code>git clone https://github.com/SaaSy-Solutions/mockforge &amp;&amp; cd mockforge/crates/mockforge-ui/ui &amp;&amp; pnpm install &amp;&amp; pnpm build &amp;&amp; cargo install --path ../../mockforge-cli --locked</code></li>
</ul>

<p>The management API is available at this host — you can POST mocks to <code>/__mockforge/api/mocks</code> directly or via the <a href="https://www.npmjs.com/package/@mockforge-dev/sdk">@mockforge-dev/sdk</a> Node.js SDK.</p>
</body>
</html>"#;

fn main() {
    // Generate version information using vergen
    if let Err(e) = vergen::EmitBuilder::builder().build_timestamp().git_sha(true).emit() {
        println!("cargo:warning=Failed to generate version info: {}", e);
    }
    println!("cargo:rerun-if-changed=ui/build.rs");
    println!("cargo:rerun-if-changed=ui/src/");

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ui_dist_path = Path::new(&crate_dir).join("ui/dist");
    let ui_public_path = Path::new(&crate_dir).join("ui/public");

    // Ensure dist directory exists (for local dev builds)
    if !ui_dist_path.exists() {
        fs::create_dir_all(&ui_dist_path).expect("Failed to create ui/dist directory");
    }

    // Try to run the UI build script, but don't fail if it doesn't exist or fails
    // This allows the crate to compile even when the UI hasn't been built
    let ui_build_script = Path::new(&crate_dir).join("build_ui.sh");
    if ui_build_script.exists() {
        let status = Command::new("bash").arg(&ui_build_script).status();

        if let Ok(status) = status {
            if !status.success() {
                println!(
                    "cargo:warning=UI build script failed, but continuing with fallback files"
                );
            }
        } else {
            println!(
                "cargo:warning=Failed to run UI build script, but continuing with fallback files"
            );
        }
    } else {
        println!("cargo:warning=UI build script not found, using fallback files from public/");
    }

    println!("cargo:rerun-if-changed={}", ui_dist_path.join("index.html").display());
    println!("cargo:rerun-if-changed={}", ui_dist_path.join("manifest.json").display());
    println!("cargo:rerun-if-changed={}", ui_dist_path.join("assets").display());
    println!("cargo:rerun-if-changed={}", ui_public_path.join("manifest.json").display());
    println!("cargo:rerun-if-changed={}", ui_public_path.join("sw.js").display());

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    generate_ui_content(out_path, &ui_dist_path);
    generate_asset_paths(out_path, &ui_dist_path);
    generate_icon_assets(out_path, &ui_public_path);
}

/// Resolve CSS and JS paths by checking the Vite manifest first, then falling back to
/// well-known paths under `ui/dist/assets/`. Each file is checked individually.
fn resolve_css_js_paths(ui_dist_path: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    let mut css_path = None;
    let mut js_path = None;

    // Try Vite build manifest first (has hashed filenames)
    let vite_manifest = ui_dist_path.join("manifest.json");
    if vite_manifest.exists() {
        if let Ok(content) = fs::read_to_string(&vite_manifest) {
            if let Ok(manifest) = serde_json::from_str::<HashMap<String, ManifestEntry>>(&content) {
                if let Some(entry) = manifest.get("index.html") {
                    let jp = ui_dist_path.join(&entry.file);
                    if jp.exists() {
                        js_path = Some(jp);
                    }
                    if let Some(css_files) = &entry.css {
                        if let Some(first) = css_files.first() {
                            let cp = ui_dist_path.join(first);
                            if cp.exists() {
                                css_path = Some(cp);
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback to well-known direct paths
    if css_path.is_none() {
        let fallback = ui_dist_path.join("assets/index.css");
        if fallback.exists() {
            css_path = Some(fallback);
        }
    }
    if js_path.is_none() {
        let fallback = ui_dist_path.join("assets/index.js");
        if fallback.exists() {
            js_path = Some(fallback);
        }
    }

    (css_path, js_path)
}

/// Generate `ui_content.rs` with `get_admin_html()`, `get_admin_css()`, `get_admin_js()`.
/// Each function embeds real assets if available, or placeholder content if not.
fn generate_ui_content(out_path: &Path, ui_dist_path: &Path) {
    let dest = out_path.join("ui_content.rs");
    let mut code = String::new();

    // get_admin_html()
    let index_html = ui_dist_path.join("index.html");
    if index_html.exists() {
        code.push_str(&format!(
            "pub fn get_admin_html() -> &'static str {{ include_str!(r\"{}\") }}\n\n",
            index_html.display()
        ));
    } else {
        // Write placeholder HTML to a file so we can include_str! it
        let placeholder_path = out_path.join("placeholder_admin.html");
        fs::write(&placeholder_path, PLACEHOLDER_HTML).unwrap();
        code.push_str(&format!(
            "pub fn get_admin_html() -> &'static str {{ include_str!(r\"{}\") }}\n\n",
            placeholder_path.display()
        ));
    }

    // get_admin_css() and get_admin_js()
    let (css_path, js_path) = resolve_css_js_paths(ui_dist_path);

    if let Some(p) = css_path {
        code.push_str(&format!(
            "pub fn get_admin_css() -> &'static str {{ include_str!(r\"{}\") }}\n\n",
            p.display()
        ));
    } else {
        code.push_str("pub fn get_admin_css() -> &'static str { \"/* UI not built */\" }\n\n");
    }

    if let Some(p) = js_path {
        code.push_str(&format!(
            "pub fn get_admin_js() -> &'static str {{ include_str!(r\"{}\") }}\n\n",
            p.display()
        ));
    } else {
        code.push_str("pub fn get_admin_js() -> &'static str { \"// UI not built\" }\n\n");
    }

    fs::write(&dest, code).unwrap();
}

/// Generate `asset_paths.rs` with only `get_asset_map()` for serving hashed vendor assets.
fn generate_asset_paths(out_path: &Path, ui_dist_path: &Path) {
    let dest = out_path.join("asset_paths.rs");
    let assets_dir = ui_dist_path.join("assets");

    // Generated file is `include!`d into a module, so inner attributes don't
    // apply — use an outer attribute on the fn. Silences lints that depend on
    // whether the map is populated (unused_mut when empty, let_and_return with
    // no inserts).
    let mut code = String::from(
        "#[allow(unused_mut, clippy::let_and_return)]\n\
         pub fn get_asset_map() -> std::collections::HashMap<&'static str, &'static str> {\n",
    );
    code.push_str("    let mut map = std::collections::HashMap::new();\n");

    if assets_dir.exists() {
        if let Ok(entries) = fs::read_dir(&assets_dir) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.path().file_name().and_then(|n| n.to_str()) {
                    if filename.ends_with(".js") || filename.ends_with(".css") {
                        let asset_path = entry.path();
                        code.push_str(&format!(
                            "    map.insert(\"{}\", include_str!(r\"{}\"));\n",
                            filename,
                            asset_path.display()
                        ));
                    }
                }
            }
        }
    }

    code.push_str("    map\n}\n");
    fs::write(&dest, code).unwrap();
}

/// Generate `icon_assets.rs` with embedded icon/logo assets.
fn generate_icon_assets(out_path: &Path, ui_public_path: &Path) {
    let icon_assets_path = out_path.join("icon_assets.rs");
    let mut icon_assets = String::from("// Embedded icon/logo assets\n");

    let icon_files = vec![
        ("ICON_DEFAULT", "mockforge-icon.png"),
        ("ICON_32", "mockforge-icon-32.png"),
        ("ICON_48", "mockforge-icon-48.png"),
        ("LOGO_40", "mockforge-logo-40.png"),
        ("LOGO_80", "mockforge-logo-80.png"),
    ];

    for (const_name, filename) in icon_files {
        let icon_path = ui_public_path.join(filename);
        if icon_path.exists() {
            match fs::read(&icon_path) {
                Ok(bytes) => {
                    let mut byte_array = String::from("&[");
                    for (i, byte) in bytes.iter().enumerate() {
                        if i > 0 {
                            byte_array.push_str(", ");
                        }
                        if i % 20 == 0 && i > 0 {
                            byte_array.push_str("\n        ");
                        }
                        byte_array.push_str(&format!("0x{:02X}", byte));
                    }
                    byte_array.push(']');
                    icon_assets
                        .push_str(&format!("pub const {}: &[u8] = {};\n", const_name, byte_array));
                }
                Err(_) => {
                    icon_assets.push_str(&format!("pub const {}: &[u8] = &[];\n", const_name));
                }
            }
        } else {
            icon_assets.push_str(&format!("pub const {}: &[u8] = &[];\n", const_name));
        }
    }

    fs::write(&icon_assets_path, icon_assets).unwrap();
}
