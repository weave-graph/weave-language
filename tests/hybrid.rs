//! LA1 part 2 (weave-language#3): embedding spaces, fact-level writes,
//! `nearest`, `similar_candidates` and `expand ... seeds from` (protocol
//! 0.24, engine A4). Golden source-to-plan tests, strict round trips,
//! oracle-fixture equality, core v1 classification and diagnostics.
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use weave_contract::{Command, GraphExpression, decode_program};
use weave_language::contract_profile::FeatureClass;
use weave_language::{compile, describe_contract};

const NOTES: &str = r#"embedding_space Notes id "notes" revision "1" encoder "enc" version "v1"
  preprocessing "words/v1" dimensions 3 metric cosine_distance;
"#;

fn plan(source: &str) -> Value {
    let program = compile(source).unwrap_or_else(|e| panic!("{source}: {e:?}"));
    let value = serde_json::to_value(&program).unwrap();
    let decoded = decode_program(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), value, "{source}");
    value
}

fn error(source: &str) -> (String, String) {
    let e = compile(source).expect_err(source);
    (e.code.clone(), source[e.start..e.end].to_owned())
}

#[test]
fn writes_lower_to_one_fact_write_command_each() {
    let source = format!(
        r#"{NOTES}write Seed graph "g" branch "main" expected new {{
  assert node "u1" entity "U1" space "s" property "status" "active";
  assert node "n1" entity "N1" space "notes" valid 5 until infinity property "text" "fan";
  assert edge "e1" from "n1" to "u1" relation "about" valid 5 until 50 property "w" 1;
  assert space Notes;
  assert embedding "emb-1" in Notes subject node "n1" vector [1.0, 0.1, 0.0] valid 5 until infinity;
}}
write Fix graph "g" expected head "fact:sha256:00" {{
  correct edge "e1" from "n1" to "u1" relation "about" polarity negative valid 5 until 60;
  correct embedding "emb-1" in Notes subject entity "N1" vector [0.0, 1.0, 0.0] valid 5 until 9;
  retract node "u1";
  retract edge "e1";
  retract embedding "emb-1";
  retract attachment "a1";
}}"#
    );
    let p = plan(&source);
    let space = json!({"kind": "embedding_space", "id": "notes", "revision": "1", "encoder": "enc",
        "encoder_version": "v1", "preprocessing": "words/v1", "dimensions": 3, "metric": "cosine_distance"});
    let edge = |valid_end: i64, polarity: &str, properties: Value| {
        json!({"kind": "edge", "id": "e1", "predicate": "about", "from": "n1", "to": "u1",
            "valid_time": {"start": 5, "end": valid_end}, "polarity": polarity, "properties": properties,
            "metadata": [], "readers": [], "derived_from": []})
    };
    assert_eq!(
        p["commands"],
        json!([
            {"op": "write", "graph_id": "g", "branch_id": "main", "expected_head": null, "changes": [
                {"change": "assert", "fact": {"kind": "node", "id": "u1", "entity_id": "U1", "space_id": "s",
                    "properties": {"status": "active"}, "metadata": [], "readers": []}},
                {"change": "assert", "fact": {"kind": "node", "id": "n1", "entity_id": "N1", "space_id": "notes",
                    "valid_time": {"start": 5, "end": null}, "properties": {"text": "fan"}, "metadata": [], "readers": []}},
                {"change": "assert", "fact": edge(50, "positive", json!({"w": 1}))},
                {"change": "assert", "fact": space},
                {"change": "assert", "fact": {"kind": "embedding", "id": "emb-1", "space": {"id": "notes", "revision": "1"},
                    "subject": {"kind": "node", "node_id": "n1"}, "vector": [1.0, 0.1_f32, 0.0],
                    "valid_time": {"start": 5, "end": null}, "readers": []}}
            ]},
            {"op": "write", "graph_id": "g", "branch_id": "main", "expected_head": "fact:sha256:00", "changes": [
                {"change": "correct", "fact": edge(60, "negative", json!({}))},
                {"change": "correct", "fact": {"kind": "embedding", "id": "emb-1", "space": {"id": "notes", "revision": "1"},
                    "subject": {"kind": "entity", "entity_id": "N1"}, "vector": [0.0, 1.0, 0.0],
                    "valid_time": {"start": 5, "end": 9}, "readers": []}},
                {"change": "retract", "target": {"kind": "node", "id": "u1"}},
                {"change": "retract", "target": {"kind": "edge", "id": "e1"}},
                {"change": "retract", "target": {"kind": "embedding", "id": "emb-1"}},
                {"change": "retract", "target": {"kind": "attachment", "id": "a1"}}
            ]}
        ])
    );
    // A write without `expected` performs no head check.
    let unchecked = plan(r#"write W graph "g" { retract node "x"; }"#);
    assert!(unchecked["commands"][0].get("expected_head").is_none());
}

