fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    tonic_prost_build::configure()
        .out_dir(std::env::var("OUT_DIR").unwrap())
        .compile_protos(&[format!("{}/proto/gretter.proto", manifest_dir)], &[format!("{}/proto", manifest_dir)])
        .unwrap();

    println!("cargo:rerun-if-changed=proto/gretter.proto");
}
