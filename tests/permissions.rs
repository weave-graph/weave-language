//! LB1 (weave-language#5): reader syntax on writes and snapshot records, the
//! `correct` restatement rule, the untrusted and read-only compile policies,
//! permission-aware diagnostics against a visible manifest, and plan effects.
//! Every emitted plan round-trips through the contract's strict decoder.
use serde_json::{Value, json};
use weave_contract::decode_program;
use weave_language::policy::{
    Access, CompilePolicy, Permission, VisibleGraph, VisibleManifest, plan_effects,
};
use weave_language::{compile, compile_artifacts_with_policy, compile_with_policy};

const NOTES: &str = r#"embedding_space Notes id "notes" revision "1" encoder "enc" version "v1"
  preprocessing "words/v1" dimensions 3 metric cosine_distance;
"#;

fn round_trip(program: &weave_contract::Program, what: &str) -> Value {
    let value = serde_json::to_value(program).unwrap();
    let decoded = decode_program(&serde_json::to_vec(&value).unwrap())
        .unwrap_or_else(|e| panic!("{what}: not core v1: {e}"));
    assert_eq!(serde_json::to_value(decoded).unwrap(), value, "{what}");
    value
}

fn plan(source: &str) -> Value {
    let program = compile(source).unwrap_or_else(|e| panic!("{source}: {e:?}"));
    round_trip(&program, source)
}

/// The code and the exact source text a diagnostic points at.
fn error(source: &str) -> (String, String) {
    let e = compile(source).expect_err(source);
    (e.code.clone(), source[e.start..e.end].to_owned())
}

fn policy_error(source: &str, policy: &CompilePolicy) -> (String, String, String) {
    let e = compile_artifacts_with_policy(source, policy).expect_err(source);
    (e.code.clone(), source[e.start..e.end].to_owned(), e.message)
}

fn fact(plan: &Value, command: usize, change: usize) -> &Value {
    &plan["commands"][command]["changes"][change]["fact"]
}

#[test]
fn readers_and_reader_groups_lower_to_the_contract_fields() {
    let source = format!(
        r#"{NOTES}write W graph "fleet" {{
  assert node "n1" entity "N1" space "notes" property "text" "fan" reader_groups "maintenance", "ops";
  assert edge "e1" from "n1" to "u1" relation "about" valid 0 until infinity readers "alice", "bob" reader_groups "maintenance";
  assert embedding "emb-1" in Notes subject node "n1" vector [1.0, 0.0, 0.0] valid 0 until infinity readers "alice";
  assert node "u1" entity "U1" space "fleet" public;
  assert node "u2" entity "U2" space "fleet";
}}"#
    );
    let p = plan(&source);
    assert_eq!(fact(&p, 0, 0)["readers"], json!([]));
    assert_eq!(
        fact(&p, 0, 0)["reader_groups"],
        json!(["maintenance", "ops"])
    );
    assert_eq!(fact(&p, 0, 1)["readers"], json!(["alice", "bob"]));
    assert_eq!(fact(&p, 0, 1)["reader_groups"], json!(["maintenance"]));
    assert_eq!(fact(&p, 0, 2)["readers"], json!(["alice"]));
    assert!(fact(&p, 0, 2).get("reader_groups").is_none());
    // `public` and no clause both lower to empty readers, with groups omitted:
    // the bytes of a public record are unchanged from before LB1.
    for change in [3, 4] {
        assert_eq!(fact(&p, 0, change)["readers"], json!([]));
        assert!(fact(&p, 0, change).get("reader_groups").is_none());
    }
    assert_eq!(fact(&p, 0, 3), &{
        let mut node = fact(&p, 0, 4).clone();
        node["id"] = json!("u1");
        node["entity_id"] = json!("U1");
        node
    });
    assert_eq!(p["version"], "1.1.0");
}