#[test]
fn nearest_lowers_every_query_form_and_composes_with_expand() {
    let source = format!(
        r#"{NOTES}vector_type V space {{"id":"notes","revision":"1","geometry":{{"kind":"embedding","encoder":"enc","preprocessing":"words/v1","dimensions":3,"metric":"cosine_distance"}}}} role embedding;
value Q vector V [0.0, 1.0, 0.0];
use Known graph "g" as_of 77;
use G graph "g";
expand Region from G {{ seeds "u1"; hops 2; direction both; }}
nearest Literal from Known {{ space Notes; vector [1.0, 0.0, 0.0]; k 10; at 50; candidates Region; }}
nearest Typed from G {{ space Notes; vector value Q; k 1; }}
nearest Like from G {{ space Notes; like embedding "emb-1"; k 2; }}
expand Back from G {{ seeds "s0"; seeds from Literal; hops 1; match relation "about"; }}"#
    );
    let p = plan(&source);
    let value = |name: &str| {
        p["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap()["value"]
            .clone()
    };
    let space = json!({"id": "notes", "revision": "1"});
    assert_eq!(
        value("Literal"),
        json!({"kind": "nearest", "graph_id": "g", "branch_id": "main", "as_of": 77, "space": space,
            "query": {"kind": "vector", "vector": [1.0, 0.0, 0.0]}, "k": 10, "valid_at": 50,
            "candidates": {"kind": "reference", "name": "Region"}})
    );
    assert_eq!(
        value("Typed")["query"],
        json!({"kind": "vector", "vector": [0.0, 1.0, 0.0]})
    );
    assert_eq!(
        value("Like")["query"],
        json!({"kind": "embedding", "embedding_id": "emb-1"})
    );
    assert_eq!(
        value("Back"),
        json!({"kind": "expand", "graph_id": "g", "branch_id": "main", "seeds": ["s0"],
            "seeds_from": {"kind": "reference", "name": "Literal"}, "hops": 1, "direction": "out",
            "predicates": ["about"], "limit": 10000})
    );
}

#[test]
fn similar_candidates_lowers_every_clause() {
    let p = plan(&format!(
        r#"{NOTES}use G graph "g" revision "fact:sha256:01";
expand A from G {{ seeds "a"; hops 1; }}
expand B from G {{ seeds "b"; hops 1; }}
similar_candidates All from G {{ space Notes; k 1; }}
similar_candidates Scoped from G {{ space Notes; k 3; max_score 0.25; at 9; sources A; targets B; exclude_linked; limit 50; }}"#
    ));
    let commands = p["commands"].as_array().unwrap();
    assert_eq!(
        commands[2]["value"],
        json!({"kind": "similar_candidates", "graph_id": "g", "branch_id": "main", "revision": "fact:sha256:01",
            "space": {"id": "notes", "revision": "1"}, "k": 1, "limit": 1000})
    );
    assert_eq!(
        commands[3]["value"],
        json!({"kind": "similar_candidates", "graph_id": "g", "branch_id": "main", "revision": "fact:sha256:01",
            "space": {"id": "notes", "revision": "1"}, "k": 3, "max_score": 0.25, "valid_at": 9,
            "sources": {"kind": "reference", "name": "A"}, "targets": {"kind": "reference", "name": "B"},
            "exclude_linked": true, "limit": 50})
    );
}

/// Inline references to earlier bindings, as the engine fixtures write them.
fn inlined(program: &weave_contract::Program) -> BTreeMap<String, GraphExpression> {
    fn walk(value: Value, bound: &BTreeMap<String, Value>) -> Value {
        match value {
            Value::Object(map) if map.get("kind") == Some(&json!("reference")) => {
                bound[map["name"].as_str().unwrap()].clone()
            }
            Value::Object(map) => map.into_iter().map(|(k, v)| (k, walk(v, bound))).collect(),
            Value::Array(items) => items.into_iter().map(|v| walk(v, bound)).collect(),
            other => other,
        }
    }
    let mut bound = BTreeMap::new();
    for command in &program.commands {
        if let Command::Bind { name, value } = command {
            let value = walk(serde_json::to_value(value).unwrap(), &bound);
            bound.insert(name.clone(), value);
        }
    }
    bound
        .into_iter()
        .map(|(k, v)| (k, serde_json::from_value(v).unwrap()))
        .collect()
}

#[test]
fn examples_compile_to_the_engine_a4_oracle_fixture_expressions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (fixture, source, names) in [
        (
            "embedding-write-and-nearest",
            "embedding_write_and_nearest",
            &["AtFifty", "AtOneFifty"][..],
        ),
        (
            "nearest-composition",
            "nearest_composition",
            &["NotesNearU1", "FromBest"][..],
        ),
        (
            "similar-candidates",
            "similar_candidates",
            &["All", "Unlinked"][..],
        ),
    ] {
        let text =
            std::fs::read_to_string(root.join(format!("examples/hybrid/{source}.weave"))).unwrap();
        let program = compile(&text).unwrap();
        let fixture: Value = serde_json::from_slice(
            &std::fs::read(root.join(format!(
                "vendor/weave-contract/fixtures/core-v1/positive/{fixture}.json"
            )))
            .unwrap(),
        )
        .unwrap();
        let expected: Vec<GraphExpression> = fixture["input"]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["op"] == "evaluate")
            .map(|c| serde_json::from_value(c["value"].clone()).unwrap())
            .collect();
        let bound = inlined(&program);
        let compiled: Vec<GraphExpression> = names.iter().map(|n| bound[*n].clone()).collect();
        assert_eq!(
            compiled, expected,
            "{source} differs from fixture {fixture}"
        );
        // The fixture's embedding space is the one the source declares.
        let space = fixture["input"]["commands"][0]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["fact"]["kind"] == "embedding_space")
            .unwrap()["fact"]
            .clone();
        let write = plan(&format!(
            "{} write W graph \"g\" {{ assert space Notes; }}",
            text.lines()
                .filter(|l| !l.starts_with("//"))
                .take(2)
                .collect::<Vec<_>>()
                .join("\n")
        ));
        assert_eq!(
            write["commands"][0]["changes"][0]["fact"], space,
            "{source}"
        );
    }
}

