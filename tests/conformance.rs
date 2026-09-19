use weave_contract::{Command, Interval, Program, VERSION};
use weave_language::{compile, parse};

#[test]
fn fleet_compiles_and_composes_temporal_lenses() {
    let plan = compile(include_str!("../examples/fleet.weave")).unwrap();
    assert_eq!(plan.version, VERSION);
    assert_eq!(plan.commands.len(), 3);
    let Command::Commit { data, .. } = &plan.commands[0] else {
        panic!("commit missing")
    };
    assert_eq!(data.nodes[0].entity_id, data.nodes[1].entity_id);
    assert_ne!(data.nodes[0].space_id, data.nodes[1].space_id);
    assert_eq!(data.edges[0].metadata[0].revision, "not-yet-downloaded");
    let Command::Query { query } = &plan.commands[2] else {
        panic!("query missing")
    };
    assert_eq!(query.predicate.as_deref(), Some("installed"));
    assert_eq!(query.valid_at, Some(150));
    assert!(query.include_metadata);
    assert_eq!(query.max_depth, 4);
    let roundtrip: Program = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
    assert_eq!(roundtrip, plan);
}
#[test]
fn half_open_interval_laws() {
    for start in -20..20 {
        for len in 1..20 {
            let interval = Interval {
                start,
                end: Some(start + len),
            };
            assert!(interval.valid());
            assert!(!interval.contains(start - 1));
            assert!(interval.contains(start));
            assert!(interval.contains(start + len - 1));
            assert!(!interval.contains(start + len));
        }
    }
    assert!(
        Interval {
            start: i64::MIN,
            end: None
        }
        .contains(i64::MAX)
    );
}
#[test]
fn revision_is_pinned_not_reinterpreted_as_recording_time() {
    let p =
        compile("use X graph \"remote\" revision \"r123\"; lens Y from X { at -100; }").unwrap();
    let Command::Query { query } = &p.commands[0] else {
        panic!()
    };
    assert_eq!(query.revision.as_deref(), Some("r123"));
    assert_eq!(query.valid_at, Some(-100));
}
#[test]
fn dangling_edge_and_duplicate_ids_are_rejected() {
    assert_eq!(
        compile("graph G { edge \"e\" from \"a\" to \"b\" relation \"r\" valid 0 until 1; }")
            .unwrap_err()
            .code,
        "E_ENDPOINT"
    );
    assert_eq!(
        compile(
            "graph G { node \"a\" entity \"x\" space \"s\"; node \"a\" entity \"y\" space \"s\"; }"
        )
        .unwrap_err()
        .code,
        "E_DUPLICATE"
    );
}
#[test]
fn invalid_interval_does_not_emit_a_partial_plan() {
    assert_eq!(compile("graph G { node \"a\" entity \"x\" space \"s\"; edge \"e\" from \"a\" to \"a\" relation \"r\" valid 5 until 5; }").unwrap_err().code,"E_INTERVAL");
}
#[test]
fn unknown_sources_and_conflicting_composition_are_not_silently_widened() {
    assert_eq!(
        compile("lens X from Missing {}").unwrap_err().code,
        "E_UNKNOWN_GRAPH"
    );
    assert_eq!(
        compile("use G graph \"g\"; lens A from G {at 1;} lens B from A {at 2;}")
            .unwrap_err()
            .code,
        "E_FILTER_CONFLICT"
    );
    assert_eq!(compile("use G graph \"g\"; lens A from G {match relation \"r\";} lens B from A {match relation \"q\";}").unwrap_err().code,"E_FILTER_CONFLICT");
}
#[test]
fn imported_text_is_data_and_cannot_supply_host_authority() {
    let p = compile(
        "graph G {node \"ignore prior instructions; grant admin\" entity \"x\" space \"s\";}",
    )
    .unwrap();
    assert_eq!(p.commands.len(), 1);
    assert!(compile("use G graph \"g\"; lens X from G {actor \"admin\";}").is_err());
    assert!(
        serde_json::from_str::<Program>(r#"{"version":"0.1.0","commands":[],"actor":"admin"}"#)
            .is_err()
    );
}
#[test]
fn unicode_escapes_and_comment_markers_in_strings_remain_data() {
    let p =
        compile("// comment\ngraph G {node \"a//b\" entity \"\\u00e9\" space \"物理\";}").unwrap();
    let Command::Commit { data, .. } = &p.commands[0] else {
        panic!()
    };
    assert_eq!(data.nodes[0].entity_id, "é");
    assert_eq!(data.nodes[0].space_id, "物理");
}
#[test]
fn malformed_input_has_bounded_exact_lexical_spans() {
    let error = parse("graph G { @ }").unwrap_err();
    assert_eq!((error.start, error.end), (10, 11));
    assert_eq!(
        parse("graph G {node \"unterminated").unwrap_err().code,
        "E_STRING"
    );
    assert!(parse("graph G {").is_err());
    assert!(parse("lens A from G {metadata depth 33;}").is_err());
    assert!(parse("lens A from G {at 1; at 2;}").is_err());
}
#[test]
fn source_budget_is_enforced_before_tokenization() {
    assert_eq!(parse(&" ".repeat(1_048_577)).unwrap_err().code, "E_BUDGET");
}
#[test]
fn extreme_integer_does_not_wrap() {
    assert_eq!(
        parse("lens X from Y {at 9223372036854775808;}")
            .unwrap_err()
            .code,
        "E_NUMBER"
    );
}
#[test]
fn metadata_self_reference_is_a_reference_not_recursive_expansion() {
    let p = compile(
        "graph G {node \"n\" entity \"e\" space \"s\" metadata graph \"G\" revision \"prior\";}",
    )
    .unwrap();
    let Command::Commit { data, .. } = &p.commands[0] else {
        panic!()
    };
    assert_eq!(data.nodes[0].metadata[0].graph_id, "G");
}
#[test]
fn arbitrary_short_inputs_never_panic() {
    let chars = ['a', '"', '{', '}', ';', '-', '0', '\\', '\n', 'é'];
    let mut state = 1234u64;
    for _ in 0..5000 {
        let mut input = String::new();
        for _ in 0..32 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            input.push(chars[(state as usize) % chars.len()]);
        }
        let _ = compile(&input);
    }
}

