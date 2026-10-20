//! LA1 part 1 (weave-language#3): source syntax for the engine A3 operators
//! `as_of`, `expand` and `changes`, and `explain` over them. Golden
//! source-to-plan tests, round trips through the vendored 0.23 decoder,
//! oracle-fixture equality, core v1 classification and diagnostics.
use serde_json::{Value, json};
use std::path::Path;
use weave_contract::{Command, GraphExpression, VERSION, decode_program};
use weave_language::contract_profile::FeatureClass;
use weave_language::{compile, describe_contract};

fn plan(source: &str) -> Value {
    let program = compile(source).unwrap_or_else(|e| panic!("{source}: {e:?}"));
    let value = serde_json::to_value(&program).unwrap();
    // Every emitted plan decodes strictly as the vendored contract and re-encodes unchanged.
    let decoded = decode_program(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), value, "{source}");
    value
}

fn error(source: &str) -> (String, String) {
    let e = compile(source).expect_err(source);
    (e.code.clone(), source[e.start..e.end].to_owned())
}

fn query(graph: &str, predicate: Option<&str>, valid_at: Option<i64>) -> Value {
    json!({"graph_id": graph, "revision": null, "branch_id": "main", "predicate": predicate,
           "from": null, "to": null, "valid_at": valid_at, "include_metadata": false, "max_depth": 8})
}

#[test]
fn as_of_use_lowers_to_the_as_of_expression_with_valid_time_selection() {
    let source = r#"use Known graph "fleet" branch "main" as_of 1759000000000;
lens Located from Known { match relation "located_at"; at 1758000000000; }
lens All from Known {}"#;
    assert_eq!(
        plan(source),
        json!({"version": VERSION, "commands": [
            {"op": "bind", "name": "Located", "value": {"kind": "as_of", "unix_millis": 1759000000000_i64,
                "query": query("fleet", Some("located_at"), Some(1758000000000))}},
            {"op": "bind", "name": "All", "value": {"kind": "as_of", "unix_millis": 1759000000000_i64,
                "query": query("fleet", None, None)}}
        ]})
    );
    // A branch other than main, and the plain head read, are unchanged otherwise.
    let other = plan(r#"use B graph "fleet" branch "offline" as_of 7; lens L from B {}"#);
    assert_eq!(
        other["commands"][0]["value"]["query"]["branch_id"],
        "offline"
    );
    let head = plan(r#"use H graph "fleet"; lens L from H {}"#);
    assert_eq!(head["commands"][0]["value"]["kind"], "query");
}

#[test]
fn expand_lowers_every_clause_and_inherits_the_recorded_selection() {
    let source = r#"use Known graph "fleet" as_of 1759000000000;
expand Near from Known {
  seeds "unit-7", "unit-8";
  hops 2;
  direction both;
  match relation "installed_at", "runs";
  at 1760000000000;
  limit 500;
}
use Pinned graph "fleet" revision "fact:sha256:abc";
expand Defaults from Pinned { seeds "unit-7"; hops 1; }"#;
    assert_eq!(
        plan(source)["commands"],
        json!([
            {"op": "bind", "name": "Near", "value": {"kind": "expand", "graph_id": "fleet",
                "branch_id": "main", "as_of": 1759000000000_i64, "seeds": ["unit-7", "unit-8"], "hops": 2,
                "direction": "both", "predicates": ["installed_at", "runs"],
                "valid_at": 1760000000000_i64, "limit": 500}},
            {"op": "bind", "name": "Defaults", "value": {"kind": "expand", "graph_id": "fleet",
                "branch_id": "main", "revision": "fact:sha256:abc", "seeds": ["unit-7"], "hops": 1,
                "direction": "out", "limit": 10000}}
        ])
    );
    for (direction, wire) in [("out", "out"), ("in", "in"), ("both", "both")] {
        let p = plan(&format!(
            r#"use G graph "g"; expand E from G {{ seeds "a"; hops 1; direction {direction}; }}"#
        ));
        assert_eq!(p["commands"][0]["value"]["direction"], wire);
    }
}

#[test]
fn changes_lowers_every_bound_and_scope() {
    let source = r#"use Fleet graph "fleet" branch "main";
changes Window from Fleet { between as_of 10 and as_of 20; entities "UNIT-7", "UNIT-8"; match relation "runs"; limit 50; }
changes Feed from Fleet { since cursor "fact:sha256:00"; }
changes Back from Fleet { between head and cursor "fact:sha256:01"; }"#;
    assert_eq!(
        plan(source)["commands"],
        json!([
            {"op": "bind", "name": "Window", "value": {"kind": "changes", "graph_id": "fleet",
                "branch_id": "main", "from": {"kind": "as_of", "unix_millis": 10},
                "to": {"kind": "as_of", "unix_millis": 20}, "entities": ["UNIT-7", "UNIT-8"],
                "predicates": ["runs"], "limit": 50}},
            {"op": "bind", "name": "Feed", "value": {"kind": "changes", "graph_id": "fleet",
                "branch_id": "main", "from": {"kind": "cursor", "revision": "fact:sha256:00"},
                "to": {"kind": "head"}, "limit": 10000}},
            {"op": "bind", "name": "Back", "value": {"kind": "changes", "graph_id": "fleet",
                "branch_id": "main", "from": {"kind": "head"},
                "to": {"kind": "cursor", "revision": "fact:sha256:01"}, "limit": 10000}}
        ])
    );
}

#[test]
fn explain_and_graph_operators_compose_over_expand_and_as_of() {
    let source = r#"graph Fleet { node "a" entity "A" space "s"; node "b" entity "B" space "s";
  edge "ab" from "a" to "b" relation "feeds" valid 0 until infinity; }
use Known graph "Fleet" as_of 5;
expand Near from Fleet { seeds "a"; hops 1; }
expand Then from Known { seeds "a"; hops 1; }
explain Why from Near;
lens Feeds from Near { match relation "feeds"; }
union Both from Near with Then;
function Keep revision "1" (graph input) { lens Out from input {} return Out; }
apply Kept from Keep { graph input Then; }"#;
    let p = plan(source);
    let commands = p["commands"].as_array().unwrap();
    assert_eq!(commands[0]["op"], "commit");
    let value = |name: &str| {
        commands
            .iter()
            .find(|c| c["name"] == name)
            .unwrap_or_else(|| panic!("{name}"))["value"]
            .clone()
    };
    assert_eq!(value("Near")["kind"], "expand");
    assert_eq!(value("Near")["graph_id"], "Fleet");
    assert_eq!(value("Then")["as_of"], 5);
    assert_eq!(
        value("Why"),
        json!({"kind": "explain", "input": {"kind": "reference", "name": "Near"}})
    );
    assert_eq!(
        value("Feeds"),
        json!({"kind": "filter", "input": {"kind": "reference", "name": "Near"}, "predicate": "feeds", "valid_at": null})
    );
    assert_eq!(value("Both")["kind"], "union");
    // A pure function consumes the as-of expansion as an ordinary graph argument.
    assert!(
        commands
            .iter()
            .any(|c| c["value"] == json!({"kind": "reference", "name": "Then"}))
    );
    assert!(commands.iter().any(|c| c["name"] == "Kept"));
}

/// The fixture-mirroring examples compile to exactly the engine core v1
/// fixtures' expressions (scripts/check_traversal.py also runs them).
#[test]
fn examples_compile_to_the_engine_oracle_fixture_expressions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (fixture, source) in [
        ("expand-k-hop", "expand_k_hop"),
        ("expand-valid-time-as-of", "expand_valid_time_as_of"),
        ("changes-between-times", "changes_between_times"),
        ("as-of-partially-visible", "as_of_partially_visible"),
    ] {
        let text = std::fs::read_to_string(root.join(format!("examples/traversal/{source}.weave")))
            .unwrap();
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
        let compiled: Vec<GraphExpression> = program
            .commands
            .iter()
            .map(|c| match c {
                Command::Bind { value, .. } => value.clone(),
                other => panic!("{source}: unexpected command {other:?}"),
            })
            .collect();
        assert_eq!(
            compiled, expected,
            "{source} differs from fixture {fixture}"
        );
    }
}

