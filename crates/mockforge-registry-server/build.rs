fn main() {
    // sqlx::migrate! embeds files known at compilation. Also watch the directory
    // so adding a migration refreshes the embedded inventory in local builds.
    println!("cargo:rerun-if-changed=migrations");
}
