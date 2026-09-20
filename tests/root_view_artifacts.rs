//! Independent module effects and literal selector checks for the 0.16 source boundary.
use weave_language::modules::{SourceModule, content_digest, link};

#[test]
fn root_imported_host_effects_fail_in_original_module_even_when_unused() {
    for effect in [
        "accepted Hidden view \"team\" decision \"d\";",
        "view_template Hidden revision \"1\" from Missing clock fixed {}",
    ] {
        let module = format!("module \"bad\" revision \"1\"; {effect}");
        let entry = format!(
            "import unused module \"bad\" revision \"1\" sha256 {:?};",
            content_digest(&module)
        );
        let modules = [SourceModule {
            id: "bad",
            revision: "1",
            source: &module,
        }];
        let error = match link("entry", &entry, &modules) {
            Err(e) => e,
            Ok(_) => panic!("imported host effect was accepted"),
        };
        assert_eq!(error.code, "E_MODULE_EFFECT");
        assert_ne!(error.source_id, "entry");
        assert_eq!(&module[error.start..error.end], "Hidden");
    }
}

#[test]
fn root_linking_never_rewrites_external_view_graph_or_decision_selectors() {
    let module = "module \"tools\" revision \"1\"; function Keep revision \"1\" (graph input) { return input; }";
    let entry = format!(
        r#"import alias module "tools" revision "1" sha256 "{}";
value Answer 42;
live_handle H graph "alias::Keep" branch "alias::main";
view_template Active revision "1" from H clock fixed {{}}
accepted Result view "alias::Keep" decision "alias::decision";
view_current Current view "alias::Keep" definition "sha256:{}" fixed;
apply Copy from alias::Keep {{ graph input Result; }}"#,
        content_digest(module),
        "0".repeat(64)
    );
    let modules = [SourceModule {
        id: "tools",
        revision: "1",
        source: module,
    }];
    let output = link("entry", &entry, &modules)
        .unwrap()
        .compile_artifacts()
        .unwrap();
    assert_eq!(output.values["Answer"].json(), 42);
    let value = serde_json::to_value(&output).unwrap();
    let template = &value["view_templates"]["Active"];
    assert_eq!(template["expression"]["query"]["graph_id"], "alias::Keep");
    assert_eq!(template["expression"]["query"]["branch_id"], "alias::main");
    let commands = value["program"]["commands"].as_array().unwrap();
    let accepted = commands
        .iter()
        .find(|v| v["value"]["kind"] == "accepted_graph")
        .unwrap();
    assert_eq!(accepted["value"]["selection"]["view_id"], "alias::Keep");
    assert_eq!(
        accepted["value"]["selection"]["decision_id"],
        "alias::decision"
    );
    let current = commands
        .iter()
        .find(|v| v["value"]["kind"] == "current_view")
        .unwrap();
    assert_eq!(current["value"]["selection"]["view_id"], "alias::Keep");
}
