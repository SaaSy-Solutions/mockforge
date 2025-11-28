// Build script for mockforge-collab
// Automatically enables SQLx offline mode if .sqlx directory exists
// This allows the crate to compile without a database connection when installed from crates.io

fn main() {
    // Check if SQLX_OFFLINE is already set by the user - don't override their choice
    if std::env::var("SQLX_OFFLINE").is_ok() {
        // User has explicitly set SQLX_OFFLINE, respect their choice
        return;
    }

    // Get the crate root directory (where Cargo.toml is located)
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set by cargo");
    let sqlx_dir = std::path::Path::new(&manifest_dir).join(".sqlx");

    // Check if .sqlx directory exists and has query cache files
    // Only enable offline mode if we have actual query cache files
    if sqlx_dir.exists() && sqlx_dir.is_dir() {
        // Count query cache files to ensure we have some
        let query_files = std::fs::read_dir(&sqlx_dir).ok().map_or(0, |entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext == "json")
                })
                .count()
        });

        // Enable SQLx offline mode if we have query cache files
        if query_files > 0 {
            println!("cargo:rustc-env=SQLX_OFFLINE=true");
            println!(
                "cargo:warning=SQLx offline mode enabled (found {query_files} cached queries). If you see compilation errors about missing queries, run: cargo sqlx prepare --database-url <your-database-url>"
            );
        } else {
            eprintln!(
                "cargo:warning=.sqlx directory exists but contains no query cache files. Run 'cargo sqlx prepare' to generate them, or set SQLX_OFFLINE=false to use a database connection."
            );
        }
    } else {
        // .sqlx directory doesn't exist - this is expected for published crates from crates.io
        // The published crate should include .sqlx files, but if they're missing, fall back to database connection
        // Don't set SQLX_OFFLINE=true without cached queries, as that would cause compilation to fail
        eprintln!(
            "cargo:warning=No .sqlx directory found. SQLx will require a database connection during compilation. \
             If you're using the published crate from crates.io, the .sqlx directory should be included. \
             If you see this warning, either: \
             1) Set SQLX_OFFLINE=false to use a database connection, or \
             2) Run 'cargo sqlx prepare --database-url <your-database-url>' to generate query cache"
        );
    }
}
