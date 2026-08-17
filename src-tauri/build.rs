fn main() {
    tauri_build::build();

    // Build the shared cipher for wasm32 and copy aesgcm.wasm into OUT_DIR.
    // The phone client (Phase 4) consumes this from the HTTP server via
    // include_bytes!(concat!(env!("OUT_DIR"), "/aesgcm.wasm")).
    //
    // The nested cargo build uses a separate target dir (target-wasm) so it
    // never contends with the parent build's target lock. Requires the
    // wasm32-unknown-unknown target: rustup target add wasm32-unknown-unknown
    let wasm_out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("aesgcm.wasm");
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let wasm_target_dir = manifest_dir.join("..").join("target-wasm");
    let status = std::process::Command::new("cargo")
        .args([
            "build",
            "-p",
            "airlynk-crypto",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--target-dir",
            wasm_target_dir.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run nested cargo for wasm32; is cargo on PATH?");
    assert!(
        status.success(),
        "wasm32 build of airlynk-crypto failed (rustup target add wasm32-unknown-unknown)"
    );
    std::fs::copy(
        wasm_target_dir.join("wasm32-unknown-unknown/release/airlynk_crypto.wasm"),
        &wasm_out,
    )
    .expect("copy aesgcm.wasm into OUT_DIR");
    println!("cargo:rerun-if-changed=../crates/airlynk-crypto/src/lib.rs");
    println!("cargo:rerun-if-changed=../crates/airlynk-crypto/Cargo.toml");
    println!("cargo:rerun-if-changed=../Cargo.toml");
}