#[test]
fn describe_contract_classifies_the_new_constructs_as_core_v1() {
    let description = describe_contract(
        r#"use Known graph "g" as_of 15; use G graph "g";
lens L from Known { at 3; }
expand E from Known { seeds "a"; hops 2; }
explain W from E;
changes C from G { since as_of 15; }"#,
    )
    .unwrap();
    let profile = &description.contract;
    assert!(profile.core_only);
    for (feature, location) in [
        ("as_of", "/program/commands/0/value"),
        ("expand", "/program/commands/1/value"),
        ("explain", "/program/commands/2/value"),
        ("changes", "/program/commands/3/value"),
    ] {
        let entry = profile
            .features
            .iter()
            .find(|f| f.feature == feature)
            .unwrap_or_else(|| panic!("{feature} not reported"));
        assert_eq!(entry.class, FeatureClass::CoreV1, "{feature}");
        assert_eq!(entry.locations, vec![location.to_owned()], "{feature}");
    }
    let since = |name: &str| {
        profile
            .features
            .iter()
            .find(|f| f.feature == name)
            .unwrap()
            .since
    };
    assert_eq!(
        (since("as_of"), since("expand"), since("changes")),
        ("0.23", "0.23", "0.23")
    );
}

#[test]
fn a_change_set_is_terminal_with_an_exact_span() {
    let prefix = r#"use G graph "g"; changes D from G { since head; } "#;
    for (rest, span) in [
        ("explain W from D;", "D"),
        ("lens L from D {}", "D"),
        ("union U from G with D;", "D"),
        ("join J from D to G relation \"r\";", "D"),
        ("expand E from D { seeds \"a\"; hops 1; }", "D"),
        (
            "function F revision \"1\" (graph input) { lens O from input {} return O; } apply A from F { graph input D; }",
            "input",
        ),
    ] {
        assert_eq!(
            error(&format!("{prefix}{rest}")),
            ("E_CHANGES_OPERAND".into(), span.into()),
            "{rest}"
        );
    }
}

