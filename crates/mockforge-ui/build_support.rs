//! Existing input paths for UI build-script invalidation.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

const FRONTEND_INPUTS: [&str; 11] = [
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "index.html",
    "vite.config.ts",
    "postcss.config.js",
    "tailwind.config.js",
    "tsconfig.json",
    "tsconfig.app.json",
    "tsconfig.node.json",
    "tsconfig.test.json",
];

/// The selected asset owner determines whether prebuilt files are inputs.
#[derive(Debug)]
pub(crate) enum AssetSource {
    Owned(PathBuf),
    Fallback(PathBuf),
}

impl AssetSource {
    /// Partial or failed producer output never replaces prebuilt assets.
    pub(crate) fn select(owned: &Path, fallback: &Path, succeeded: bool) -> Self {
        if succeeded && owned.join("index.html").is_file() {
            Self::Owned(owned.to_path_buf())
        } else {
            Self::Fallback(fallback.to_path_buf())
        }
    }

    pub(crate) fn directory(&self) -> &Path {
        match self {
            Self::Owned(path) | Self::Fallback(path) => path,
        }
    }
}

/// Watch source inputs and only the active prebuilt fallback.
///
/// Complete source layouts avoid `ui/node_modules`. Missing source inputs or
/// an active missing fallback conservatively watch their existing UI parent.
pub(crate) fn watched_inputs(crate_dir: &Path, assets: &AssetSource) -> io::Result<Vec<PathBuf>> {
    let ui_dir = crate_dir.join("ui");
    if !ui_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "the packaged UI input directory is missing",
        ));
    }
    let mut inputs = BTreeSet::from([crate_dir.join("build.rs")]);
    for relative in ["src", "public"].into_iter().chain(FRONTEND_INPUTS) {
        let candidate = ui_dir.join(relative);
        inputs.insert(if candidate.exists() {
            candidate
        } else {
            ui_dir.clone()
        });
    }
    if let AssetSource::Fallback(path) = assets {
        inputs.insert(if path.exists() {
            path.clone()
        } else {
            ui_dir.clone()
        });
    }
    // This hook is present in source checkouts and deliberately omitted from
    // the published crate. Its removal updates the next watch inventory.
    let source_build = crate_dir.join("build_ui.sh");
    if source_build.exists() {
        inputs.insert(source_build);
    }
    Ok(inputs.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "mockforge-ui-watch-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).expect("create a new owned test directory");
            fs::write(root.join("build.rs"), "fn main() {}\n").expect("build script");
            fs::create_dir_all(root.join("ui/dist")).expect("UI directory");
            Self(root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove only the owned test fixture");
        }
    }

    fn observed(paths: &[PathBuf], input: &Path) -> bool {
        paths
            .iter()
            .any(|path| path == input || (path.is_dir() && input.starts_with(path)))
    }

    #[test]
    fn source_layout_avoids_node_modules_and_missing_watch_targets() {
        let fixture = Fixture::new();
        for name in ["src", "public", "node_modules"] {
            fs::create_dir(fixture.0.join("ui").join(name)).expect("source directory");
        }
        for name in FRONTEND_INPUTS {
            fs::write(fixture.0.join("ui").join(name), "fixture").expect("frontend input");
        }
        fs::write(fixture.0.join("build_ui.sh"), "#!/bin/sh\n").expect("source hook");
        let paths = watched_inputs(&fixture.0, &AssetSource::Fallback(fixture.0.join("ui/dist")))
            .expect("watch inputs");
        assert!(paths.iter().all(|path| path.exists()));
        assert!(observed(&paths, &fixture.0.join("build.rs")));
        assert!(observed(&paths, &fixture.0.join("build_ui.sh")));
        assert!(!observed(&paths, &fixture.0.join("ui/node_modules/package.json")));
        assert!(!observed(&paths, &fixture.0.join("target/debug/generated.rs")));
        assert!(!paths.contains(&fixture.0.join("ui/build.rs")));
    }

    #[test]
    fn absent_dist_assets_are_observed_without_creating_fake_content() {
        let fixture = Fixture::new();
        let paths = watched_inputs(&fixture.0, &AssetSource::Fallback(fixture.0.join("ui/dist")))
            .expect("watch inputs");
        for relative in ["index.html", "manifest.json", "assets/main.js"] {
            let input = fixture.0.join("ui/dist").join(relative);
            assert!(!input.exists());
            assert!(observed(&paths, &input));
        }
        assert!(paths.iter().all(|path| path.exists()));
    }

    #[test]
    fn published_layout_observes_future_source_and_public_assets() {
        let fixture = Fixture::new();
        let paths = watched_inputs(&fixture.0, &AssetSource::Fallback(fixture.0.join("ui/dist")))
            .expect("watch inputs");
        assert!(observed(&paths, &fixture.0.join("ui/src/main.tsx")));
        assert!(observed(&paths, &fixture.0.join("ui/public/mockforge-icon.png")));
        assert!(!observed(&paths, &fixture.0.join("target/debug/generated.rs")));
    }

    #[test]
    fn removal_replaces_missing_directory_watch_with_existing_parent() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("ui/public")).expect("public directory");
        let before = watched_inputs(&fixture.0, &AssetSource::Fallback(fixture.0.join("ui/dist")))
            .expect("watch inputs");
        assert!(before.contains(&fixture.0.join("ui/public")));
        fs::remove_dir(fixture.0.join("ui/public")).expect("remove owned empty directory");
        let after = watched_inputs(&fixture.0, &AssetSource::Fallback(fixture.0.join("ui/dist")))
            .expect("watch inputs");
        assert!(!after.contains(&fixture.0.join("ui/public")));
        assert!(after.contains(&fixture.0.join("ui")));
        assert!(after.iter().all(|path| path.exists()));
    }

    #[test]
    fn source_configuration_inputs_are_observed() {
        let fixture = Fixture::new();
        for name in ["src", "public", "node_modules"] {
            fs::create_dir(fixture.0.join("ui").join(name)).expect("source directory");
        }
        let inputs = [
            "package.json",
            "pnpm-lock.yaml",
            "pnpm-workspace.yaml",
            "index.html",
            "vite.config.ts",
            "postcss.config.js",
            "tailwind.config.js",
            "tsconfig.json",
            "tsconfig.app.json",
            "tsconfig.node.json",
            "tsconfig.test.json",
        ];
        for name in inputs {
            fs::write(fixture.0.join("ui").join(name), "fixture").expect("frontend input");
        }
        let paths = watched_inputs(&fixture.0, &AssetSource::Fallback(fixture.0.join("ui/dist")))
            .expect("watch inputs");
        for name in inputs {
            assert!(
                observed(&paths, &fixture.0.join("ui").join(name)),
                "frontend input is not observed: {name}"
            );
        }
        assert!(!observed(&paths, &fixture.0.join("ui/node_modules/package.json")));
    }

    #[test]
    fn owned_output_ignores_absent_prebuilt_and_node_modules() {
        let fixture = Fixture::new();
        for name in ["src", "public", "node_modules"] {
            fs::create_dir(fixture.0.join("ui").join(name)).expect("source directory");
        }
        for name in FRONTEND_INPUTS {
            fs::write(fixture.0.join("ui").join(name), "fixture").expect("frontend input");
        }
        fs::remove_dir(fixture.0.join("ui/dist")).expect("remove owned empty fallback");
        let owned = fixture.0.join("target/owned-ui");
        fs::create_dir_all(&owned).expect("owned output");
        fs::write(owned.join("index.html"), "owned output").expect("owned index");
        let assets = AssetSource::select(&owned, &fixture.0.join("ui/dist"), true);
        let paths = watched_inputs(&fixture.0, &assets).expect("owned watch inputs");
        assert!(!observed(&paths, &owned.join("index.html")));
        assert!(!observed(&paths, &fixture.0.join("ui/dist/index.html")));
        assert!(!observed(&paths, &fixture.0.join("ui/node_modules/package.json")));
    }

    #[test]
    fn incomplete_or_failed_producer_uses_fallback() {
        let fixture = Fixture::new();
        let owned = fixture.0.join("target/owned-ui");
        let fallback = fixture.0.join("ui/dist");
        fs::create_dir_all(&owned).expect("owned output");
        assert!(matches!(AssetSource::select(&owned, &fallback, true), AssetSource::Fallback(_)));
        fs::write(owned.join("index.html"), "partial output").expect("owned index");
        assert!(matches!(
            AssetSource::select(&owned, &fallback, false),
            AssetSource::Fallback(_)
        ));
        let selected = AssetSource::select(&owned, &fallback, true);
        assert!(matches!(selected, AssetSource::Owned(_)));
        assert_eq!(selected.directory(), owned);
    }

    #[test]
    fn active_absent_fallback_observes_appearance_conservatively() {
        let fixture = Fixture::new();
        fs::remove_dir(fixture.0.join("ui/dist")).expect("remove owned empty fallback");
        let assets = AssetSource::Fallback(fixture.0.join("ui/dist"));
        let paths = watched_inputs(&fixture.0, &assets).expect("fallback watch inputs");
        assert!(observed(&paths, &fixture.0.join("ui/dist/index.html")));
        assert!(paths.contains(&fixture.0.join("ui")));
        assert!(!observed(&paths, &fixture.0.join("target/generated.rs")));
    }

    #[test]
    fn unchanged_inventory_is_stable() {
        let fixture = Fixture::new();
        assert_eq!(
            watched_inputs(&fixture.0, &AssetSource::Fallback(fixture.0.join("ui/dist")))
                .expect("first"),
            watched_inputs(&fixture.0, &AssetSource::Fallback(fixture.0.join("ui/dist")))
                .expect("second")
        );
    }
}
