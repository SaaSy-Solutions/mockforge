// Build script for mockforge-collab
// Automatically enables SQLx offline mode if .sqlx directory exists
// This allows the crate to compile without a database connection when installed from crates.io

fn main() {
    // Get the crate root directory (where Cargo.toml is located)
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set by cargo");
    let sqlx_dir = std::path::Path::new(&manifest_dir).join(".sqlx");

    // Check if .sqlx directory exists in the crate root
    if sqlx_dir.exists() && sqlx_dir.is_dir() {
        // Enable SQLx offline mode
        println!("cargo:rustc-env=SQLX_OFFLINE=true");
    }
}