#[test]
fn describe_contract_classifies_writes_nearest_and_candidates_as_core_v1() {
    let description = describe_contract(&format!(
        r#"{NOTES}write W graph "g" {{ assert space Notes; assert node "n" entity "N" space "s"; assert embedding "e" in Notes subject node "n" vector [1, 0, 0] valid 0 until infinity; }}
use G graph "g";
nearest Hits from G {{ space Notes; vector [1, 0, 0]; k 1; }}
expand Back from G {{ seeds from Hits; hops 1; }}
similar_candidates Pairs from G {{ space Notes; k 1; sources Back; }}"#
    ))
    .unwrap();
    let profile = &description.contract;
    assert!(profile.core_only, "{:?}", profile.features);
    let find = |name: &str| profile.features.iter().find(|f| f.feature == name).unwrap();
    for (name, since, locations) in [
        ("write", "0.22", vec!["/program/commands/0"]),
        ("nearest", "0.24", vec!["/program/commands/1/value"]),
        (
            "similar_candidates",
            "0.24",
            vec!["/program/commands/3/value"],
        ),
        ("expand", "0.23", vec!["/program/commands/2/value"]),
        (
            "reference",
            "0.3",
            vec![
                "/program/commands/2/value/seeds_from",
                "/program/commands/3/value/sources",
            ],
        ),
    ] {
        let entry = find(name);
        assert_eq!(entry.class, FeatureClass::CoreV1, "{name}");
        assert_eq!(entry.since, since, "{name}");
        assert_eq!(entry.locations, locations, "{name}");
    }
}

#[test]
fn the_phase_a_exit_questions_fit_in_fifteen_lines() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (file, kinds) in [
        ("q1_fact_writes", &["write", "write"][..]),
        ("q2_as_known", &["expand", "explain", "changes"][..]),
        ("q3_hybrid_retrieval", &["expand", "nearest"][..]),
        ("q4_candidate_relations", &["similar_candidates"][..]),
        (
            "q5_restricted",
            &["expand", "similar_candidates", "expand", "changes"][..],
        ),
    ] {
        let text =
            std::fs::read_to_string(root.join(format!("examples/phase_a/{file}.weave"))).unwrap();
        assert!(text.lines().count() <= 15, "{file}");
        let p = plan(&text);
        let got: Vec<&str> = p["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                if c["op"] == "write" {
                    "write"
                } else {
                    c["value"]["kind"].as_str().unwrap()
                }
            })
            .collect();
        assert_eq!(got, kinds, "{file}");
        assert!(
            describe_contract(&text).unwrap().contract.core_only,
            "{file}"
        );
    }
}