#[test]
fn clauses_mix_with_properties_and_metadata_in_any_order() {
    let a = plan(
        r#"write W graph "g" { assert node "n" entity "N" space "s" reader_groups "team" property "k" 1 metadata graph "m" revision "r" readers "alice"; }"#,
    );
    let b = plan(
        r#"write W graph "g" { assert node "n" entity "N" space "s" readers "alice" metadata graph "m" revision "r" property "k" 1 reader_groups "team"; }"#,
    );
    assert_eq!(a, b);
}

#[test]
fn snapshot_records_carry_readers_too() {
    let p = plan(
        r#"graph Plain {
  node "a" entity "A" space "s" reader_groups "maintenance";
  node "b" entity "B" space "s" public;
  edge "ab" from "a" to "b" relation "r" valid 0 until 10 readers "alice" reader_groups "maintenance";
  attachment "m" on node "a" key "k" graph "Other" revision "r1" valid 0 until 10 readers "bob";
}
graph Claims explicit {
  node "a" entity "A" space "s";
  node "b" entity "B" space "s";
  relation "rel" from "a" to "b" predicate "p" reader_groups "maintenance";
  claim "c" on "rel" source "sensor" polarity positive valid 0 until 10 readers "alice";
}"#,
    );
    let data = &p["commands"][0]["data"];
    assert_eq!(data["nodes"][0]["reader_groups"], json!(["maintenance"]));
    assert!(data["nodes"][1].get("reader_groups").is_none());
    assert_eq!(data["edges"][0]["readers"], json!(["alice"]));
    assert_eq!(data["edges"][0]["reader_groups"], json!(["maintenance"]));
    assert_eq!(data["attachments"][0]["readers"], json!(["bob"]));
    let explicit = &p["commands"][1]["data"];
    assert_eq!(
        explicit["structural_edges"][0]["reader_groups"],
        json!(["maintenance"])
    );
    assert_eq!(explicit["assertions"][0]["readers"], json!(["alice"]));
}

