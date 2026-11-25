fn main() {
    println!("cargo:rerun-if-changed=../package.json");
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("../package.json").expect("read central version source"),
    )
    .expect("parse package.json");
    assert_eq!(
        manifest["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION")),
        "Cargo version differs from package.json; run npm run version:sync"
    );
    tauri_build::build()
}