#[test]
fn space_and_descriptor_mismatches_are_compile_time_diagnostics() {
    let typed = |metric: &str, dims: usize, encoder: &str| {
        format!(
            r#"vector_type V space {{"id":"notes","revision":"1","geometry":{{"kind":"embedding","encoder":"{encoder}","preprocessing":"words/v1","dimensions":{dims},"metric":"{metric}"}}}} role embedding;
value Q vector V [{}];
"#,
            vec!["1.0"; dims].join(", ")
        )
    };
    let units = r#"embedding_space Units id "units" revision "1" encoder "enc" version "v1" preprocessing "words/v1" dimensions 3 metric cosine_distance;
"#;
    let use_g = "use G graph \"g\";\n";
    for (source, code, span) in [
        // A literal query vector of the wrong dimension.
        (
            format!("{NOTES}{use_g}nearest N from G {{ space Notes; vector [1.0, 0.0]; k 1; }}"),
            "E_SPACE_MISMATCH",
            "[1.0, 0.0]",
        ),
        // A zero vector in a cosine space.
        (
            format!("{NOTES}{use_g}nearest N from G {{ space Notes; vector [0, 0, 0]; k 1; }}"),
            "E_SPACE_MISMATCH",
            "[0, 0, 0]",
        ),
        // A typed vector whose encoder differs: equal dimensions never suffice.
        (
            format!(
                "{NOTES}{}{use_g}nearest N from G {{ space Notes; vector value Q; k 1; }}",
                typed("cosine_distance", 3, "other")
            ),
            "E_SPACE_MISMATCH",
            "Q",
        ),
        // A typed vector whose metric differs.
        (
            format!(
                "{NOTES}{}{use_g}nearest N from G {{ space Notes; vector value Q; k 1; }}",
                typed("euclidean", 3, "enc")
            ),
            "E_SPACE_MISMATCH",
            "Q",
        ),
        // A query fact written in another space by this program.
        (
            format!(
                "{NOTES}{units}write W graph \"g\" {{ assert embedding \"e\" in Units subject node \"n\" vector [1, 0, 0] valid 0 until infinity; }}\n{use_g}nearest N from G {{ space Notes; like embedding \"e\"; k 1; }}"
            ),
            "E_SPACE_MISMATCH",
            "\"e\"",
        ),
        // A written embedding of the wrong dimension.
        (
            format!(
                "{NOTES}write W graph \"g\" {{ assert embedding \"e\" in Notes subject node \"n\" vector [1, 0, 0, 0] valid 0 until infinity; }}"
            ),
            "E_SPACE_MISMATCH",
            "[1, 0, 0, 0]",
        ),
        // One space name with two descriptors.
        (
            format!(
                "{NOTES}embedding_space Other id \"notes\" revision \"1\" encoder \"enc\" version \"v2\" preprocessing \"words/v1\" dimensions 3 metric cosine_distance;"
            ),
            "E_SPACE_MISMATCH",
            "Other",
        ),
        (
            format!("{use_g}nearest N from G {{ space Missing; vector [1]; k 1; }}"),
            "E_UNKNOWN_SPACE",
            "Missing",
        ),
        (
            "write W graph \"g\" { assert space Missing; }".to_owned(),
            "E_UNKNOWN_SPACE",
            "Missing",
        ),
        (
            format!(
                "{NOTES}value S 3;\n{use_g}nearest N from G {{ space Notes; vector value S; k 1; }}"
            ),
            "E_SCALAR_TYPE",
            "S",
        ),
    ] {
        assert_eq!(error(&source), (code.into(), span.into()), "{source}");
    }
}

