use weave_language::modules::{SourceModule, compile_with_modules, content_digest, link};

fn import(alias: &str, id: &str, source: &str) -> String {
    format!(
        "import {alias} module {id:?} revision \"1\" sha256 {:?};\n",
        content_digest(source)
    )
}
const CORE: &str = r#"module "core" revision "1";
schema Fleet revision "1" { node Device space "s" { property "span" string optional; } }
function Keep revision "1" (graph input schema Fleet) returns graph schema Fleet { return input; }
"#;
fn module<'a>(id: &'a str, source: &'a str) -> SourceModule<'a> {
    SourceModule {
        id,
        revision: "1",
        source,
    }
}
#[test]
fn exact_imports_qualify_schema_and_function_identity_without_rewriting_literals() {
    let entry = import("tools", "core", CORE)
        + r#"graph G schema tools::Fleet {node "a" type Device entity "tools::Keep" space "s" property "span" "tools::Fleet";}
apply Output from tools::Keep {graph input G;}"#;
    let plan = compile_with_modules("main.weave", &entry, &[module("core", CORE)]).unwrap();
    let json = serde_json::to_value(&plan).unwrap();
    assert_eq!(
        json["commands"][0]["data"]["schema"]["id"],
        "module:core:schema:Fleet"
    );
    assert_eq!(
        json["commands"][0]["data"]["nodes"][0]["entity_id"],
        "tools::Keep"
    );
    assert_eq!(
        json["commands"][0]["data"]["nodes"][0]["properties"]["span"],
        "tools::Fleet"
    );
    assert!(
        plan.source_revisions
            .iter()
            .any(|r| r.name == "module:core:function:Keep")
    );
    assert!(
        plan.source_revisions
            .iter()
            .any(|r| r.name == "module:core"
                && r.digest == format!("sha256:{}", content_digest(CORE)))
    );
    assert_eq!(
        weave_language::compile(&entry).unwrap_err().code,
        "E_MODULE_RESOLUTION"
    );
}
#[test]
fn aliases_and_import_order_do_not_change_linked_identity_and_diamonds_deduplicate() {
    let left = format!(
        "module \"left\" revision \"1\";\n{}function L revision \"1\" (graph input schema c::Fleet) returns graph schema c::Fleet {{apply V from c::Keep {{graph input input;}} return V;}}",
        import("c", "core", CORE)
    );
    let right = format!(
        "module \"right\" revision \"1\";\n{}function R revision \"1\" (graph input schema common::Fleet) returns graph schema common::Fleet {{apply V from common::Keep {{graph input input;}} return V;}}",
        import("common", "core", CORE)
    );
    let declarations = r#"graph G schema C::Fleet {} apply A from L::L {graph input G;} apply B from R::R {graph input A;}"#;
    let first = import("L", "left", &left)
        + &import("R", "right", &right)
        + &import("C", "core", CORE)
        + declarations;
    let second = import("C", "core", CORE)
        + &import("R", "right", &right)
        + &import("L", "left", &left)
        + declarations;
    let units = [
        module("right", &right),
        module("core", CORE),
        module("left", &left),
    ];
    let a = link("entry", &first, &units).unwrap();
    let b = link("different-local-path", &second, &units).unwrap();
    assert_eq!(a.compile().unwrap(), b.compile().unwrap());
    assert_eq!(a.fingerprint().unwrap(), b.fingerprint().unwrap());
    let renamed = first
        .replace("import L ", "import Renamed ")
        .replace("L::L", "Renamed::L");
    assert_eq!(
        a.fingerprint().unwrap(),
        link("entry", &renamed, &units)
            .unwrap()
            .fingerprint()
            .unwrap()
    );
    assert_eq!(
        a.compile()
            .unwrap()
            .source_revisions
            .iter()
            .filter(|r| r.name == "module:core")
            .count(),
        1
    );
}
#[test]
fn pins_headers_missing_units_kinds_and_imported_effects_fail() {
    let mut bad = import("c", "core", CORE);
    bad = bad.replace(&content_digest(CORE), &"0".repeat(64));
    assert_eq!(
        compile_with_modules("entry", &bad, &[module("core", CORE)])
            .unwrap_err()
            .code,
        "E_MODULE_DIGEST"
    );
    assert_eq!(
        compile_with_modules("entry", &import("c", "core", CORE), &[])
            .unwrap_err()
            .code,
        "E_MODULE_MISSING"
    );
    let effect = "module \"bad\" revision \"1\"; graph Hidden {}";
    assert_eq!(
        compile_with_modules(
            "entry",
            &import("b", "bad", effect),
            &[module("bad", effect)]
        )
        .unwrap_err()
        .code,
        "E_MODULE_EFFECT"
    );
    let wrong = "module \"elsewhere\" revision \"1\";";
    assert_eq!(
        compile_with_modules("entry", &import("b", "bad", wrong), &[module("bad", wrong)])
            .unwrap_err()
            .code,
        "E_MODULE_HEADER"
    );
    let entry = import("c", "core", CORE) + "apply V from c::Fleet {}";
    assert_eq!(
        compile_with_modules("entry", &entry, &[module("core", CORE)])
            .unwrap_err()
            .code,
        "E_MODULE_KIND"
    );
    let entry = import("c", "core", CORE) + "apply V from c::Missing {}";
    let error = compile_with_modules("entry", &entry, &[module("core", CORE)]).unwrap_err();
    assert_eq!(error.code, "E_MODULE_EXPORT");
    assert_eq!(&entry[error.start..error.end], "c::Missing");
    let duplicated = import("c", "core", CORE) + &import("c", "core", CORE);
    assert_eq!(
        compile_with_modules("entry", &duplicated, &[module("core", CORE)])
            .unwrap_err()
            .code,
        "E_DUPLICATE"
    );
}
#[test]
fn diagnostics_keep_imported_original_spans_and_trace_after_qualification() {
    let broken = r#"module "broken" revision "1";
// Unicode λ, earlier Missing and renamed symbols do not affect the diagnostic.
function Bad revision "1" (graph input) { lens V from Missing {} return V; }
"#;
    let outer = format!(
        "module \"outer\" revision \"1\";{}",
        import("b", "broken", broken)
    );
    let entry = import("o", "outer", &outer);
    let error = compile_with_modules(
        "main",
        &entry,
        &[module("outer", &outer), module("broken", broken)],
    )
    .unwrap_err();
    assert_eq!(error.source_id, "module:broken@1");
    assert_eq!(&broken[error.start..error.end], "Missing");
    assert_eq!(error.start, broken.rfind("Missing").unwrap());
    assert_eq!(error.import_trace.len(), 2);
    assert_eq!(error.import_trace[0].source_id, "main");
    assert_eq!(error.import_trace[1].source_id, "module:outer@1");
}
#[test]
fn module_namespaces_are_nominal_and_changed_pins_change_plan_identity() {
    let other = CORE.replace("module \"core\"", "module \"other\"");
    let entry = import("c", "core", CORE)
        + &import("o", "other", &other)
        + "graph G schema c::Fleet {} apply Bad from o::Keep {graph input G;}";
    assert_eq!(
        compile_with_modules(
            "entry",
            &entry,
            &[module("core", CORE), module("other", &other)]
        )
        .unwrap_err()
        .code,
        "E_SCHEMA_MISMATCH"
    );
    let changed = format!("{CORE}\n// exact transport bytes changed\n");
    let a = link("entry", &import("c", "core", CORE), &[module("core", CORE)])
        .unwrap()
        .fingerprint()
        .unwrap();
    let b = link(
        "entry",
        &import("c", "core", &changed),
        &[module("core", &changed)],
    )
    .unwrap()
    .fingerprint()
    .unwrap();
    assert_ne!(a, b);
    assert_eq!(
        compile_with_modules(
            "entry",
            "",
            &[module("core", CORE), module("core", &changed)]
        )
        .unwrap_err()
        .code,
        "E_MODULE_REVISION"
    );
}

