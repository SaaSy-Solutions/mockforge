//! Existing input paths for UI build-script invalidation.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

/// Select existing inputs after the caller has created `ui/dist`.
///
/// Source checkouts watch the three separate UI directories, avoiding
/// `ui/node_modules`. Published packages omit `ui/src`; their existing `ui`
/// parent observes later directory creation without watching missing paths.
pub(crate) fn watched_inputs(crate_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let ui_dir = crate_dir.join("ui");
    if !ui_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "create the UI output directory before selecting build inputs",
        ));
    }
    let mut inputs = BTreeSet::from([crate_dir.join("build.rs")]);
    for relative in ["src", "dist", "public"] {
        let candidate = ui_dir.join(relative);
        inputs.insert(if candidate.exists() {
            candidate
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
        fs::write(fixture.0.join("build_ui.sh"), "#!/bin/sh\n").expect("source hook");
        let paths = watched_inputs(&fixture.0).expect("watch inputs");
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
        let paths = watched_inputs(&fixture.0).expect("watch inputs");
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
        let paths = watched_inputs(&fixture.0).expect("watch inputs");
        assert!(observed(&paths, &fixture.0.join("ui/src/main.tsx")));
        assert!(observed(&paths, &fixture.0.join("ui/public/mockforge-icon.png")));
        assert!(!observed(&paths, &fixture.0.join("target/debug/generated.rs")));
    }

    #[test]
    fn removal_replaces_missing_directory_watch_with_existing_parent() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("ui/public")).expect("public directory");
        let before = watched_inputs(&fixture.0).expect("watch inputs");
        assert!(before.contains(&fixture.0.join("ui/public")));
        fs::remove_dir(fixture.0.join("ui/public")).expect("remove owned empty directory");
        let after = watched_inputs(&fixture.0).expect("watch inputs");
        assert!(!after.contains(&fixture.0.join("ui/public")));
        assert!(after.contains(&fixture.0.join("ui")));
        assert!(after.iter().all(|path| path.exists()));
    }

    #[test]
    fn unchanged_inventory_is_stable() {
        let fixture = Fixture::new();
        assert_eq!(
            watched_inputs(&fixture.0).expect("first"),
            watched_inputs(&fixture.0).expect("second")
        );
    }
}
