use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static ID: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "weave-module-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_weave"))
            .args(["plan"])
            .arg(self.0.join("main.weave"))
            .arg("--modules")
            .arg(self.0.join("map.json"))
            .output()
            .unwrap()
    }
    fn error(&self) -> String {
        let out = self.run();
        assert!(!out.status.success());
        serde_json::from_slice::<serde_json::Value>(&out.stderr).unwrap()["code"]
            .as_str()
            .unwrap()
            .into()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn explicit_map_compiles_and_rejects_changed_pins_and_unsafe_paths() {
    let f = Fixture::new();
    let module = "module \"core\" revision \"1\"; function Keep revision \"1\" (graph input) {return input;}";
    let source = format!(
        "import c module \"core\" revision \"1\" sha256 {:?}; graph G {{}} apply V from c::Keep {{graph input G;}}",
        weave_language::modules::content_digest(module)
    );
    fs::write(f.0.join("core.weave"), module).unwrap();
    fs::write(f.0.join("main.weave"), source).unwrap();
    let valid = r#"[{"id":"core","revision":"1","path":"core.weave"}]"#;
    fs::write(f.0.join("map.json"), valid).unwrap();
    let out = f.run();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(plan["commands"].as_array().unwrap().len(), 3);
    fs::write(f.0.join("core.weave"), format!("{module}\n// changed")).unwrap();
    assert_eq!(f.error(), "E_MODULE_DIGEST");
    for invalid in [
        r#"[{"id":"core","id":"other","revision":"1","path":"core.weave"}]"#,
        r#"[{"id":"core","revision":"1","path":"../core.weave"}]"#,
        r#"[{"id":"core","revision":"1","path":"/core.weave"}]"#,
        r#"[{"id":"core","revision":"1","path":"core.weave","unknown":true}]"#,
        r#"[{"id":"core","revision":"1","path":"core.weave"},{"id":"core","revision":"2","path":"core.weave"}]"#,
    ] {
        fs::write(f.0.join("map.json"), invalid).unwrap();
        assert_eq!(f.error(), "E_MODULE_IO");
    }
    fs::write(f.0.join("map.json"), valid).unwrap();
    fs::write(f.0.join("core.weave"), "x".repeat(1_048_577)).unwrap();
    assert_eq!(f.error(), "E_MODULE_IO");
}
#[cfg(unix)]
#[test]
fn symlink_target_must_remain_within_map_directory() {
    let f = Fixture::new();
    let external = Fixture::new();
    fs::write(
        external.0.join("outside.weave"),
        "module \"core\" revision \"1\";",
    )
    .unwrap();
    std::os::unix::fs::symlink(external.0.join("outside.weave"), f.0.join("core.weave")).unwrap();
    fs::write(f.0.join("main.weave"), "").unwrap();
    fs::write(
        f.0.join("map.json"),
        r#"[{"id":"core","revision":"1","path":"core.weave"}]"#,
    )
    .unwrap();
    assert_eq!(f.error(), "E_MODULE_IO");
}
