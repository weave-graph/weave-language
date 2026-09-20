use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use weave_language::{
    compile_artifacts, format_source,
    modules::{self, SourceModule},
};

#[test]
fn exact_tokens_comments_and_literal_spelling_survive() {
    let source = "// π leading\nvalue A integer_add(1,2); // inline  spaces  \n\tgraph G{node \"a\" entity \"a\" space \"s\" property \"span\" {\"value\":\"// not a comment\\n\\u0061\",\"param\":[1.0e+2,-3,true]};}\n// tail";
    let expected = "// π leading\nvalue A integer_add(1, 2); // inline  spaces  \ngraph G {\n  node \"a\" entity \"a\" space \"s\" property \"span\" {\n    \"value\": \"// not a comment\\n\\u0061\", \"param\": [1.0e+2, -3, true]\n  };\n}\n// tail\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert_eq!(
        compile_artifacts(source).unwrap().fingerprint().unwrap(),
        compile_artifacts(&formatted)
            .unwrap()
            .fingerprint()
            .unwrap()
    );
}
fn sources(dir: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, paths);
        } else if path.extension().is_some_and(|ext| ext == "weave") {
            paths.push(path);
        }
    }
}
#[test]
fn every_example_parses_and_formats_idempotently_with_semantic_roundtrip() {
    let mut paths = vec![];
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("examples"),
        &mut paths,
    );
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/proposals/scalar-functions/fixtures"),
        &mut paths,
    );
    paths.sort();
    assert!(paths.len() >= 29);
    for path in paths {
        let source = fs::read_to_string(&path).unwrap();
        let formatted =
            format_source(&source).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        assert_eq!(
            format_source(&formatted).unwrap(),
            formatted,
            "{}",
            path.display()
        );
        let parsed = weave_language::parse(&formatted).unwrap();
        if !parsed.statements.iter().any(|s| {
            matches!(
                s,
                weave_language::syntax::Statement::Import { .. }
                    | weave_language::syntax::Statement::ModuleHeader { .. }
            )
        }) {
            let before =
                compile_artifacts(&source).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
            let after = compile_artifacts(&formatted).unwrap();
            assert_eq!(
                serde_json::to_value(before).unwrap(),
                serde_json::to_value(after).unwrap(),
                "{}",
                path.display()
            );
        }
    }
}
#[test]
fn formatting_modules_requires_new_content_pins_and_explicit_relinking() {
    let module =
        "module \"core\" revision \"1\"; function Keep revision \"1\"(graph input){return input;}";
    let formatted = format_source(module).unwrap();
    assert_ne!(module, formatted);
    let entry = format!(
        "import c module \"core\" revision \"1\" sha256 {:?}; graph G{{}} apply A from c::Keep{{graph input G;}}",
        modules::content_digest(module)
    );
    let unit = |source| SourceModule {
        id: "core",
        revision: "1",
        source,
    };
    let old = modules::link("entry", &entry, &[unit(module)])
        .unwrap()
        .compile_artifacts()
        .unwrap();
    let error = match modules::link("entry", &entry, &[unit(&formatted)]) {
        Err(error) => error,
        Ok(_) => panic!("old byte pin accepted"),
    };
    assert_eq!(error.code, "E_MODULE_DIGEST");
    let new_entry = format_source(&entry.replace(
        &modules::content_digest(module),
        &modules::content_digest(&formatted),
    ))
    .unwrap();
    let new = modules::link("entry", &new_entry, &[unit(&formatted)])
        .unwrap()
        .compile_artifacts()
        .unwrap();
    assert_eq!(old.program.commands, new.program.commands);
    assert_eq!(old.values, new.values);
    assert_ne!(old.fingerprint().unwrap(), new.fingerprint().unwrap());
}
#[test]
fn syntax_errors_are_original_diagnostics_and_comments_stay_opaque() {
    let invalid = "// keep\ngraph G { node \"unterminated";
    assert_eq!(
        format_source(invalid).unwrap_err(),
        weave_language::parse(invalid).unwrap_err()
    );
    for source in [
        "",
        " \t\n",
        "// Unicode 🧭\r\n// { \"unterminated text\r\n",
        "graph G { // start\n // middle\n} // close",
    ] {
        let value = format_source(source).unwrap();
        assert_eq!(format_source(&value).unwrap(), value);
        if source.contains('\r') {
            assert!(value.contains("text\r\n"));
        }
    }
    assert_eq!(
        format_source(&" ".repeat(1_048_577)).unwrap_err().code,
        "E_BUDGET"
    );
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "weave-formatter-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn cli(path: &Path, option: Option<&str>) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_weave"));
    cmd.arg("fmt").arg(path);
    if let Some(option) = option {
        cmd.arg(option);
    }
    cmd.output().unwrap()
}
#[test]
fn cli_is_stdout_by_default_and_write_is_explicit_and_failure_safe() {
    let temp = Temp::new();
    let path = temp.0.join("input.weave");
    let original = "graph G{}";
    fs::write(&path, original).unwrap();
    let output = cli(&path, None);
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "graph G {}\n");
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert!(!cli(&path, Some("--check")).status.success());
    assert!(cli(&path, Some("--write")).status.success());
    assert!(cli(&path, Some("--check")).status.success());
    fs::write(&path, "graph {").unwrap();
    assert!(!cli(&path, Some("--write")).status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), "graph {");
    assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 1);
}
#[cfg(unix)]
#[test]
fn explicit_write_preserves_permissions_and_rejects_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = Temp::new();
    let path = temp.0.join("source.weave");
    fs::write(&path, "graph G{}").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    assert!(cli(&path, Some("--write")).status.success());
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    let link = temp.0.join("alias.weave");
    symlink(&path, &link).unwrap();
    assert!(!cli(&link, Some("--write")).status.success());
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn actual_module_example_entry_formats_without_changing_linked_artifacts() {
    let entry = include_str!("../examples/modules/main.weave");
    let units = [
        SourceModule {
            id: "example.core",
            revision: "1",
            source: include_str!("../examples/modules/core.weave"),
        },
        SourceModule {
            id: "example.views",
            revision: "1",
            source: include_str!("../examples/modules/views.weave"),
        },
    ];
    let before = modules::link("entry", entry, &units)
        .unwrap()
        .compile_artifacts()
        .unwrap();
    let after = modules::link("entry", &format_source(entry).unwrap(), &units)
        .unwrap()
        .compile_artifacts()
        .unwrap();
    assert_eq!(before.fingerprint().unwrap(), after.fingerprint().unwrap());
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(after).unwrap()
    );
}
#[test]
fn layout_expansion_and_comment_work_are_bounded() {
    let source = format!(
        "graph G{{//{}\n}}",
        "x".repeat(1_048_576 - "graph G{//\n}".len())
    );
    assert_eq!(source.len(), 1_048_576);
    weave_language::parse(&source).unwrap();
    assert_eq!(format_source(&source).unwrap_err().code, "E_BUDGET");
    assert_eq!(
        format_source(&"//\n".repeat(100_001)).unwrap_err().code,
        "E_BUDGET"
    );
    // A long first comment must not be rescanned for every later comment.
    let source = format!(
        "graph G {{//{}\n{}}}",
        "x".repeat(500_000),
        "//y\n".repeat(10_000)
    );
    let formatted = format_source(&source).unwrap();
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}