#[test]
fn diagnostics_have_stable_codes_and_exact_spans() {
    let use_g = r#"use G graph "g"; "#;
    for (source, code, span) in [
        (
            r#"use G graph "g" revision "r" as_of 5;"#.to_owned(),
            "E_HISTORY_SELECTOR",
            "as_of",
        ),
        (
            r#"use G graph "g" as_of 5 revision "r";"#.to_owned(),
            "E_HISTORY_SELECTOR",
            "revision",
        ),
        (
            format!("{use_g}expand E from G {{ hops 1; }}"),
            "E_EXPAND",
            "E",
        ),
        (
            format!("{use_g}expand E from G {{ seeds \"a\"; }}"),
            "E_EXPAND",
            "E",
        ),
        (
            format!("{use_g}expand E from G {{ seeds \"a\"; hops 0; }}"),
            "E_EXPAND",
            "0",
        ),
        (
            format!("{use_g}expand E from G {{ seeds \"a\"; hops 17; }}"),
            "E_BUDGET",
            "17",
        ),
        (
            format!("{use_g}expand E from G {{ seeds \"a\"; hops 1; limit 200001; }}"),
            "E_BUDGET",
            "200001",
        ),
        (
            format!("{use_g}changes C from G {{ since head; limit 0; }}"),
            "E_BUDGET",
            "0",
        ),
        (
            format!("{use_g}expand E from G {{ seeds \"a\"; hops 1; hops 2; }}"),
            "E_DUPLICATE",
            "hops",
        ),
        (
            format!("{use_g}changes C from G {{ since head; between head and head; }}"),
            "E_DUPLICATE",
            "between",
        ),
        (
            format!("{use_g}expand E from G {{ seeds \"a\"; hops 1; entities \"A\"; }}"),
            "E_PARSE",
            "entities",
        ),
        (
            format!("{use_g}changes C from G {{ since head; hops 1; }}"),
            "E_PARSE",
            "hops",
        ),
        (
            format!("{use_g}expand E from G {{ seeds \"a\"; hops 1; direction up; }}"),
            "E_PARSE",
            "up",
        ),
        (
            format!("{use_g}changes C from G {{ since yesterday; }}"),
            "E_PARSE",
            "yesterday",
        ),
        (
            format!("{use_g}changes C from G {{ entities \"A\"; }}"),
            "E_CHANGES_BOUND",
            "}",
        ),
        (
            format!("{use_g}expand E from G {{ seeds \"\"; hops 1; }}"),
            "E_ID",
            "\"\"",
        ),
        (
            "expand E from Missing { seeds \"a\"; hops 1; }".to_owned(),
            "E_UNKNOWN_GRAPH",
            "Missing",
        ),
        (
            format!("{use_g}lens L from G {{ match relation \"r\"; }} expand E from L {{ seeds \"a\"; hops 1; }}"),
            "E_EXPAND_SOURCE",
            "L",
        ),
        (
            format!("{use_g}lens L from G {{}} expand E from L {{ seeds \"a\"; hops 1; }}"),
            "E_EXPAND_SOURCE",
            "L",
        ),
        (
            r#"use K graph "g" as_of 5; changes C from K { since head; }"#.to_owned(),
            "E_CHANGES_SOURCE",
            "K",
        ),
        (
            r#"use K graph "g" revision "r"; changes C from K { since head; }"#.to_owned(),
            "E_CHANGES_SOURCE",
            "K",
        ),
        (
            "function F revision \"1\" (graph input) { expand E from input { seeds \"a\"; hops 1; } return E; }".to_owned(),
            "E_FUNCTION_EFFECT",
            "E",
        ),
        (
            "function F revision \"1\" (graph input) { changes C from input { since head; } return input; }".to_owned(),
            "E_FUNCTION_EFFECT",
            "C",
        ),
        (
            format!("{use_g}nearest N from G {{ k 5; }}"),
            "E_NOT_YET_SUPPORTED",
            "nearest",
        ),
        (
            format!("{use_g}similar_candidates N from G {{}}"),
            "E_NOT_YET_SUPPORTED",
            "similar_candidates",
        ),
    ] {
        assert_eq!(
            error(&source),
            (code.into(), span.into()),
            "{source}"
        );
    }
    let many = (0..=1000)
        .map(|i| format!("\"n{i}\""))
        .collect::<Vec<_>>()
        .join(", ");
    assert_eq!(
        error(&format!(
            "{use_g}expand E from G {{ seeds {many}; hops 1; }}"
        )),
        ("E_BUDGET".into(), "seeds".into())
    );
}

#[test]
fn the_phase_a_question_two_example_fits_in_fifteen_lines() {
    let text = include_str!("../examples/traversal/fleet_questions.weave");
    assert!(text.lines().count() <= 15);
    let p = plan(text);
    let kinds: Vec<&str> = p["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["value"]["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["expand", "explain", "changes"]);
}