#[test]
fn correct_must_restate_readers_or_say_public() {
    // The logged LA1 risk: a correction without readers would publish a
    // restricted fact's new version. It is now a compile-time error.
    for (fact, text) in [
        (
            r#"correct node "n" entity "N" space "s" property "k" 1"#,
            r#"correct node "n" entity "N" space "s" property "k" 1;"#,
        ),
        (
            r#"correct edge "e" from "a" to "b" relation "r" valid 0 until 5"#,
            r#"correct edge "e" from "a" to "b" relation "r" valid 0 until 5;"#,
        ),
        (
            r#"correct embedding "x" in Notes subject node "n" vector [1.0, 0.0, 0.0] valid 0 until 5"#,
            r#"correct embedding "x" in Notes subject node "n" vector [1.0, 0.0, 0.0] valid 0 until 5;"#,
        ),
    ] {
        let source = format!("{NOTES}write W graph \"g\" {{ {fact}; }}");
        assert_eq!(
            error(&source),
            ("E_VISIBILITY_REQUIRED".into(), text.into()),
            "{fact}"
        );
        for clause in [" public", " readers \"alice\"", " reader_groups \"team\""] {
            plan(&format!("{NOTES}write W graph \"g\" {{ {fact}{clause}; }}"));
        }
    }
    // Asserting and retracting are unchanged: an assert cannot replace a live
    // fact, and a retraction writes no new version.
    plan(r#"write W graph "g" { assert node "n" entity "N" space "s"; retract node "m"; }"#);
    let p = plan(
        r#"write W graph "g" { correct node "n" entity "N" space "s" reader_groups "team"; }"#,
    );
    assert_eq!(p["commands"][0]["changes"][0]["change"], "correct");
    assert_eq!(fact(&p, 0, 0)["reader_groups"], json!(["team"]));
}

#[test]
fn visibility_clause_diagnostics() {
    let node = r#"write W graph "g" { assert node "n" entity "N" space "s""#;
    for (clauses, code, text) in [
        (r#" public readers "a""#, "E_VISIBILITY", "readers"),
        (r#" reader_groups "t" public"#, "E_VISIBILITY", "public"),
        (r#" readers "a" readers "b""#, "E_DUPLICATE", "readers"),
        (r#" public public"#, "E_DUPLICATE", "public"),
        (r#" readers "a", "b", "a""#, "E_DUPLICATE", r#""a""#),
        (r#" reader_groups """#, "E_ID", r#""""#),
        (r#" readers "a","#, "E_PARSE", ";"),
    ] {
        let source = format!("{node}{clauses}; }}");
        let (actual_code, actual_text) = error(&source);
        assert_eq!(
            (actual_code.as_str(), actual_text.as_str()),
            (code, text),
            "{clauses}"
        );
    }
    let many: Vec<String> = (0..=256).map(|i| format!("\"p{i}\"")).collect();
    assert_eq!(
        error(&format!("{node} readers {}; }}", many.join(", "))),
        ("E_BUDGET".into(), "\"p256\"".into())
    );
    plan(&format!("{node} readers {}; }}", many[..256].join(", ")));
    // Spaces are immutable public descriptors: they take no clause.
    assert_eq!(
        error(&format!(
            "{NOTES}write W graph \"g\" {{ assert space Notes public; }}"
        ))
        .0,
        "E_PARSE"
    );
}

#[test]
fn restricted_writes_example_round_trips() {
    let source = include_str!("../examples/permissions/restricted_writes.weave");
    let p = plan(source);
    assert_eq!(fact(&p, 0, 2)["reader_groups"], json!(["maintenance"]));
    assert_eq!(fact(&p, 0, 5)["readers"], json!(["bob"]));
    assert_eq!(fact(&p, 1, 0)["reader_groups"], json!(["maintenance"]));
    let formatted = weave_language::format_source(source).unwrap();
    assert_eq!(plan(&formatted), p, "formatting keeps the clauses");
}

fn manifest(graphs: &[(&str, Permission)], groups: &[&str]) -> VisibleManifest {
    VisibleManifest {
        format: "weave-visible/1".into(),
        principal: "agent-1".into(),
        graphs: graphs
            .iter()
            .map(|(id, permission)| VisibleGraph {
                graph_id: (*id).into(),
                permission: *permission,
                branches: vec!["main".into()],
                schema: None,
                embedding_spaces: None,
            })
            .collect(),
        reader_groups: groups.iter().map(|g| (*g).into()).collect(),
    }
}

#[test]
fn untrusted_policy_requires_stated_readers_and_rejects_host_templates() {
    let untrusted = CompilePolicy {
        untrusted: true,
        ..Default::default()
    };
    let (code, text, _) = policy_error(
        r#"write W graph "g" { assert node "n" entity "N" space "s"; }"#,
        &untrusted,
    );
    assert_eq!(
        (code.as_str(), text.as_str()),
        (
            "E_VISIBILITY_REQUIRED",
            r#"assert node "n" entity "N" space "s";"#
        )
    );
    let (code, text, _) = policy_error(
        r#"graph G { node "a" entity "A" space "s" public; node "b" entity "B" space "s"; }"#,
        &untrusted,
    );
    assert_eq!(
        (code.as_str(), text.as_str()),
        ("E_VISIBILITY_REQUIRED", r#""b""#)
    );
    let (code, text, _) = policy_error(
        r#"transaction t { graph G { node "a" entity "A" space "s"; } }"#,
        &untrusted,
    );
    assert_eq!(
        (code.as_str(), text.as_str()),
        ("E_VISIBILITY_REQUIRED", r#""a""#)
    );
    let (code, _, _) = policy_error(
        r#"live_handle Head graph "Fleet" branch "main"; view_template Current revision "1" from Head clock tick { at 0; }"#,
        &untrusted,
    );
    assert_eq!(code, "E_UNTRUSTED_ARTIFACT");
    // Stated readers, retractions and reads all pass, and the plan is the
    // same as without the policy.
    let source = r#"write W graph "g" { assert node "n" entity "N" space "s" public; retract edge "e"; }
use G graph "g"; lens L from G { match relation "r"; }"#;
    let checked = compile_with_policy(source, &untrusted).unwrap();
    assert_eq!(round_trip(&checked, source), plan(source));
}

#[test]
fn read_only_policy_rejects_every_write() {
    let read_only = CompilePolicy {
        read_only: true,
        ..Default::default()
    };
    for (source, text) in [
        (r#"write W graph "g" { retract node "n"; }"#, "W"),
        (r#"graph G { node "a" entity "A" space "s"; }"#, "G"),
        (
            r#"transaction t { graph G { node "a" entity "A" space "s"; } }"#,
            "t",
        ),
        (
            r#"live_handle Head graph "Fleet" branch "main"; view_template Current revision "1" from Head clock tick { at 0; }"#,
            "Current",
        ),
    ] {
        let (code, actual, _) = policy_error(source, &read_only);
        assert_eq!((code.as_str(), actual.as_str()), ("E_EFFECT_DENIED", text));
    }
    for path in [
        "examples/phase_a/q2_as_known.weave",
        "examples/phase_a/q3_hybrid_retrieval.weave",
        "examples/phase_a/q4_candidate_relations.weave",
    ] {
        let source = std::fs::read_to_string(path).unwrap();
        let program = compile_with_policy(&source, &read_only).unwrap();
        assert!(
            plan_effects(&program)
                .iter()
                .all(|e| e.access != Access::Write),
            "{path}"
        );
    }
}

#[test]
fn visible_manifest_gives_engine_codes_at_compile_time() {
    let visible = |m: VisibleManifest| CompilePolicy {
        visible: Some(m),
        ..Default::default()
    };
    let m = manifest(
        &[
            ("fleet", Permission::Write),
            ("manuals", Permission::Read),
            ("audit", Permission::Discover),
        ],
        &["maintenance"],
    );
    // A missing graph and a hidden one give the same diagnostic.
    let (code, text, message) = policy_error(
        r#"use P graph "payroll"; lens L from P { }"#,
        &visible(m.clone()),
    );
    assert_eq!((code.as_str(), text.as_str()), ("E_UNAVAILABLE", "P"));
    assert_eq!(
        message,
        "Graph 'payroll' is not visible to principal 'agent-1': it does not exist or this principal cannot discover it"
    );
    let (code, text, _) = policy_error(
        r#"use A graph "audit"; lens L from A { }"#,
        &visible(m.clone()),
    );
    assert_eq!((code.as_str(), text.as_str()), ("E_FORBIDDEN", "A"));
    let (code, text, _) = policy_error(
        r#"write W graph "manuals" { retract node "n"; }"#,
        &visible(m.clone()),
    );
    assert_eq!((code.as_str(), text.as_str()), ("E_FORBIDDEN", "W"));
    let (code, text, _) = policy_error(
        r#"write W graph "fleet" { assert node "n" entity "N" space "s" reader_groups "maintenance", "finance"; }"#,
        &visible(m.clone()),
    );
    assert_eq!(
        (code.as_str(), text.as_str()),
        ("E_UNAVAILABLE", r#""finance""#)
    );
    // Plan-level backstop: graphs named by any operator are checked too.
    let (code, text, _) = policy_error(
        r#"recorded_handle H graph "payroll" branch "main" known_at 10; pin P from H at 7;"#,
        &visible(m.clone()),
    );
    assert_eq!(
        (code.as_str(), text.as_str()),
        ("E_UNAVAILABLE", r#""payroll""#)
    );
    // Within the manifest everything compiles to the ordinary plan.
    let source = r#"use F graph "fleet"; use M graph "manuals"; lens L from M { }
write W graph "fleet" { assert node "n" entity "N" space "s" reader_groups "maintenance"; }"#;
    assert_eq!(
        round_trip(&compile_with_policy(source, &visible(m)).unwrap(), source),
        plan(source)
    );
}

#[test]
fn visible_spaces_type_check_queries_against_the_store() {
    let stored: weave_contract::embedding::EmbeddingSpace = serde_json::from_value(json!({
        "id": "notes", "revision": "1", "encoder": "enc", "encoder_version": "v1",
        "preprocessing": "words/v1", "dimensions": 4, "metric": "cosine_distance"}))
    .unwrap();
    let mut m = manifest(&[("fleet", Permission::Write)], &[]);
    m.graphs[0].embedding_spaces = Some(vec![stored.clone()]);
    let policy = CompilePolicy {
        visible: Some(m.clone()),
        ..Default::default()
    };
    let query =
        "use F graph \"fleet\"; nearest N from F { space Notes; vector [1.0, 0.0, 0.0]; k 3; }";
    // The source declares 3 dimensions; the store's space has 4.
    let source = format!("{NOTES}{query}");
    let (code, text, message) = policy_error(&source, &policy);
    assert_eq!(
        (code.as_str(), text.as_str()),
        ("E_SPACE_MISMATCH", "Notes")
    );
    assert!(message.contains("4 dimensions"), "{message}");
    let fixed = format!(
        "{}{}",
        NOTES.replace("dimensions 3", "dimensions 4"),
        query.replace("[1.0, 0.0, 0.0]", "[1.0, 0.0, 0.0, 0.0]")
    );
    compile_with_policy(&fixed, &policy).unwrap();
    // A space the principal cannot see in that graph.
    m.graphs[0].embedding_spaces = Some(vec![]);
    let (code, _, _) = policy_error(
        &fixed,
        &CompilePolicy {
            visible: Some(m),
            ..Default::default()
        },
    );
    assert_eq!(code, "E_UNKNOWN_SPACE");
}

#[test]
fn manifests_decode_strictly() {
    let ok = json!({"format": "weave-visible/1", "principal": "p", "graphs": [
        {"graph_id": "g", "permission": "read", "branches": ["main"]}], "reader_groups": ["t"]});
    VisibleManifest::decode(&serde_json::to_vec(&ok).unwrap()).unwrap();
    for (change, code) in [
        (json!({"format": "weave-visible/2"}), "E_MANIFEST"),
        (json!({"principal": ""}), "E_MANIFEST"),
        (json!({"extra": true}), "E_MANIFEST"),
        (json!({"reader_groups": ["t", "t"]}), "E_MANIFEST"),
        (
            json!({"graphs": [{"graph_id": "g", "permission": "read"}, {"graph_id": "g", "permission": "write"}]}),
            "E_MANIFEST",
        ),
        (
            json!({"graphs": [{"graph_id": "g", "permission": "owner"}]}),
            "E_MANIFEST",
        ),
    ] {
        let mut value = ok.clone();
        for (k, v) in change.as_object().unwrap() {
            value[k] = v.clone();
        }
        let e = VisibleManifest::decode(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert_eq!(e.code, code, "{change}");
    }
}

#[test]
fn plan_effects_list_reads_references_and_writes() {
    let source = r#"graph Catalog {
  node "a" entity "A" space "s";
  attachment "m" on node "a" key "k" graph "Evidence" revision "r1" valid 0 until 10;
}
use F graph "fleet";
expand E from F { seeds "unit-1"; hops 1; }
write W graph "fleet" branch "draft" { retract node "n"; }"#;
    let program = compile(source).unwrap();
    let effects: Vec<_> = plan_effects(&program)
        .into_iter()
        .map(|e| (e.command, e.op, e.access, e.graph_id, e.branch_id))
        .collect();
    assert_eq!(
        effects,
        [
            (
                0,
                "commit".into(),
                Access::Reference,
                "Evidence".into(),
                None
            ),
            (
                0,
                "commit".into(),
                Access::Write,
                "Catalog".into(),
                Some("main".into())
            ),
            (
                1,
                "bind".into(),
                Access::Read,
                "fleet".into(),
                Some("main".into())
            ),
            (
                2,
                "write".into(),
                Access::Write,
                "fleet".into(),
                Some("draft".into())
            ),
        ]
    );
}
