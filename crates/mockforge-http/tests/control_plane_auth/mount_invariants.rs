//! Structural coverage of every data-port router mount.
use super::assembly::{is_public_mount, manifest_dir, mount_calls, production_source, read};

#[test]
fn every_control_mount_is_protected() {
    let lib = production_source(&read(&manifest_dir().join("src/lib.rs")));
    let serve = production_source(&read(&manifest_dir().join("../mockforge-cli/src/serve.rs")));
    let mut unprotected = Vec::new();
    let mut protected = 0;
    for (file, source) in [("lib.rs", &lib), ("serve.rs", &serve)] {
        for (call, args) in mount_calls(source) {
            // Routers built inside a control router (e.g. `Router::new()
            // .route(...)` chains passed to protect) are covered by it.
            if args.contains("management_auth::protect(") {
                protected += 1;
                continue;
            }
            if is_public_mount(&args) {
                continue;
            }
            // `.route("/__mockforge/...")` inside a router that is itself
            // passed to protect: the router variable is checked at its mount.
            if call == ".route("
                && (args.starts_with("\"/__mockforge/routes\"")
                    || args.starts_with("\"/__mockforge/coverage\""))
            {
                continue;
            }
            unprotected.push(format!("{file}: {call}{}", args.replace('\n', " ")));
        }
    }
    assert!(
        unprotected.is_empty(),
        "control routers mounted on the data port without management_auth::protect \
         (wrap them, or add a reviewed entry to PUBLIC_MOUNTS):\n{}",
        unprotected.join("\n")
    );
    assert!(protected >= 40, "only {protected} protected mounts found; is the scan broken?");
}