#[test]
fn cycles_depth_units_and_precise_schema_spans_are_bounded() {
    // Cycles cannot satisfy recursive raw-content hashes in ordinary authored files.
    // A back-edge is rejected before its impossible digest would be checked.
    let b = format!(
        "module \"b\" revision \"1\"; import a module \"a\" revision \"1\" sha256 \"{}\";",
        "0".repeat(64)
    );
    let a = format!("module \"a\" revision \"1\"; {}", import("b", "b", &b));
    let entry = import("a", "a", &a);
    assert_eq!(
        compile_with_modules("entry", &entry, &[module("a", &a), module("b", &b)])
            .unwrap_err()
            .code,
        "E_MODULE_CYCLE"
    );
    let mut sources = vec![String::new(); 17];
    sources[16] = "module \"m16\" revision \"1\";".into();
    for i in (0..16).rev() {
        sources[i] = format!(
            "module \"m{i}\" revision \"1\"; {}",
            import("next", &format!("m{}", i + 1), &sources[i + 1])
        );
    }
    let ids: Vec<_> = (0..17).map(|i| format!("m{i}")).collect();
    let supplied: Vec<_> = ids
        .iter()
        .zip(&sources)
        .map(|(id, source)| module(id, source))
        .collect();
    assert_eq!(
        compile_with_modules("entry", &import("m", "m0", &sources[0]), &supplied)
            .unwrap_err()
            .code,
        "E_MODULE_BUDGET"
    );
    let too_many = vec![module("core", CORE); 65];
    assert_eq!(
        compile_with_modules("entry", "", &too_many)
            .unwrap_err()
            .code,
        "E_MODULE_BUDGET"
    );
    let source = import("c", "core", CORE) + "graph G schema c::Missing {}";
    let error = compile_with_modules("entry", &source, &[module("core", CORE)]).unwrap_err();
    assert_eq!(&source[error.start..error.end], "c::Missing");
}