#[test]
fn diagnostics_have_stable_codes_and_exact_spans() {
    let use_g = format!("{NOTES}use G graph \"g\"; ");
    for (source, code, span) in [
        (
            format!("{use_g}nearest N from G {{ space Notes; k 1; }}"),
            "E_NEAREST",
            "N",
        ),
        (
            format!("{use_g}nearest N from G {{ space Notes; vector [1, 0, 0]; k 0; }}"),
            "E_NEAREST",
            "0",
        ),
        (
            format!("{use_g}nearest N from G {{ space Notes; vector [1, 0, 0]; k 1001; }}"),
            "E_BUDGET",
            "1001",
        ),
        (
            format!("{use_g}nearest N from G {{ space Notes; vector [1, 0, 0]; like embedding \"e\"; k 1; }}"),
            "E_DUPLICATE",
            "like",
        ),
        (
            format!("{use_g}similar_candidates C from G {{ k 1; }}"),
            "E_NEAREST",
            "C",
        ),
        (
            format!("{use_g}similar_candidates C from G {{ space Notes; k 1; max_score -1; }}"),
            "E_NEAREST",
            "-1",
        ),
        (
            format!("{use_g}similar_candidates C from G {{ space Notes; k 1; limit 100001; }}"),
            "E_BUDGET",
            "100001",
        ),
        (
            format!("{use_g}nearest N from G {{ space Notes; vector [1, 0, 0]; k 1; hops 2; }}"),
            "E_PARSE",
            "hops",
        ),
        (
            format!("{use_g}lens L from G {{}} nearest N from L {{ space Notes; vector [1, 0, 0]; k 1; }}"),
            "E_NEAREST_SOURCE",
            "L",
        ),
        (
            format!("{use_g}nearest N from G {{ space Notes; vector [1, 0, 0]; k 1; candidates Missing; }}"),
            "E_UNKNOWN_GRAPH",
            "Missing",
        ),
        (
            format!("{use_g}changes D from G {{ since head; }} nearest N from G {{ space Notes; vector [1, 0, 0]; k 1; candidates D; }}"),
            "E_CHANGES_OPERAND",
            "D",
        ),
        (
            format!("{use_g}changes D from G {{ since head; }} expand E from G {{ seeds from D; hops 1; }}"),
            "E_CHANGES_OPERAND",
            "D",
        ),
        (
            format!("{use_g}expand E from G {{ hops 1; }}"),
            "E_EXPAND",
            "E",
        ),
        (
            format!("{use_g}nearest N from G {{ space Notes; vector [1, \"x\", 0]; k 1; }}"),
            "E_VECTOR_VALUE",
            "[1, \"x\", 0]",
        ),
        (
            "function F revision \"1\" (graph input) { write W graph \"g\" { retract node \"n\"; } return input; }".to_owned(),
            "E_FUNCTION_EFFECT",
            "W",
        ),
        (
            "function F revision \"1\" (graph input) { nearest N from input { k 1; } return N; }".to_owned(),
            "E_FUNCTION_EFFECT",
            "N",
        ),
        (
            "embedding_space S id \"s\" revision \"a@b\" encoder \"e\" version \"1\" preprocessing \"p\" dimensions 3 metric euclidean;".to_owned(),
            "E_SPACE",
            "S",
        ),
        (
            "embedding_space S id \"s\" revision \"1\" encoder \"e\" version \"1\" preprocessing \"p\" dimensions 3 metric dot;".to_owned(),
            "E_PARSE",
            "dot",
        ),
        (
            format!("{NOTES}write W graph \"g\" {{ correct space Notes; }}"),
            "E_CONFLICT",
            "space",
        ),
        (
            "write W graph \"g\" { assert edge \"e\" from \"a\" to \"b\" relation \"r\" valid 5 until 5; }".to_owned(),
            "E_INTERVAL",
            "assert edge \"e\" from \"a\" to \"b\" relation \"r\" valid 5 until 5;",
        ),
        (
            "write W graph \"g\" { retract space \"s\"; }".to_owned(),
            "E_PARSE",
            "space",
        ),
        (
            "write W graph \"g\" { upsert node \"n\"; }".to_owned(),
            "E_PARSE",
            "upsert",
        ),
        (
            "write W graph \"g\" {}".to_owned(),
            "E_PARSE",
            "}",
        ),
        (
            "write W graph \"g\" expected maybe { retract node \"n\"; }".to_owned(),
            "E_PARSE",
            "maybe",
        ),
        (
            "write W graph \"g\" { retract node \"n\"; } lens L from W {}".to_owned(),
            "E_UNKNOWN_GRAPH",
            "W",
        ),
    ] {
        assert_eq!(error(&source), (code.into(), span.into()), "{source}");
    }
}
