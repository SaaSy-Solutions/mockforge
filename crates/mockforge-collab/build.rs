// Build script for mockforge-collab
// Automatically enables SQLx offline mode if .sqlx directory exists
// This allows the crate to compile without a database connection when installed from crates.io

fn main() {
    // Get the crate root directory (where Cargo.toml is located)
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set by cargo");
    let sqlx_dir = std::path::Path::new(&manifest_dir).join(".sqlx");

    // Check if .sqlx directory exists and has query cache files
    // Only enable offline mode if we have actual query cache files
    if sqlx_dir.exists() && sqlx_dir.is_dir() {
        // Count query cache files to ensure we have some
        let query_files = std::fs::read_dir(&sqlx_dir)
            .ok()
            .map(|entries| entries.filter_map(|e| e.ok()).count())
            .unwrap_or(0);

        // Enable SQLx offline mode if we have query cache files
        // Note: This assumes all queries are cached. If some queries are missing,
        // users may need to set SQLX_OFFLINE=false and provide DATABASE_URL
        if query_files > 0 {
            println!("cargo:rustc-env=SQLX_OFFLINE=true");
        }
    }
}