#[test]
fn module_labels_cannot_collide_with_declaration_namespaces() {
    for id in ["core:function:Keep", "ü"] {
        let unit = format!("module {id:?} revision \"1\";");
        let entry = import("m", id, &unit);
        assert_eq!(
            compile_with_modules("entry", &entry, &[module(id, &unit)])
                .unwrap_err()
                .code,
            "E_MODULE_BUDGET"
        );
    }
    let duplicate = "module \"core\" revision \"1\"; schema X revision \"1\" {} function X revision \"1\" (graph input) {return input;}";
    assert_eq!(
        compile_with_modules(
            "entry",
            &import("m", "core", duplicate),
            &[module("core", duplicate)]
        )
        .unwrap_err()
        .code,
        "E_DUPLICATE"
    );
}

#[test]
fn authored_internal_names_cannot_bypass_explicit_imports() {
    let outer = format!(
        "module \"outer\" revision \"1\"; {}",
        import("c", "core", CORE)
    );
    let forged = format!("__weave_module_{}_Keep", content_digest("core"));
    let entry = import("o", "outer", &outer)
        + &format!("graph G {{}} apply V from {forged} {{graph input G;}}");
    let error = compile_with_modules(
        "entry",
        &entry,
        &[module("core", CORE), module("outer", &outer)],
    )
    .unwrap_err();
    assert_eq!(error.code, "E_MODULE_NAME");
    assert_eq!(&entry[error.start..error.end], forged);
}

#[test]
fn cached_dependency_height_cannot_bypass_longest_path_limit() {
    let mut sources = vec![String::new(); 20];
    let ids: Vec<_> = (0..20)
        .map(|i| {
            if i < 10 {
                format!("a{i}")
            } else {
                format!("z{i}")
            }
        })
        .collect();
    sources[9] = format!("module {:?} revision \"1\";", ids[9]);
    for i in (0..9).rev() {
        sources[i] = format!(
            "module {:?} revision \"1\"; {}",
            ids[i],
            import("next", &ids[i + 1], &sources[i + 1])
        );
    }
    sources[19] = format!(
        "module {:?} revision \"1\"; {}",
        ids[19],
        import("next", &ids[0], &sources[0])
    );
    for i in (10..19).rev() {
        sources[i] = format!(
            "module {:?} revision \"1\"; {}",
            ids[i],
            import("next", &ids[i + 1], &sources[i + 1])
        );
    }
    let supplied: Vec<_> = ids
        .iter()
        .zip(&sources)
        .map(|(id, s)| module(id, s))
        .collect();
    let entry = import("early", &ids[0], &sources[0]) + &import("late", &ids[10], &sources[10]);
    assert_eq!(
        compile_with_modules("entry", &entry, &supplied)
            .unwrap_err()
            .code,
        "E_MODULE_BUDGET"
    );
}

#[test]
fn rule_and_context_schema_exports_keep_canonical_runtime_identities() {
    let source = r#"module "semantics" revision "1";
context_schema World revision "1" { axis "region" enum ["EE", "FI"]; }
rules Reach revision "1" { rule Seed { when "link"(x, y); yield "reach"(x, y); } }
function Closure revision "1" (graph input) { reason result from input using Reach; return result; }
"#;
    let entry = import("s", "semantics", source)
        + r#"
context_value Estonia schema s::World source "author" {axis "region" "EE";}
graph G {node "a" entity "a" space "s";node "b" entity "b" space "s";edge "ab" from "a" to "b" relation "link" valid 0 until 10;}
reason R from G using s::Reach;
apply ViaFunction from s::Closure {graph input G;}
"#;
    let plan = compile_with_modules("entry", &entry, &[module("semantics", source)]).unwrap();
    let encoded = serde_json::to_value(&plan).unwrap();
    let claim = &encoded["commands"][0]["data"]["assertions"][0];
    assert_eq!(
        claim["properties"]["weave.context"]["schema"]["reference"]["id"],
        "module:semantics:context:World"
    );
    assert!(
        plan.source_revisions
            .iter()
            .any(|r| r.name == "module:semantics:rules:Reach")
    );
    assert!(
        serde_json::to_string(&plan.commands)
            .unwrap()
            .contains("module:semantics:rules:Reach")
    );
}