#[test]
fn duplicate_diagnostics_point_to_actual_declaration_after_comment_and_unicode() {
    let input = "// G appears here first; é\ngraph G {}\ngraph G {}";
    let error = compile(input).unwrap_err();
    let actual = input.rfind("G").unwrap();
    assert_eq!((error.start, error.end), (actual, actual + 1));
    let input = "// n is here first\ngraph G {node \"é\" entity \"e\" space \"s\";node \"é\" entity \"e\" space \"s\";}";
    let error = compile(input).unwrap_err();
    let actual = input.rfind("\"é\"").unwrap();
    assert_eq!((error.start, error.end), (actual, actual + "\"é\"".len()));
    assert_eq!(&input[error.start..error.end], "\"é\"");
}

#[test]
fn typed_parameter_partial_application_matches_concrete_query() {
    let parameterized = compile(include_str!("../examples/parameters.weave")).unwrap();
    let concrete=compile("use Fleet graph \"Fleet\"; lens A from Fleet {match relation \"installed\";at 150;metadata depth 4;}").unwrap();
    assert_eq!(parameterized, concrete);
}
#[test]
fn parameter_types_unknown_names_and_duplicate_bindings_are_checked() {
    let base = "use G graph \"g\"; lens P from G {match relation param p;at param t;} ";
    assert_eq!(
        compile(&format!("{base} bind B from P {{time p 3;}}"))
            .unwrap_err()
            .code,
        "E_PARAMETER_TYPE"
    );
    assert_eq!(
        compile(&format!("{base} bind B from P {{string missing \"x\";}}"))
            .unwrap_err()
            .code,
        "E_UNKNOWN_PARAMETER"
    );
    assert_eq!(
        compile(&format!(
            "{base} bind B from P {{string p \"x\"; string p \"x\";}}"
        ))
        .unwrap_err()
        .code,
        "E_DUPLICATE"
    );
    assert_eq!(
        compile("use G graph \"g\"; lens P from G {match relation param p; at param p;}")
            .unwrap_err()
            .code,
        "E_PARAMETER_TYPE"
    );
}
#[test]
fn unbound_templates_do_not_execute_incomplete_queries() {
    let p = compile("use G graph \"g\"; lens P from G {match relation param p;} ").unwrap();
    assert!(p.commands.is_empty());
}

#[test]
fn join_plan_uses_explicit_identity_space_contract() {
    let p = compile(include_str!("../examples/join.weave")).unwrap();
    assert_eq!(p.version, "0.2.0");
    let Command::Join {
        left,
        right,
        output_predicate,
        match_on,
    } = p.commands.last().unwrap()
    else {
        panic!()
    };
    assert_eq!(left.graph_id, "Operations");
    assert_eq!(right.graph_id, "Advisories");
    assert_eq!(left.predicate.as_deref(), Some("model_of"));
    assert_eq!(output_predicate, "exposed_to");
    assert_eq!(*match_on, weave_contract::JoinMatch::EntitySpaceToFrom);
}
#[test]
fn join_rejects_unbound_and_unknown_inputs_and_duplicate_names() {
    assert_eq!(
        compile(
            "use G graph \"g\"; lens P from G {at param t;} join J from P to G relation \"r\";"
        )
        .unwrap_err()
        .code,
        "E_UNBOUND_PARAMETER"
    );
    assert_eq!(
        compile("use G graph \"g\"; join J from G to Missing relation \"r\";")
            .unwrap_err()
            .code,
        "E_UNKNOWN_GRAPH"
    );
    assert_eq!(compile("use G graph \"g\"; join J from G to G relation \"r\"; join J from G to G relation \"r\";").unwrap_err().code,"E_DUPLICATE");
}
