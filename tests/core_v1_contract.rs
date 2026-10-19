//! LA0 (weave-language#1): emitted plans validate against engine core contract
//! v1, and `describe --contract` reports extension and deferred features
//! (engine `docs/contract/core-v1.md` sections 12 and 13.4).
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use weave_contract::{
    CompiledHandlerTemplate, CompiledViewTemplate, VERSION, decode, decode_program,
};
use weave_language::contract_profile::FeatureClass;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn weave(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(args)
        .current_dir(root())
        .output()
        .unwrap()
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "weave") {
            out.push(path);
        }
    }
}

/// The module map in an example directory, if any.
fn module_map(dir: &Path) -> Option<PathBuf> {
    ["map.json", "module-map.json"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// Files a module map names are module units; they compile through their entry.
fn module_units(map: &Path) -> BTreeSet<PathBuf> {
    let entries: Vec<Value> = serde_json::from_slice(&std::fs::read(map).unwrap()).unwrap();
    entries
        .iter()
        .map(|entry| map.parent().unwrap().join(entry["path"].as_str().unwrap()))
        .collect()
}

/// Decode with the contract's strict wire decoder and require a canonical round trip.
fn assert_canonical<T: serde::Serialize>(value: &Value, decoded: T, what: &str) {
    assert_eq!(
        &serde_json::to_value(decoded).unwrap(),
        value,
        "{what}: decoding and re-encoding changed the emitted JSON"
    );
}

#[test]
fn every_plan_emitted_for_examples_decodes_as_core_v1() {
    let mut files = vec![];
    sources(&root().join("examples"), &mut files);
    files.sort();
    let units: BTreeSet<PathBuf> = files
        .iter()
        .filter_map(|file| module_map(file.parent().unwrap()))
        .collect::<BTreeSet<_>>()
        .iter()
        .flat_map(|map| module_units(map))
        .collect();
    let mut checked = 0;
    let mut templates = 0;
    for file in files.iter().filter(|file| !units.contains(*file)) {
        let name = file.strip_prefix(root()).unwrap().display().to_string();
        let path = file.to_str().unwrap();
        let map = module_map(file.parent().unwrap());
        let mut args = vec!["artifacts", path];
        if let Some(map) = &map {
            args.extend(["--modules", map.to_str().unwrap()]);
        }
        let out = weave(&args);
        assert!(
            out.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let output: Value = serde_json::from_slice(&out.stdout).unwrap();
        let artifacts = &output["artifacts"];
        let program = &artifacts["program"];
        let bytes = serde_json::to_vec(program).unwrap();
        let decoded = decode_program(&bytes)
            .unwrap_or_else(|e| panic!("{name}: emitted plan is not core v1: {e}"));
        assert_eq!(decoded.version, VERSION, "{name}");
        assert_canonical(program, decoded, &name);
        for (key, template) in artifacts["view_templates"].as_object().unwrap() {
            let bytes = serde_json::to_vec(template).unwrap();
            let decoded: CompiledViewTemplate =
                decode(&bytes).unwrap_or_else(|e| panic!("{name}: view template {key}: {e}"));
            weave_contract::view_registration::validate_template(&decoded).unwrap();
            assert_eq!(decoded.protocol, VERSION, "{name}");
            assert_canonical(template, decoded, &name);
            templates += 1;
        }
        if let Some(handlers) = artifacts.get("handler_templates") {
            for (key, template) in handlers.as_object().unwrap() {
                let bytes = serde_json::to_vec(template).unwrap();
                let decoded: CompiledHandlerTemplate = decode(&bytes)
                    .unwrap_or_else(|e| panic!("{name}: handler template {key}: {e}"));
                assert_eq!(decoded.protocol, VERSION, "{name}");
                assert_canonical(template, decoded, &name);
                templates += 1;
            }
        }
        checked += 1;
    }
    // Guard against silently skipping examples: every entry source is checked.
    assert!(checked >= 30, "only {checked} examples checked");
    assert!(templates >= 3, "only {templates} templates checked");
}

#[test]
fn emitted_plans_are_rejected_by_v1_when_they_drift() {
    // The golden check above is meaningful only if the decoder is strict.
    let plan = serde_json::to_value(
        weave_language::compile("graph G { node \"a\" entity \"e\" space \"s\"; }").unwrap(),
    )
    .unwrap();
    let mut extra = plan.clone();
    // `provenance` is active from 0.22.0; `reader_groups` stays reserved (B1).
    extra["commands"][0]["data"]["nodes"][0]["reader_groups"] = serde_json::json!([]);
    let mut newer = plan.clone();
    newer["version"] = "0.99.0".into();
    for (value, code) in [(extra, "E_UNKNOWN_FIELD"), (newer, "E_VERSION")] {
        let error = decode_program(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert_eq!(error.code, code);
    }
}

fn non_core(source: &str) -> BTreeSet<(String, FeatureClass)> {
    let description = weave_language::describe_contract(source).unwrap();
    assert_eq!(description.contract.profile, "core-v1");
    assert_eq!(description.contract.protocol, VERSION);
    let set: BTreeSet<_> = description
        .contract
        .features
        .iter()
        .filter(|f| f.class != FeatureClass::CoreV1)
        .map(|f| (f.feature.to_string(), f.class))
        .collect();
    assert_eq!(description.contract.core_only, set.is_empty(), "{source}");
    set
}

fn example(name: &str) -> String {
    std::fs::read_to_string(root().join("examples").join(name)).unwrap()
}

#[test]
fn describe_contract_reports_extension_and_deferred_features_per_section_12() {
    use FeatureClass::{Deferred, Extension};
    let expect = |items: &[(&str, FeatureClass)]| -> BTreeSet<(String, FeatureClass)> {
        items.iter().map(|(f, c)| (f.to_string(), *c)).collect()
    };
    let cases: &[(&str, &[(&str, FeatureClass)])] = &[
        ("fleet.weave", &[]),
        ("algebra.weave", &[]),
        ("temporal.weave", &[]),
        ("rules.weave", &[("reason", Extension)]),
        (
            "contexts.weave",
            &[
                ("attachment_context", Extension),
                ("context", Extension),
                ("reason", Extension),
            ],
        ),
        ("geometry.weave", &[("geometry", Extension)]),
        ("vectors.weave", &[("geometry", Extension)]),
        ("counterparts.weave", &[("counterparts", Extension)]),
        (
            "quantities.weave",
            &[("decimal", Extension), ("quantity", Extension)],
        ),
        (
            "accepted_identity.weave",
            &[("resolve_identity", Extension)],
        ),
        ("typed_contexts.weave", &[("typed_context", Extension)]),
        ("cluster_navigation.weave", &[("cluster", Deferred)]),
        (
            "handlers.weave",
            &[
                ("compiled_handler_template", Deferred),
                ("reason", Extension),
            ],
        ),
        (
            "view_services/template.weave",
            &[("compiled_view_template", Extension)],
        ),
    ];
    for (name, expected) in cases {
        assert_eq!(non_core(&example(name)), expect(expected), "{name}");
    }
    let digest = format!("sha256:{}", "0".repeat(64));
    let inline: &[(String, &[(&str, FeatureClass)])] = &[
        (
            r#"recorded_handle H graph "Facts" branch "main" known_at 20;
pin Historical from H at 7;
recorded_range R graph "Facts" branch "main" observer "local" between 5 and 20 limit 10;"#
                .into(),
            &[],
        ),
        (
            r#"accepted_history Old view "team" accepted_at 25;"#.into(),
            &[("accepted_history", Extension)],
        ),
        (
            r#"accepted_range A view "team" observer "local" between 10 and 50 limit 20;"#.into(),
            &[("accepted_range", Extension)],
        ),
        (
            r#"accepted Approved view "team" decision "d";"#.into(),
            &[("accepted_graph", Extension)],
        ),
        (
            format!(r#"view_current Current view "team" definition "{digest}" fixed;"#),
            &[("current_view", Extension)],
        ),
    ];
    for (source, expected) in inline {
        assert_eq!(non_core(source), expect(expected), "{source}");
    }
    // Core features are listed too, with exact plan locations.
    let description = weave_language::describe_contract(&example("temporal.weave")).unwrap();
    let window = description
        .contract
        .features
        .iter()
        .find(|f| f.feature == "window")
        .unwrap();
    assert_eq!((window.since, window.construct), ("0.19", "expression"));
    assert!(window.locations[0].starts_with("/program/commands/"));
}

#[test]
fn cli_describe_contract_is_explicit_and_legacy_describe_is_unchanged() {
    let handlers = "examples/handlers.weave";
    let out = weave(&["describe", handlers, "--contract"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["contract"]["core_only"], false);
    assert!(
        value["contract"]["features"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| {
                f["feature"] == "compiled_handler_template"
                    && f["class"] == "deferred"
                    && f["locations"][0] == "/handler_templates/Warning"
            })
    );
    // The legacy command keeps its shape and still rejects unreturned templates.
    let legacy = weave(&["describe", handlers]);
    assert!(!legacy.status.success() && legacy.stdout.is_empty());
    let schema = weave(&["describe", "examples/schema.weave"]);
    assert!(
        serde_json::from_slice::<Value>(&schema.stdout)
            .unwrap()
            .is_array()
    );
    // Linked modules are described the same way.
    let linked = weave(&[
        "describe",
        "examples/modules/main.weave",
        "--modules",
        "examples/modules/map.json",
        "--contract",
    ]);
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );
    let value: Value = serde_json::from_slice(&linked.stdout).unwrap();
    assert!(!value["schemas"].as_array().unwrap().is_empty());
    assert_eq!(value["contract"]["profile"], "core-v1");
    // The flag belongs to describe only and is not repeatable.
    for args in [
        &["plan", "examples/fleet.weave", "--contract"][..],
        &[
            "describe",
            "examples/fleet.weave",
            "--contract",
            "--contract",
        ][..],
    ] {
        assert_eq!(weave(args).status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn describe_contract_classifies_fact_writes_and_provenance_as_core_v1() {
    // The compiler never emits `write`; classify the engine's golden program.
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(root().join(
            "vendor/weave-contract/fixtures/core-v1/positive/fact-write-assert-correct-retract.json",
        ))
        .unwrap(),
    )
    .unwrap();
    let mut input = fixture["input"].clone();
    // A node carrying 0.22 valid time and claimed provenance, plus an
    // extension field, inside a write change.
    let node = &mut input["commands"][0]["changes"][0]["fact"];
    node["valid_time"] = serde_json::json!({"start": 0, "end": 10});
    node["provenance"] = serde_json::json!({"source": "sensor", "method": {"kind": "human"}});
    node["context_scope"] = serde_json::json!({"kind": "default"});
    let program = match decode_program(&serde_json::to_vec(&input).unwrap()) {
        Ok(program) => program,
        Err(error) => panic!("{error}"),
    };
    let profile = weave_language::contract_profile::classify(
        &program,
        &Default::default(),
        &Default::default(),
    );
    let write = profile
        .features
        .iter()
        .find(|f| f.feature == "write")
        .unwrap();
    assert_eq!(
        (write.since, write.construct, write.class),
        ("0.22", "command", FeatureClass::CoreV1)
    );
    let writes = input["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["op"] == "write")
        .count();
    assert!(writes >= 2);
    assert_eq!(write.locations.len(), writes);
    let scope = profile
        .features
        .iter()
        .find(|f| f.feature == "context_scope")
        .unwrap();
    assert_eq!(scope.class, FeatureClass::Extension);
    assert_eq!(
        scope.locations,
        ["/program/commands/0/changes/0/fact/context_scope"]
    );
    assert!(!profile.core_only);
    // Node valid time and claimed provenance are core v1 data, not extensions.
    let features: BTreeSet<_> = profile.features.iter().map(|f| f.feature).collect();
    assert_eq!(
        features,
        BTreeSet::from(["context_scope", "query", "write"])
    );
}
