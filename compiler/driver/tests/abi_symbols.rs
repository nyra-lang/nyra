//! Legacy entry: manifest-based ABI symbol presence (see abi_manifest.rs).

#[test]
fn nyra_rt_modules_declare_manifest_symbols() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("docs/abi-manifest.toml");
    let text = std::fs::read_to_string(&path).expect("abi-manifest.toml");
    let manifest: toml::Value = toml::from_str(&text).expect("parse manifest");
    let symbols = manifest
        .get("symbol")
        .and_then(|v| v.as_array())
        .expect("symbol array");

    for entry in symbols {
        let name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .expect("symbol name");
        let module = entry
            .get("module")
            .and_then(|v| v.as_str())
            .expect("symbol module");
        let src = if module.starts_with("rt-tls/") || module.starts_with("rt-tls-native/") {
            root.join(module)
        } else {
            root.join("stdlib/rt").join(module)
        };
        let body = std::fs::read_to_string(&src)
            .unwrap_or_else(|e| panic!("read {}: {e}", src.display()));
        assert!(
            body.contains(name),
            "missing {name} in {}",
            src.display()
        );
    }
}
