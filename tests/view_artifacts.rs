use weave_contract::{Command, GraphExpression, ViewReadTime};
use weave_language::{
    compile, compile_artifacts,
    modules::{SourceModule, content_digest, link},
};

const TEMPLATE: &str = r#"live_handle Head graph "Fleet" branch "main";
view_template Active revision "1" from Head clock tick { match relation "connected"; at 0; }"#;
const HASH: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn all_legacy_execution_outputs_reject_unreturned_templates() {
    let source = format!("value Answer integer_add(20,22); {TEMPLATE}");
    assert_eq!(
        compile(&source).unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
    assert_eq!(
        weave_language::specialize(&source).unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
    assert_eq!(
        weave_language::fingerprint(&source).unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
    assert_eq!(
        weave_language::describe(&source).unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
    let linked = link("entry", &source, &[]).unwrap();
    assert_eq!(
        linked.compile().unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
    assert_eq!(
        linked.specialize().unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
    assert_eq!(
        linked.fingerprint().unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
    assert!(
        linked.schemas().is_empty(),
        "AST inspection remains explicit and compatible"
    );
    let artifacts = linked.compile_artifacts().unwrap();
    assert!(artifacts.program.commands.is_empty());
    assert_eq!(artifacts.values["Answer"].json(), 42);
    assert_eq!(artifacts.view_templates.len(), 1);
    weave_contract::view_registration::validate_template(&artifacts.view_templates["Active"])
        .unwrap();
    assert_ne!(
        artifacts.fingerprint().unwrap(),
        compile_artifacts(TEMPLATE).unwrap().fingerprint().unwrap()
    );
}

#[test]
fn template_identity_is_semantic_but_retains_all_recipe_inputs() {
    let original = compile_artifacts(TEMPLATE).unwrap();
    let spaced = compile_artifacts(&format!(
        "// presentation\n{}",
        TEMPLATE.replace(';', ";\n")
    ))
    .unwrap();
    assert_eq!(
        original.fingerprint().unwrap(),
        spaced.fingerprint().unwrap()
    );
    let template = &original.view_templates["Active"];
    assert!(
        template
            .source_revisions
            .iter()
            .any(|s| s.name.starts_with("weave:view-template:"))
    );
    for changed in [
        TEMPLATE.replace("connected", "other"),
        TEMPLATE.replace("clock tick", "clock fixed"),
        TEMPLATE.replace("revision \"1\"", "revision \"2\""),
        TEMPLATE.replace("Fleet", "Other"),
    ] {
        assert_ne!(
            template.definition_digest,
            compile_artifacts(&changed).unwrap().view_templates["Active"].definition_digest
        );
    }
}

#[test]
fn reads_bind_once_and_preserve_explicit_fact_time() {
    let source = format!(
        r#"value Instant time 5;
accepted Approved view "team" decision "d1";
view_current Cached view "active" definition "{HASH}" tick value Instant;
view_current Fixed view "fixed" definition "{HASH}" fixed;"#
    );
    let p = compile(&source).unwrap();
    assert_eq!(p.commands.len(), 3);
    assert!(
        matches!(&p.commands[0], Command::Bind { value: GraphExpression::AcceptedGraph { selection }, .. } if selection.decision_id == "d1")
    );
    assert!(
        matches!(&p.commands[1], Command::Bind { value: GraphExpression::CurrentView { selection }, .. } if selection.time == ViewReadTime::Tick { valid_at: 5 })
    );
    assert!(
        matches!(&p.commands[2], Command::Bind { value: GraphExpression::CurrentView { selection }, .. } if selection.time == ViewReadTime::Fixed)
    );
    assert_eq!(
        compile(&source.replace("value Instant time 5", "value Instant 5"))
            .unwrap_err()
            .code,
        "E_PARAMETER_TYPE"
    );
}

#[test]
fn unused_functions_cannot_hide_host_reads_or_templates() {
    for effect in [
        "accepted X view \"team\" decision \"d1\";".to_string(),
        format!("view_current X view \"v\" definition \"{HASH}\" fixed;"),
        "view_template X revision \"1\" from input clock fixed {}".into(),
    ] {
        let source =
            format!("function Forbidden revision \"1\" (graph input) {{ {effect} return input; }}");
        assert_eq!(
            compile_artifacts(&source).unwrap_err().code,
            "E_FUNCTION_EFFECT"
        );
    }
}

#[test]
fn templates_cannot_capture_materialized_values_or_duplicate_names() {
    assert_eq!(
        compile_artifacts(
            "use G graph \"Fleet\"; view_template T revision \"1\" from G clock fixed {}"
        )
        .unwrap_err()
        .code,
        "E_VIEW_TEMPLATE"
    );
    assert!(
        compile_artifacts(&format!(
            "{TEMPLATE} view_template Active revision \"2\" from Head clock fixed {{}}"
        ))
        .is_err()
    );
    let mut source = "live_handle Head graph \"Fleet\" branch \"main\";".to_owned();
    for n in 0..17 {
        source += &format!("view_template V{n} revision \"1\" from Head clock fixed {{}}");
    }
    assert_eq!(compile_artifacts(&source).unwrap_err().code, "E_BUDGET");
}

#[test]
fn linked_template_manifests_are_alias_independent_and_retain_dependencies() {
    let module = r#"module "tools" revision "1"; function Keep revision "1" (graph input) { return input; }"#;
    let entry = |alias| {
        format!(
            r#"import {alias} module "tools" revision "1" sha256 "{}";
{TEMPLATE}
accepted Approved view "team" decision "d1";
apply Copy from {alias}::Keep {{ graph input Approved; }}"#,
            content_digest(module)
        )
    };
    let units = [SourceModule {
        id: "tools",
        revision: "1",
        source: module,
    }];
    let a = link("one", &entry("first"), &units)
        .unwrap()
        .compile_artifacts()
        .unwrap();
    let b = link("two", &entry("renamed"), &units)
        .unwrap()
        .compile_artifacts()
        .unwrap();
    assert_eq!(a.fingerprint().unwrap(), b.fingerprint().unwrap());
    let template = &a.view_templates["Active"];
    assert!(
        template
            .source_revisions
            .iter()
            .any(|s| s.name == "module:tools:function:Keep")
    );
    assert!(
        template
            .source_revisions
            .iter()
            .any(|s| s.name == "module:tools")
    );
    weave_contract::view_registration::validate_template(template).unwrap();
}

#[test]
fn service_schema_is_unknown_not_silently_untyped() {
    let source = r#"schema Fleet revision "1" { node Device space "s" {} }
function Keep revision "1" (graph input schema Fleet) returns graph schema Fleet { return input; }
accepted Approved view "team" decision "d1";
apply Copy from Keep { graph input Approved; }"#;
    assert_eq!(compile(source).unwrap_err().code, "E_SCHEMA_UNKNOWN");
}

#[test]
fn artifact_api_preserves_existing_program_outputs_and_typed_template_literals() {
    let source = "value N 42; graph G {} lens R from G {}";
    assert_eq!(
        compile_artifacts(source).unwrap().program,
        compile(source).unwrap()
    );
    let linked = link("entry", source, &[]).unwrap();
    assert_eq!(
        linked.compile_artifacts().unwrap().program,
        linked.compile().unwrap()
    );
    let source = r#"value Relation "span/value/param"; value Instant time 5;
live_handle Head graph "Fleet" branch "main";
view_template Active revision "1" from Head clock fixed {match relation value Relation; at value Instant;}"#;
    let a = compile_artifacts(source).unwrap();
    let GraphExpression::Query { query } = &a.view_templates["Active"].expression else {
        panic!()
    };
    assert_eq!(query.predicate.as_deref(), Some("span/value/param"));
    assert_eq!(query.valid_at, Some(5));
    assert_eq!(
        compile_artifacts(&source.replace("value Instant time 5", "value Instant 5"))
            .unwrap_err()
            .code,
        "E_PARAMETER_TYPE"
    );
}

#[test]
fn cli_artifact_selection_is_explicit_and_legacy_describe_keeps_its_shape() {
    let path =
        std::env::temp_dir().join(format!("weave-view-artifacts-{}.weave", std::process::id()));
    std::fs::write(&path, TEMPLATE).unwrap();
    let compiler = env!("CARGO_BIN_EXE_weave");
    let out = std::process::Command::new(compiler)
        .args(["view-plan", path.to_str().unwrap(), "--template", "Active"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let artifact: weave_contract::CompiledViewTemplate =
        serde_json::from_slice(&out.stdout).unwrap();
    weave_contract::view_registration::validate_template(&artifact).unwrap();
    for cmd in ["plan", "values", "fingerprint", "describe"] {
        let out = std::process::Command::new(compiler)
            .args([cmd, path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(!out.status.success() && out.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&out.stderr).unwrap()["code"],
            "E_HOST_ARTIFACT_REQUIRED"
        );
    }
    std::fs::write(&path, "").unwrap();
    let out = std::process::Command::new(compiler)
        .args(["describe", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap(),
        serde_json::json!([])
    );
    std::fs::remove_file(path).unwrap();
}
