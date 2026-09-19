use weave_contract::{Command, GraphExpression, Interval, Program, VERSION};
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
    let Command::Bind {
        value: GraphExpression::Query { query },
        ..
    } = &plan.commands[1]
    else {
        panic!("stored query missing")
    };
    assert!(query.include_metadata);
    assert_eq!(query.max_depth, 4);
    let Command::Bind {
        value:
            GraphExpression::Filter {
                input,
                predicate,
                valid_at,
            },
        ..
    } = &plan.commands[2]
    else {
        panic!("composed filter missing")
    };
    assert!(matches!(input.as_ref(),GraphExpression::Reference{name} if name=="Installed"));
    assert_eq!(predicate.as_deref(), Some("installed"));
    assert_eq!(*valid_at, Some(150));
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
    let Command::Bind {
        value: GraphExpression::Query { query },
        ..
    } = &p.commands[0]
    else {
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
    let Command::Bind { value: a, .. } = &parameterized.commands[0] else {
        panic!()
    };
    let Command::Bind { value: b, .. } = &concrete.commands[0] else {
        panic!()
    };
    assert_eq!(a, b);
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
    assert_eq!(p.version, weave_contract::VERSION);
    let Command::Bind {
        value:
            GraphExpression::Join {
                left,
                right,
                output_predicate,
                match_on,
            },
        ..
    } = p.commands.last().unwrap()
    else {
        panic!()
    };
    let GraphExpression::Filter { input, .. } = left.as_ref() else {
        panic!()
    };
    assert!(matches!(input.as_ref(),GraphExpression::Reference{name} if name=="ModelOf"));
    let GraphExpression::Filter { input, .. } = right.as_ref() else {
        panic!()
    };
    assert!(matches!(input.as_ref(),GraphExpression::Reference{name} if name=="AffectedBy"));
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

#[test]
fn derived_graph_values_feed_parameterized_lenses_and_further_joins_without_commits() {
    let p = compile(include_str!("../examples/composed.weave")).unwrap();
    assert_eq!(
        p.commands
            .iter()
            .filter(|c| matches!(c, Command::Commit { .. }))
            .count(),
        3
    );
    let bindings: Vec<_> = p
        .commands
        .iter()
        .filter_map(|c| {
            if let Command::Bind { name, value } = c {
                Some((name.as_str(), value))
            } else {
                None
            }
        })
        .collect();
    assert!(bindings.iter().any(|(n, _)| *n == "Exposure"));
    assert!(!bindings.iter().any(|(n, _)| *n == "Window"));
    let value = bindings.iter().find(|(n, _)| *n == "InWindow").unwrap().1;
    let GraphExpression::Filter {
        input, valid_at, ..
    } = value
    else {
        panic!()
    };
    assert_eq!(*valid_at, Some(175));
    assert!(matches!(input.as_ref(),GraphExpression::Reference{name} if name=="Exposure"));
    assert!(
        bindings
            .iter()
            .any(|(n, v)| *n == "Action" && matches!(v, GraphExpression::Join { .. }))
    );
    let (_, GraphExpression::Filter { input, .. }) = bindings.last().unwrap() else {
        panic!()
    };
    assert!(matches!(input.as_ref(),GraphExpression::Reference{name} if name=="Action"));
}
#[test]
fn materialized_graph_metadata_expansion_is_explicitly_unsupported() {
    assert_eq!(
        compile("use G graph \"g\";lens A from G {} lens B from A {metadata depth 2;}")
            .unwrap_err()
            .code,
        "E_METADATA_VALUE"
    );
}
#[test]
fn negative_claims_and_scalar_metadata_preserve_existing_protocol_fields() {
    let p=compile("graph G {node \"a\" entity \"e\" space \"s\" property \"name\" \"\" property \"active\" true;edge \"e\" from \"a\" to \"a\" relation \"r\" polarity negative valid 0 until infinity property \"weight\" -3 property \"source\" null;}").unwrap();
    let Command::Commit { data, .. } = &p.commands[0] else {
        panic!()
    };
    assert_eq!(data.edges[0].polarity, weave_contract::Polarity::Negative);
    assert_eq!(data.nodes[0].properties["name"], serde_json::json!(""));
    assert_eq!(data.nodes[0].properties["active"], serde_json::json!(true));
    assert_eq!(data.edges[0].properties["weight"], serde_json::json!(-3));
    assert!(data.edges[0].properties["source"].is_null());
    assert_eq!(
        compile("graph G {node \"n\" entity \"e\" space \"s\" property \"k\" 1 property \"k\" 2;}")
            .unwrap_err()
            .code,
        "E_DUPLICATE"
    );
    assert_eq!(
        compile("graph G {node \"n\" entity \"e\" space \"s\" property \"k\" NaN;}")
            .unwrap_err()
            .code,
        "E_PROPERTY_TYPE"
    );
}

#[test]
fn typed_schema_graph_lowering_and_discovery_agree() {
    let source = include_str!("../examples/schema.weave");
    let plan = compile(source).unwrap();
    let schemas = weave_language::describe(source).unwrap();
    assert_eq!(schemas.len(), 1);
    assert_eq!(schemas[0].id, "Infrastructure");
    assert_eq!(schemas[0].revision, "1");
    let Command::Commit { data, .. } = &plan.commands[0] else {
        panic!()
    };
    assert_eq!(data.schema.as_ref(), Some(&schemas[0]));
    assert_eq!(data.nodes[0].type_id.as_deref(), Some("Device"));
    assert_eq!(data.edges[0].type_id.as_deref(), Some("Connected"));
    assert!(weave_contract::validate_schema_graph(data).is_empty());
}
#[test]
fn schema_required_property_type_extra_fields_and_missing_type_are_rejected() {
    let source = include_str!("../examples/schema.weave");
    for (modified, code) in [
        (
            source.replace("property \"label\" \"Device 17\"", ""),
            "E_SCHEMA_REQUIRED",
        ),
        (
            source.replace("property \"label\" \"Device 17\"", "property \"label\" 17"),
            "E_SCHEMA_PROPERTY_TYPE",
        ),
        (
            source.replace(
                "property \"active\" true",
                "property \"active\" true property \"secret\" true",
            ),
            "E_SCHEMA_PROPERTY",
        ),
        (
            source.replace("\"device\" type Device", "\"device\""),
            "E_SCHEMA_NODE_TYPE",
        ),
    ] {
        assert_eq!(compile(&modified).unwrap_err().code, code);
    }
}
#[test]
fn schema_wrong_endpoint_and_space_are_compile_time_errors() {
    let source = include_str!("../examples/schema.weave");
    assert_eq!(
        compile(&source.replace("to \"gateway\" relation", "to \"device\" relation"))
            .unwrap_err()
            .code,
        "E_SCHEMA_ENDPOINT_TYPE"
    );
    assert_eq!(
        compile(&source.replace(
            "entity \"device-17\" space \"operations\"",
            "entity \"device-17\" space \"physical\""
        ))
        .unwrap_err()
        .code,
        "E_SCHEMA_SPACE"
    );
}
#[test]
fn schema_cross_space_edges_require_explicit_permission_in_schema() {
    let source = "schema S revision \"1\" {node A {} node B {} edge R from A to B {}} graph G schema S {node \"a\" type A entity \"a\" space \"one\";node \"b\" type B entity \"b\" space \"two\";edge \"e\" type R from \"a\" to \"b\" relation \"r\" valid 0 until 1;}";
    assert_eq!(compile(source).unwrap_err().code, "E_SCHEMA_CROSS_SPACE");
    assert!(
        compile(&source.replace("edge R from A to B {}", "edge R from A to B cross_space {}"))
            .is_ok()
    );
}
#[test]
fn unknown_schema_and_invalid_declared_endpoint_types_are_rejected() {
    assert_eq!(
        compile("graph G schema Missing {}").unwrap_err().code,
        "E_UNKNOWN_SCHEMA"
    );
    assert_eq!(
        compile("schema S revision \"1\" {node A{} edge R from A to Missing{}}")
            .unwrap_err()
            .code,
        "E_SCHEMA_ENDPOINT_TYPE"
    );
    assert_eq!(
        compile("graph G {node \"a\" type A entity \"e\" space \"s\";}")
            .unwrap_err()
            .code,
        "E_SCHEMA_MISSING"
    );
}
#[test]
fn schema_open_properties_are_explicit_and_duplicate_schema_fields_fail() {
    let source = "schema S revision \"1\" {node A {open;}} graph G schema S {node \"a\" type A entity \"e\" space \"s\" property \"extra\" true;}";
    assert!(compile(source).is_ok());
    assert_eq!(compile("schema S revision \"1\" {node A {property \"k\" string optional;property \"k\" string required;}}").unwrap_err().code,"E_DUPLICATE");
    assert_eq!(
        compile("schema S revision \"1\" {} schema S revision \"2\" {}")
            .unwrap_err()
            .code,
        "E_DUPLICATE"
    );
}

#[test]
fn named_metadata_and_cycle_compile_to_one_atomic_batch_then_graph_values() {
    let plan = compile(include_str!("../examples/metadata_cycle.weave")).unwrap();
    let Command::CommitBatch { batch_id, commits } = &plan.commands[0] else {
        panic!()
    };
    assert_eq!(batch_id, "boot");
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].data.attachments[0].key, "evidence");
    let weave_contract::MetadataValue::Graph { reference } = &commits[1].data.attachments[0].value
    else {
        panic!()
    };
    assert_eq!(reference.revision, "logical:boot:Operations");
    let Command::Bind {
        value: GraphExpression::Metadata { host, key, .. },
        ..
    } = &plan.commands[2]
    else {
        panic!()
    };
    assert_eq!(
        *host,
        weave_contract::MetadataHost::Edge {
            id: "connection".into()
        }
    );
    assert_eq!(key, "evidence");
}
#[test]
fn invalid_attachment_hosts_intervals_and_unbound_sources_fail() {
    assert_eq!(compile("graph G {attachment \"a\" on edge \"missing\" key \"proof\" graph \"P\" revision \"r\" valid 0 until 1;}").unwrap_err().code,"E_ATTACHMENT_HOST");
    assert_eq!(compile("graph G {attachment \"a\" on graph key \"proof\" graph \"P\" revision \"r\" valid 1 until 1;}").unwrap_err().code,"E_INTERVAL");
    assert_eq!(compile("use G graph \"g\";lens L from G {at param t;} metadata M from L on graph key \"proof\";").unwrap_err().code,"E_UNBOUND_PARAMETER");
}
#[test]
fn transactions_are_bounded_declaration_batches_not_nested_effect_programs() {
    assert!(compile("transaction B {}").is_err());
    assert!(compile("transaction B {transaction C {graph G {}}}").is_err());
    assert!(compile("transaction B {use G graph \"g\";}").is_err());
    assert!(compile("}").is_err());
    assert_eq!(
        compile("transaction B {graph G{}} transaction B {graph H{}}")
            .unwrap_err()
            .code,
        "E_DUPLICATE"
    );
    let source = format!("transaction {} {{graph G{{}}}}", "x".repeat(65));
    assert!(compile(&source).is_err());
}

#[test]
fn known_mixed_typed_untyped_join_is_rejected_without_dropping_types() {
    let source = format!(
        "{} graph Untyped {{}} join Invalid from Network to Untyped relation \"r\";",
        include_str!("../examples/schema.weave")
    );
    assert_eq!(compile(&source).unwrap_err().code, "E_SCHEMA_JOIN");
    assert!(compile(include_str!("../examples/typed_join.weave")).is_ok());
}

#[test]
fn graph_algebra_is_reusable_and_lowers_to_explicit_v05_expressions() {
    let plan = compile(include_str!("../examples/algebra.weave")).unwrap();
    assert_eq!(plan.version, weave_contract::VERSION);
    assert!(
        matches!(&plan.commands[2],Command::Bind{value:GraphExpression::Project{edge_ids,..},..} if edge_ids==&["claim"])
    );
    assert!(matches!(
        &plan.commands[4],
        Command::Bind {
            value: GraphExpression::Diff { .. },
            ..
        }
    ));
    assert!(matches!(
        &plan.commands[5],
        Command::Bind {
            value: GraphExpression::Union { .. },
            ..
        }
    ));
    let Command::Bind {
        value: GraphExpression::Support {
            from, to, valid_at, ..
        },
        ..
    } = &plan.commands[8]
    else {
        panic!()
    };
    assert_eq!(from.entity_id, "device-17");
    assert_eq!(to.space_id, "knowledge");
    assert_eq!(*valid_at, 15);
}
#[test]
fn algebra_unknown_and_unbound_inputs_have_exact_source_diagnostics() {
    let source =
        "// Missing is mentioned here first.\nuse A graph \"a\";union U from A with Missing;";
    let d = compile(source).unwrap_err();
    assert_eq!(d.code, "E_UNKNOWN_GRAPH");
    assert_eq!(&source[d.start..d.end], "Missing");
    assert_eq!(d.start, source.rfind("Missing").unwrap());
    assert_eq!(
        compile("use A graph \"a\";lens T from A {at param t;}project P from T {}")
            .unwrap_err()
            .code,
        "E_UNBOUND_PARAMETER"
    );
    assert!(compile("use A graph \"a\";support S from A relation \"p\" from entity \"a\" space \"s\" to entity \"b\" space \"s\";").is_err());
}
#[test]
fn algebra_cannot_silently_drop_known_schema_meaning() {
    let source = format!(
        "{} graph U{{}} union Invalid from Network with U;",
        include_str!("../examples/schema.weave")
    );
    assert_eq!(compile(&source).unwrap_err().code, "E_SCHEMA_ALGEBRA");
    let source = "graph G{} support S from G relation \"p\" from entity \"a\" space \"s\" to entity \"b\" space \"s\" at 0; union Invalid from S with G;";
    assert_eq!(compile(source).unwrap_err().code, "E_SCHEMA_ALGEBRA");
}

#[test]
fn graph_functions_specialize_partial_and_higher_order_values_without_hidden_commits() {
    let p = compile(include_str!("../examples/functions.weave")).unwrap();
    assert_eq!(
        p.commands
            .iter()
            .filter(|c| matches!(c, Command::Commit { .. }))
            .count(),
        1
    );
    assert!(
        p.commands
            .iter()
            .any(|c| matches!(c,Command::Bind{name,..}if name=="HigherOrder"))
    );
    assert!(
        !p.commands
            .iter()
            .any(|c| matches!(c,Command::Bind{name,..}if name=="AtFifteen"||name=="Specialized"))
    );
    assert!(
        p.commands
            .iter()
            .all(|c| !matches!(c, Command::CommitBatch { .. }))
    );
}
#[test]
fn graph_functions_typecheck_bodies_and_reject_hidden_reads_or_effects() {
    assert_eq!(
        compile("function F revision \"1\" (time instant) {lens R from instant {} return R;}")
            .unwrap_err()
            .code,
        "E_FUNCTION_SCOPE"
    );
    assert_eq!(compile("use Hidden graph \"hidden\"; function F revision \"1\" (graph input) {lens R from Hidden {} return R;}").unwrap_err().code,"E_FUNCTION_SCOPE");
    assert_eq!(compile("function F revision \"1\" (graph input, time t) {lens R from input {match relation param t;} return R;}").unwrap_err().code,"E_PARAMETER_TYPE");
    assert!(compile("function F revision \"1\" (graph input) {graph G{} return G;}").is_err());
    assert_eq!(
        compile("function F revision \"1\" (function transform) {return transform;}")
            .unwrap_err()
            .code,
        "E_FUNCTION_RETURN"
    );
    assert_eq!(
        compile(
            "function F revision \"1\" (graph input) {apply R from F {graph input input;}return R;}"
        )
        .unwrap_err()
        .code,
        "E_FUNCTION_SCOPE"
    );
}
#[test]
fn graph_function_argument_kinds_arity_and_rebinding_are_checked() {
    let f = "function F revision \"1\" (graph input, time t) {lens R from input {at param t;}return R;} graph G{} ";
    assert_eq!(
        compile(&format!("{f} apply X from F {{string t \"bad\";}}"))
            .unwrap_err()
            .code,
        "E_PARAMETER_TYPE"
    );
    assert_eq!(
        compile(&format!("{f} apply X from F {{time t 1;time t 2;}}"))
            .unwrap_err()
            .code,
        "E_DUPLICATE"
    );
    assert_eq!(
        compile(&format!("{f} apply X from F {{time absent 1;}}"))
            .unwrap_err()
            .code,
        "E_UNKNOWN_PARAMETER"
    );
    assert_eq!(
        compile(&format!(
            "{f} lens T from G {{at param t;}}apply X from F {{graph input T;}}"
        ))
        .unwrap_err()
        .code,
        "E_UNBOUND_PARAMETER"
    );
    assert_eq!(compile(&format!("{f} function Map revision \"1\" (graph input,function transform){{apply R from transform{{graph input input;}}return R;}}apply X from Map{{function transform F;}}")).unwrap_err().code,"E_FUNCTION_SIGNATURE");
}
#[test]
fn graph_capture_is_immutable_and_names_are_hygienic() {
    let s = "function F revision \"1\" (graph input,time t){lens R from input{at param t;}return R;}graph G{}apply Captured from F{graph input G;}graph Later{}apply Result from Captured{time t 5;}graph __weave_function_1_capture{}";
    let p = compile(s).unwrap();
    assert!(
        matches!(&p.commands[1],Command::Bind{value:GraphExpression::Query{query},..}if query.graph_id=="G")
    );
    assert!(matches!(&p.commands[2],Command::Commit{graph_id,..}if graph_id=="Later"));
    assert_eq!(p.commands.iter().filter(|c|matches!(c,Command::Bind{value:GraphExpression::Query{query},..}if query.graph_id=="G")).count(),1);
}
#[test]
fn function_expansion_has_a_deterministic_budget() {
    let mut s = String::from("function F0 revision \"1\" (graph input){return input;}");
    for i in 1..18 {
        s.push_str(&format!("function F{i} revision \"1\" (graph input){{apply A from F{}{{graph input input;}}apply B from F{}{{graph input A;}}return B;}}",i-1,i-1));
    }
    s.push_str("graph G{}apply TooMuch from F17{graph input G;}");
    assert_eq!(compile(&s).unwrap_err().code, "E_EXPANSION_BUDGET");
}

#[test]
fn explicit_relations_and_attributed_claims_have_distinct_ids_and_properties() {
    let plan = compile(include_str!("../examples/assertions.weave")).unwrap();
    let Command::Commit { data, .. } = &plan.commands[0] else {
        panic!()
    };
    assert_eq!(data.profile, weave_contract::GraphProfile::Explicit);
    assert_eq!(data.structural_edges.len(), 1);
    assert!(data.assertions.is_empty());
    assert!(data.edges.is_empty());
    let Command::Commit { data, .. } = &plan.commands[3] else {
        panic!()
    };
    assert_eq!(data.structural_edges[0].properties["category"], "advisory");
    assert_eq!(data.assertions.len(), 3);
    assert_eq!(data.assertions[0].id, "source-positive");
    assert_eq!(data.assertions[0].edge_id, "affected-relation");
    assert_eq!(data.assertions[0].properties["method"], "inspection");
    assert_eq!(data.assertions[2].context.as_ref().unwrap().revision, "r1");
    assert_eq!(
        data.assertions[1].polarity,
        weave_contract::Polarity::Negative
    );
}
#[test]
fn explicit_claims_reject_ambiguous_ids_dangling_relations_and_profile_mixing() {
    let prefix = "graph G explicit {node \"a\" entity \"a\" space \"s\";relation \"r\" from \"a\" to \"a\" predicate \"p\";";
    assert_eq!(
        compile(&format!(
            "{prefix}claim \"c\" on \"missing\" source \"s\" polarity positive valid 0 until 1;}}"
        ))
        .unwrap_err()
        .code,
        "E_ASSERTION_EDGE"
    );
    assert_eq!(
        compile(&format!(
            "{prefix}claim \"r\" on \"r\" source \"s\" polarity positive valid 0 until 1;}}"
        ))
        .unwrap_err()
        .code,
        "E_DUPLICATE"
    );
    assert_eq!(
        compile(&format!(
            "{prefix}edge \"e\" from \"a\" to \"a\" relation \"p\" valid 0 until 1;}}"
        ))
        .unwrap_err()
        .code,
        "E_ASSERTION_PROFILE"
    );
    assert_eq!(compile("graph G {node \"a\" entity \"a\" space \"s\";relation \"r\" from \"a\" to \"a\" predicate \"p\";}").unwrap_err().code,"E_ASSERTION_PROFILE");
    assert_eq!(
        compile(&format!(
            "{prefix}claim \"c\" on \"r\" source \"\" polarity positive valid 0 until 1;}}"
        ))
        .unwrap_err()
        .code,
        "E_ID"
    );
}
#[test]
fn typed_structural_relations_are_validated_without_inventing_claims() {
    let s = "schema S revision \"1\"{node N{}edge E from N to N{property \"label\" string required;}}graph G explicit schema S{node \"n\" type N entity \"n\" space \"s\";relation \"e\" type E from \"n\" to \"n\" predicate \"p\" property \"label\" \"edge\";}";
    assert!(compile(s).is_ok());
    assert_eq!(
        compile(&s.replace("property \"label\" \"edge\"", "property \"label\" 9"))
            .unwrap_err()
            .code,
        "E_SCHEMA_PROPERTY_TYPE"
    );
}

#[test]
fn function_source_identity_ignores_locations_and_tracks_revision_and_parameters() {
    let source = include_str!("../examples/functions.weave");
    let shifted = format!("// extra comment\n\n{}", source.replace("  ", "    "));
    let first = compile(source).unwrap();
    let second = compile(&shifted).unwrap();
    assert_eq!(first.source_revisions, second.source_revisions);
    assert_eq!(first.source_revisions.len(), 2);
    assert_eq!(
        weave_language::fingerprint(source).unwrap(),
        weave_language::fingerprint(&shifted).unwrap()
    );
    assert_ne!(
        weave_language::fingerprint(source).unwrap(),
        weave_language::fingerprint(&source.replacen("revision \"1\"", "revision \"2\"", 1))
            .unwrap()
    );
    assert_ne!(
        weave_language::fingerprint(source).unwrap(),
        weave_language::fingerprint(&source.replacen("time instant 15", "time instant 16", 1))
            .unwrap()
    );
}

#[test]
fn named_attachments_can_target_individual_source_assertions() {
    let source = "graph G explicit {node \"n\" entity \"n\" space \"s\";relation \"r\" from \"n\" to \"n\" predicate \"p\";claim \"c\" on \"r\" source \"inspection\" polarity positive valid 0 until 10;attachment \"a\" on assertion \"c\" key \"proof\" graph \"Proof\" revision \"r1\" valid 0 until 10;}";
    let p = compile(source).unwrap();
    let Command::Commit { data, .. } = &p.commands[0] else {
        panic!()
    };
    assert_eq!(
        data.attachments[0].host,
        weave_contract::MetadataHost::Assertion { id: "c".into() }
    );
    assert_eq!(
        compile(&source.replace("on assertion \"c\"", "on assertion \"missing\""))
            .unwrap_err()
            .code,
        "E_ATTACHMENT_HOST"
    );
}

#[test]
fn finite_rules_compile_through_graph_functions_and_preserve_source_identity() {
    let source = include_str!("../examples/rules.weave");
    let plan = compile(source).unwrap();
    assert!(
        plan.source_revisions
            .iter()
            .any(|s| s.name == "Reach" && s.revision == "1")
    );
    let reason = plan
        .commands
        .iter()
        .find_map(|c| match c {
            Command::Bind {
                name,
                value: GraphExpression::Reason { rules, .. },
            } if name == "Closure" => Some(rules),
            _ => None,
        })
        .unwrap();
    assert_eq!(reason.rules.len(), 2);
    assert_eq!(
        plan.source_revisions
            .iter()
            .find(|s| s.name == "Reach")
            .unwrap()
            .digest,
        weave_contract::identity::source_fingerprint(reason).unwrap()
    );
    assert_eq!(
        weave_language::fingerprint(source).unwrap(),
        weave_language::fingerprint(&format!("// shifted locations\n{source}")).unwrap()
    );
    assert_ne!(
        weave_language::fingerprint(source).unwrap(),
        weave_language::fingerprint(&source.replace("rule Seed", "rule First")).unwrap()
    );
}
#[test]
fn unsafe_rule_head_reports_exact_token_and_unbound_modules_fail() {
    let source = "// missing\nrules R revision \"1\" { rule A { when \"p\"(x,y); yield \"q\"(x,missing); } }";
    let error = compile(source).unwrap_err();
    assert_eq!(error.code, "E_RULE_RANGE");
    assert_eq!(&source[error.start..error.end], "missing");
    assert_eq!(error.start, source.rfind("missing").unwrap());
    assert!(compile("use G graph \"G\"; reason X from G using Missing;").is_err());
    assert!(
        compile(
            "rules R revision \"1\" {rule A {when \"p\"(x,y);yield \"q\"(x,y);}} lens X from R {}"
        )
        .is_err()
    );
}
#[test]
fn negative_rule_atoms_are_explicit_evidence_and_constants_are_preserved() {
    let p=compile("use G graph \"G\"; rules R revision \"1\" {rule A {when negative \"p\"(x,\"fixed\");yield negative \"q\"(x,\"fixed\"); cross_space;}} reason X from G using R;").unwrap();
    let Command::Bind {
        value: GraphExpression::Reason { rules, .. },
        ..
    } = &p.commands[0]
    else {
        panic!("reason expected")
    };
    assert_eq!(
        rules.rules[0].body[0].polarity,
        weave_contract::Polarity::Negative
    );
    assert!(matches!(&rules.rules[0].head.to,weave_contract::RuleTerm::Node{id} if id == "fixed"));
    assert!(rules.rules[0].allow_cross_space);
}

#[test]
fn exact_context_selection_and_attachment_qualifiers_survive_lowering() {
    let source = include_str!("../examples/contexts.weave");
    let plan = compile(source).unwrap();
    assert!(plan.commands.iter().any(|c| matches!(c,Command::Bind {name,value:GraphExpression::Context {selection:weave_contract::ContextSelection::Default,..}} if name=="Default")));
    assert!(plan.commands.iter().any(|c| matches!(c,Command::Bind {name,value:GraphExpression::Context {selection:weave_contract::ContextSelection::Pinned {reference},..}} if name=="Empty" && reference.revision=="not-selected")));
    let Command::CommitBatch { commits, .. } = &plan.commands[0] else {
        panic!("batch")
    };
    let claims = commits.iter().find(|c| c.graph_id == "Claims").unwrap();
    assert_eq!(
        claims.data.attachments[0]
            .context
            .as_ref()
            .unwrap()
            .revision,
        "logical:worlds:World"
    );
    assert_eq!(
        weave_language::fingerprint(source).unwrap(),
        weave_language::fingerprint(&format!("// locations\n{source}")).unwrap()
    );
}
#[test]
fn context_selectors_require_exact_bounded_revisions() {
    assert!(compile("use G graph \"G\"; context C from G graph \"World\";").is_err());
    assert_eq!(
        compile("use G graph \"G\"; context C from G graph \"World\" revision \"\";")
            .unwrap_err()
            .code,
        "E_ID"
    );
    assert!(compile("context C from Unknown default;").is_err());
    assert!(
        compile("use G graph \"G\"; lens L from G {at param t;} context C from L default;")
            .is_err()
    );
}

#[test]
fn geometry_and_explanations_lower_to_reusable_authorized_graph_operands() {
    let source = include_str!("../examples/geometry.weave");
    let plan = compile(source).unwrap();
    assert!(plan.commands.iter().any(|c|matches!(c,Command::Bind {name,value:GraphExpression::Geometry {operation:weave_contract::GeometryOperation::Distance {left,right},valid_at:7}} if name=="Range" && left.assertion_id=="coordinate-a" && right.assertion_id=="coordinate-b")));
    assert!(plan.commands.iter().any(
        |c| matches!(c,Command::Bind {name,value:GraphExpression::Explain {..}} if name=="Proof")
    ));
    assert_eq!(
        weave_language::fingerprint(source).unwrap(),
        weave_language::fingerprint(&format!("// shifted\n{source}")).unwrap()
    );
    let Command::Commit { data, .. } = &plan.commands[0] else {
        panic!("source commit")
    };
    assert_eq!(
        data.assertions[0].properties["weave.geometry"]["values"],
        serde_json::json!([0.0, 0.0, 0.0])
    );
}
#[test]
fn finite_float_literals_and_structured_properties_keep_strict_time_types() {
    let source = "schema S revision \"1\" {node N {property \"x\" float required;}} graph G schema S {node \"n\" type N entity \"N\" space \"s\" property \"x\" 1.25e-2;}";
    let plan = compile(source).unwrap();
    let Command::Commit { data, .. } = &plan.commands[0] else {
        panic!("commit")
    };
    assert_eq!(data.nodes[0].properties["x"], serde_json::json!(0.0125));
    assert!(compile(&source.replace("1.25e-2", "1e400")).is_err());
    assert!(compile(&source.replace("1.25e-2", "NaN")).is_err());
    assert!(compile("use G graph \"G\"; lens L from G {at 1.0;}").is_err());
    assert!(
        compile(
            "use G graph \"G\"; distance D from G assertion \"a\" to G assertion \"b\" at 1e0;"
        )
        .is_err()
    );
    assert!(compile(&source.replace("float required", "integer required")).is_err());
}
#[test]
fn structured_literals_reject_duplicate_keys_and_bound_nesting() {
    let source = "graph G {node \"n\" entity \"N\" space \"s\" property \"data\" {\"span\": [1.5, true, null, {\"x\":\"ok\"}]};}";
    let plan = compile(source).unwrap();
    let Command::Commit { data, .. } = &plan.commands[0] else {
        panic!("commit")
    };
    assert_eq!(data.nodes[0].properties["data"]["span"][3]["x"], "ok");
    let duplicate = source.replace("{\"x\":\"ok\"}", "{\"x\":1,\"x\":2}");
    let error = compile(&duplicate).unwrap_err();
    assert_eq!(error.code, "E_DUPLICATE");
    assert_eq!(&duplicate[error.start..error.end], "\"x\"");
    let deep = format!(
        "graph G {{node \"n\" entity \"N\" space \"s\" property \"data\" {}0{};}}",
        "[".repeat(40),
        "]".repeat(40)
    );
    assert_eq!(compile(&deep).unwrap_err().code, "E_BUDGET");
    assert_ne!(
        weave_language::fingerprint(source).unwrap(),
        weave_language::fingerprint(&source.replace("\"ok\"", "\"changed\"")).unwrap()
    );
}
#[test]
fn geometry_axes_and_graph_bindings_have_static_diagnostics() {
    let source = "use G graph \"G\"; project_axes P from G assertion \"e\" axes (0,1,1) revision \"r\" at 1;";
    let error = compile(source).unwrap_err();
    assert_eq!(error.code, "E_GEOMETRY_AXIS");
    assert_eq!(&source[error.start..error.end], "1");
    assert!(compile(&source.replace("(0,1,1)", "(0,-1,2)")).is_err());
    assert!(
        compile(
            "use G graph \"G\"; distance D from G assertion \"a\" to Missing assertion \"b\" at 1;"
        )
        .is_err()
    );
    assert!(compile("use G graph \"G\"; lens T from G {at param t;} transform D from G assertion \"a\" using T assertion \"b\" at 1;").is_err());
}

#[test]
fn declared_counterparts_lower_to_reusable_typed_graph_expression() {
    let plan = compile(include_str!("../examples/counterparts.weave")).unwrap();
    let Command::Bind {
        value: GraphExpression::Counterparts { input, selection },
        ..
    } = plan
        .commands
        .iter()
        .find(|c| matches!(c, Command::Bind { name, .. } if name == "Bridge"))
        .unwrap()
    else {
        panic!("counterpart expression missing")
    };
    assert!(matches!(input.as_ref(), GraphExpression::Reference { name } if name == "Evidence"));
    assert_eq!(selection.predicate, "counterpart");
    assert_eq!(selection.entity_id, "device-17");
    assert_eq!(selection.valid_at, 7);
    assert_ne!(selection.from_space_id, selection.to_space_id);
    let roundtrip: Program = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
    assert_eq!(roundtrip, plan);
}

#[test]
fn counterpart_selection_requires_explicit_distinct_bounded_spaces_and_integer_time() {
    let source = "use G graph \"G\"; counterparts C from G relation \"bridge\" entity \"e\" from space \"same\" to space \"same\" at 7;";
    assert_eq!(compile(source).unwrap_err().code, "E_SPACE");
    assert_eq!(
        compile(
            &source
                .replace("entity \"e\"", "entity \"\"")
                .replace("to space \"same\"", "to space \"other\"")
        )
        .unwrap_err()
        .code,
        "E_ID"
    );
    assert!(compile(&source.replace("at 7", "at 7.0")).is_err());
    let errors = compile(
        &source
            .replace("from G relation", "from Missing relation")
            .replace("to space \"same\"", "to space \"other\""),
    )
    .unwrap_err();
    assert_eq!(errors.code, "E_UNKNOWN_GRAPH");
}

#[test]
fn bootstrap_nodes_do_not_claim_runtime_node_origin_authority() {
    let plan = compile("graph G { node \"n\" entity \"e\" space \"s\" property \"derived_nodes\" \"untrusted data\"; }").unwrap();
    let Command::Commit { data, .. } = &plan.commands[0] else {
        panic!("commit expected")
    };
    assert!(data.nodes[0].derived_nodes.is_empty());
    assert_eq!(data.nodes[0].properties["derived_nodes"], "untrusted data");
    let wire = serde_json::to_value(&data.nodes[0]).unwrap();
    assert!(wire.get("derived_nodes").is_none());
    let plan = compile(r#"graph Legacy { node "n" entity "n" space "s";
        edge "e" from "n" to "n" relation "p" valid 0 until 10 property "node_premises" "data"; }
        graph Explicit explicit {node "n" entity "n" space "s";
        relation "e" from "n" to "n" predicate "p";
        claim "c" on "e" source "author" polarity positive valid 0 until 10 property "influence" "data";}"#).unwrap();
    for command in plan.commands {
        let Command::Commit { data, .. } = command else {
            panic!("commit expected")
        };
        assert!(data.influence.is_none());
        assert!(data.edges.iter().all(|edge| edge.derived_nodes.is_empty()));
        assert!(
            data.assertions
                .iter()
                .all(|claim| claim.derived_nodes.is_empty())
        );
        let wire = serde_json::to_value(&data).unwrap();
        assert!(wire.get("influence").is_none());
        for record in data
            .edges
            .iter()
            .map(|e| serde_json::to_value(e).unwrap())
            .chain(
                data.assertions
                    .iter()
                    .map(|a| serde_json::to_value(a).unwrap()),
            )
        {
            assert!(record.get("derived_nodes").is_none());
        }
    }
}

#[test]
fn exact_decimal_and_nominal_quantity_literals_lower_without_float_conversion() {
    let plan = compile(include_str!("../examples/quantities.weave")).unwrap();
    let Command::Commit { data, .. } = &plan.commands[0] else {
        panic!("commit expected")
    };
    let values = &data.nodes[0].properties;
    assert_eq!(values["exact_count"], serde_json::json!("9007199254740993"));
    assert_eq!(values["length"]["amount"], serde_json::json!("1.23"));
    assert_eq!(values["length"]["unit"]["revision"], serde_json::json!("1"));
    assert!(values["approximate"].is_f64());
    assert!(weave_contract::validate_schema_graph(data).is_empty());
    let source = include_str!("../examples/quantities.weave");
    assert!(
        compile(&source.replace("decimal \"9007199254740993\"", "9007199254740993.0")).is_err()
    );
    assert!(
        compile(&source.replace(
            "quantity \"+001.2300\" dimension \"length\" unit \"metre\" revision \"1\"",
            "quantity \"1.23\" dimension \"length\" unit \"metre\" revision \"2\""
        ))
        .is_err()
    );
}
#[test]
fn decimal_literal_bounds_report_the_offending_string() {
    let source = "graph G {node \"n\" entity \"e\" space \"s\" property \"x\" decimal \"1e-19\";}";
    let error = compile(source).unwrap_err();
    assert_eq!(error.code, "E_DECIMAL");
    assert_eq!(&source[error.start..error.end], "\"1e-19\"");
}

#[test]
fn pure_numeric_literal_operations_are_exact_bounded_and_explicit() {
    let source = r#"graph G { node "n" entity "e" space "s"
      property "sum" decimal_add(decimal "0.1", decimal "0.2")
      property "difference" decimal_sub(decimal "2", decimal "1.2")
      property "product" decimal_mul(decimal "1.25", decimal "0.8")
      property "ratio" decimal_div(decimal "1", decimal "8")
      property "scaled" quantity_scale(quantity "1.25" dimension "length" unit "m" revision "1", decimal "2")
      property "converted" quantity_convert(quantity "3" dimension "length" unit "m" revision "1", {
        "from":{"dimension_id":"length","unit_id":"m","revision":"1"},
        "to":{"dimension_id":"length","unit_id":"third-m","revision":"1"},
        "numerator":decimal "1", "denominator":decimal "3"}); }"#;
    let plan = compile(source).unwrap();
    let Command::Commit { data, .. } = &plan.commands[0] else {
        panic!()
    };
    let p = &data.nodes[0].properties;
    for (key, value) in [
        ("sum", "0.3"),
        ("difference", "0.8"),
        ("product", "1"),
        ("ratio", "0.125"),
    ] {
        assert_eq!(p[key], value);
    }
    assert_eq!(p["scaled"]["amount"], "2.5");
    assert_eq!(p["converted"]["amount"], "1");
    assert!(
        compile(&source.replace(
            "decimal_div(decimal \"1\", decimal \"8\")",
            "decimal_div(decimal \"1\", decimal \"3\")"
        ))
        .is_err()
    );
    assert!(
        compile(&source.replace(
            "decimal_add(decimal \"0.1\", decimal \"0.2\")",
            "decimal_add(0.1, 0.2)"
        ))
        .is_err()
    );
}

#[test]
fn native_services_lower_exact_pins_and_reusable_graph_values() {
    let identity = compile(include_str!("../examples/accepted_identity.weave")).unwrap();
    let Command::Bind {
        value: GraphExpression::ResolveIdentity { selection },
        ..
    } = &identity.commands[0]
    else {
        panic!("identity selection missing")
    };
    assert_eq!(selection.mapping_id, "equipment");
    assert_eq!(selection.source.revision, "SOURCE_REVISION");
    assert_eq!(selection.revision, "MAPPING_REVISION");
    assert_eq!(selection.context, weave_contract::ContextSelection::Default);
    assert!(
        identity
            .commands
            .iter()
            .all(|c| !matches!(c, Command::Commit { .. } | Command::CommitBatch { .. }))
    );
    let cluster = compile(include_str!("../examples/cluster_navigation.weave")).unwrap();
    let Command::Bind {
        value: GraphExpression::Cluster { selection },
        ..
    } = &cluster.commands[1]
    else {
        panic!("cluster selection missing")
    };
    assert_eq!(
        selection.source.revision,
        "logical:navigation_source:Network"
    );
    assert_eq!(selection.levels, 3);
    assert_eq!(selection.valid_at, 5);
    let roundtrip: Program =
        serde_json::from_str(&serde_json::to_string(&cluster).unwrap()).unwrap();
    assert_eq!(roundtrip, cluster);
}
#[test]
fn native_selectors_require_bounded_explicit_pins_context_and_integer_levels() {
    let source = "cluster_navigation N source graph \"G\" revision \"r\" relation \"p\" at 5 levels 3 context default;";
    for bad in [
        source.replace("revision \"r\" ", ""),
        source.replace("context default", ""),
        source.replace("levels 3", "levels 3.0"),
    ] {
        assert!(compile(&bad).is_err());
    }
    for count in ["-1", "10001"] {
        let bad = source.replace("levels 3", &format!("levels {count}"));
        let error = compile(&bad).unwrap_err();
        assert_eq!(error.code, "E_CLUSTER_INPUT");
        assert_eq!(&bad[error.start..error.end], count);
    }
    assert_eq!(
        compile(&source.replace("at 5", "at 9223372036854775807"))
            .unwrap_err()
            .code,
        "E_CLUSTER_TIME"
    );
    assert_eq!(
        compile(&source.replace("\"G\"", &format!("\"{}\"", "g".repeat(513))))
            .unwrap_err()
            .code,
        "E_ID"
    );
    let body = format!("function Hidden revision \"1\" (graph input) {{ {source} return N; }}");
    assert_eq!(compile(&body).unwrap_err().code, "E_FUNCTION_EFFECT");
}
#[test]
fn service_bindings_do_not_smuggle_policy_authority_or_schema_compatibility() {
    let cluster = "cluster_navigation N source graph \"G\" revision \"r\" relation \"p\" at 5 levels 0 context default;";
    let identity = "resolve_identity R source graph \"S\" revision \"s\" node \"n\" mapping \"m\" revision \"mr\" policy \"p\" revision \"pr\" to space \"ops\" at 5 context default;";
    assert_eq!(
        compile(&format!("{cluster}{identity}union Mixed from N with R;"))
            .unwrap_err()
            .code,
        "E_SCHEMA_ALGEBRA"
    );
    assert_eq!(
        compile(&format!("{cluster}{cluster}")).unwrap_err().code,
        "E_DUPLICATE"
    );
    assert!(compile("install_identity_policy Grant { approver \"reader\"; }").is_err());
    assert!(compile("accept_identity Accepted mapping \"m\";").is_err());
}

#[test]
fn typed_context_source_declarations_lower_canonical_descriptor_and_exact_read() {
    let source = include_str!("../examples/typed_contexts.weave");
    let program = weave_language::compile(source).unwrap();
    assert_eq!(program.version, "0.15.0");
    let json = serde_json::to_value(program).unwrap();
    assert_eq!(json["commands"][0]["op"], "commit_batch");
    let descriptor = &json["commands"][0]["commits"][0]["data"];
    assert_eq!(descriptor["profile"], "explicit");
    assert_eq!(descriptor["assertions"][0]["source"], "scenario-author");
    assert_eq!(descriptor["assertions"][0]["valid_time"]["start"], i64::MIN);
    assert_eq!(
        descriptor["assertions"][0]["properties"]["weave.context"]["values"]["load"],
        "0.75"
    );
    let selected = json["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Selected")
        .unwrap();
    assert_eq!(selected["value"]["kind"], "typed_context");
    assert_eq!(
        selected["value"]["reference"]["revision"],
        "logical:typed_worlds:Estonia"
    );
    assert_eq!(
        selected["value"]["expected_schema"]["reference"]["id"],
        "OperatingWorld"
    );
    assert_eq!(
        json["commands"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["op"] == "commit_batch" || c["op"] == "commit")
            .count(),
        1
    );
}
#[test]
fn context_assignment_errors_preserve_exact_literal_and_duplicate_spans() {
    let source = "// earlier region and 0.75\ncontext_schema S revision \"1\" { axis \"region\" decimal; } context_value C schema S source \"author\" { axis \"region\" 0.75; }";
    let error = weave_language::compile(source).unwrap_err();
    assert_eq!(error.code, "E_CONTEXT_VALUE");
    assert_eq!(&source[error.start..error.end], "0.75");
    let source = "// earlier duplicate\ncontext_schema S revision \"1\" { axis \"é\" string; axis \"é\" string; }";
    let error = weave_language::compile(source).unwrap_err();
    assert_eq!(error.code, "E_DUPLICATE");
    assert_eq!(error.start, source.rfind("\"é\"").unwrap());
    let source = "context_schema S revision \"1\" { axis \"r\" enum [\"EE\", \"EE\"]; }";
    assert_eq!(
        weave_language::compile(source).unwrap_err().code,
        "E_DUPLICATE"
    );
}
#[test]
fn typed_context_requires_total_exact_assignments_and_declared_explicit_pins() {
    let schema = "context_schema S revision \"1\" {axis \"x\" integer;}";
    for body in [
        "",
        "axis \"y\" 1;",
        "axis \"x\" 1.5;",
        "axis \"x\" true;",
        "axis \"x\" 1; axis \"x\" 2;",
    ] {
        let source = format!("{schema} context_value C schema S source \"a\" {{{body}}}");
        assert!(weave_language::compile(&source).is_err(), "{body}");
    }
    for source in [
        "context_value C schema Missing source \"a\" {axis \"x\" 1;}",
        "graph G{} typed_context T from G graph \"C\" schema S;",
        "graph G{} typed_context T from G graph \"C\" revision \"r\" schema Missing;",
        "context_schema S revision \"1\"{axis \"x\" float;}",
        "context_schema S revision \"1\"{}",
    ] {
        assert!(weave_language::compile(source).is_err(), "{source}");
    }
    let source = format!(
        "{schema} graph G{{}} function F revision \"1\"(graph input) {{typed_context T from input graph \"C\" revision \"r\" schema S; return T;}} "
    );
    assert_eq!(
        weave_language::compile(&source).unwrap_err().code,
        "E_FUNCTION_EFFECT"
    );
}
#[test]
fn context_quantities_validate_nominal_units_without_authorizing_conversion() {
    let prefix = "context_schema S revision \"1\" {axis \"x\" quantity dimension \"length\" unit \"metre\" revision \"1\";} context_value C schema S source \"author\" {axis \"x\" ";
    let good = format!(
        "{prefix} quantity \"1.250\" dimension \"length\" unit \"metre\" revision \"1\";}}"
    );
    assert!(weave_language::compile(&good).is_ok());
    let wrong = good.replacen(
        "quantity \"1.250\" dimension \"length\" unit \"metre\" revision \"1\"",
        "quantity \"1.250\" dimension \"length\" unit \"metre\" revision \"2\"",
        1,
    );
    assert_eq!(
        weave_language::compile(&wrong).unwrap_err().code,
        "E_CONTEXT_VALUE"
    );
}

#[test]
fn live_handles_are_deferred_and_pin_emits_an_immutable_reference() {
    let plan = compile(
        r#"
        live_handle H graph "Evidence" branch "local";
        pin Snapshot from H at 5 metadata depth 2;
        lens Reused from Snapshot { at 5; }
    "#,
    )
    .unwrap();
    assert_eq!(plan.commands.len(), 2);
    let json = serde_json::to_value(plan).unwrap();
    assert_eq!(json["version"], "0.15.0");
    let query = &json["commands"][0]["value"]["query"];
    assert_eq!(query["graph_id"], "Evidence");
    assert_eq!(query["branch_id"], "local");
    assert!(query["revision"].is_null());
    assert_eq!(query["valid_at"], 5);
    assert_eq!(query["include_metadata"], true);
    assert_eq!(query["max_depth"], 2);
    assert_eq!(json["commands"][1]["value"]["input"]["kind"], "reference");
    assert_eq!(json["commands"][1]["value"]["input"]["name"], "Snapshot");
    assert!(
        compile(r#"live_handle H graph "Evidence" branch "main";"#)
            .unwrap()
            .commands
            .is_empty()
    );
}

#[test]
fn live_handle_types_and_pure_function_read_boundaries_are_explicit() {
    for source in [
        r#"live_handle H graph "G" branch "main"; lens Bad from H {}"#,
        r#"graph G {} pin Bad from G;"#,
        r#"pin Bad from Missing;"#,
        r#"live_handle H graph "G" branch "main"; function F revision "1" (graph x) { lens X from x {} return X; } apply Bad from F { graph x H; }"#,
        r#"function F revision "1" (graph x) { live_handle H graph "G" branch "main"; return x; }"#,
        r#"function F revision "1" (graph x) { pin Bad from x; return x; }"#,
        r#"live_handle H graph "G";"#,
        r#"live_handle H graph "G" branch "";"#,
        r#"live_handle H graph "G" branch "main"; pin Bad from H at 1.5;"#,
        r#"live_handle H graph "G" branch "main"; pin Bad from H metadata depth 33;"#,
    ] {
        assert!(compile(source).is_err(), "unexpectedly compiled {source}");
    }
    let source = "// H is not a materialized graph\nlive_handle H graph \"G\" branch \"main\"; pin V from Missing;";
    let error = compile(source).unwrap_err();
    assert_eq!(error.code, "E_HANDLE_TYPE");
    assert_eq!(&source[error.start..error.end], "Missing");
}

#[test]
fn explicit_snapshot_replacement_preserves_branch_cas_and_metadata_binding_kind() {
    let plan = compile(r#"
        transaction changed {
            graph Catalog branch "local" replace revision "old-head" {
                node "a" entity "A" space "s";
                attachment "live" on node "a" key "live" live graph "Evidence" branch "remote-cache" valid 0 until 10;
                attachment "fixed" on node "a" key "fixed" graph "Evidence" revision "r1" valid 0 until 10;
            }
        }
        lens Current from Catalog {}
    "#).unwrap();
    let json = serde_json::to_value(plan).unwrap();
    let commit = &json["commands"][0]["commits"][0];
    assert_eq!(commit["expected_head"], "old-head");
    assert_eq!(commit["branch_id"], "local");
    assert_eq!(
        commit["data"]["attachments"][0]["value"]["kind"],
        "live_graph"
    );
    assert_eq!(
        commit["data"]["attachments"][0]["value"]["branch_id"],
        "remote-cache"
    );
    assert_eq!(commit["data"]["attachments"][1]["value"]["kind"], "graph");
    assert_eq!(json["commands"][1]["value"]["query"]["branch_id"], "local");
    for source in [
        r#"graph G replace {}"#,
        r#"graph G replace revision "" {}"#,
        r#"graph G branch "" {}"#,
        r#"graph G replace revision "r" replace revision "s" {}"#,
    ] {
        assert!(compile(source).is_err());
    }
}

#[test]
fn schema_constrained_functions_support_partial_higher_order_and_captured_calls() {
    let plan = compile(include_str!("../examples/schema_functions.weave")).unwrap();
    assert_eq!(plan.version, "0.15.0");
    assert_eq!(
        plan.commands
            .iter()
            .filter(|c| matches!(c, weave_contract::Command::CommitBatch { .. }))
            .count(),
        1
    );
    assert_eq!(plan.source_revisions.len(), 3);
    let body = r#"
        schema S revision "1" { node N space "s" {} }
        function Keep revision "1" (graph input schema S) returns graph schema S {
            context Scoped from input default;
            project Result from Scoped { node "a"; }
            return Result;
        }
        graph G schema S { node "a" type N entity "A" space "s"; }
        apply Result from Keep { graph input G; }
    "#;
    assert!(compile(body).is_ok());
}

#[test]
fn exact_schema_constraints_reject_unknown_untyped_and_distinct_nominal_graphs() {
    let header = r#"schema S revision "1" { node N space "s" {} }
        function Keep revision "1" (graph input schema S) returns graph schema S { return input; }
    "#;
    for (declaration, expected) in [
        (r#"graph G {}"#, "E_SCHEMA_MISMATCH"),
        (
            r#"schema Other revision "1" { node N space "s" {} } graph G schema Other {}"#,
            "E_SCHEMA_MISMATCH",
        ),
        (r#"use G graph "remote" revision "r";"#, "E_SCHEMA_UNKNOWN"),
        (
            r#"live_handle H graph "remote" branch "main"; pin G from H;"#,
            "E_SCHEMA_UNKNOWN",
        ),
    ] {
        let source = format!(
            "{header}{declaration}// G mentioned before the actual argument\napply Bad from Keep {{ graph input G; }}"
        );
        let error = compile(&source).unwrap_err();
        assert_eq!(error.code, expected, "{source}");
        assert_eq!(&source[error.start..error.end], "G");
        assert_eq!(error.start, source.rfind("G;").unwrap());
    }
    let unknown =
        compile("function F revision \"1\" (graph x schema Missing) { return x; }").unwrap_err();
    assert_eq!(unknown.code, "E_UNKNOWN_SCHEMA");
}

#[test]
fn returns_require_proven_preservation_and_algebra_cannot_erase_constraints() {
    let schema = r#"schema S revision "1" { node N space "s" {} edge E from N to N {} }"#;
    for source in [
        r#"function Bad revision "1" (graph input) returns graph schema S { return input; }"#,
        r#"function Bad revision "1" (graph input schema S) returns graph schema S { union Result from input with input; return Result; }"#,
        r#"function Bad revision "1" (graph input schema S) returns graph schema S { join Result from input to input relation "q"; return Result; }"#,
        r#"function Bad revision "1" (graph input schema S) returns graph schema S { explain Result from input; return Result; }"#,
    ] {
        let source = format!("{schema}{source}");
        let error = compile(&source).unwrap_err();
        assert_eq!(error.code, "E_SCHEMA_UNKNOWN");
        assert_eq!(error.start, source.rfind("return ").unwrap() + 7);
    }
    let source = format!(
        r#"{schema}
        function Keep revision "1" (graph input schema S) {{ return input; }}
        graph A schema S {{}}
        union Combined from A with A;
        apply Bad from Keep {{ graph input Combined; }}"#
    );
    assert_eq!(compile(&source).unwrap_err().code, "E_SCHEMA_UNKNOWN");
}

#[test]
fn higher_order_and_partial_capture_preserve_schema_obligations() {
    let source = include_str!("../examples/schema_functions.weave");
    let bad = source.replace(
        "apply HigherOrder from Specialized { graph input Network; }",
        "graph Wrong {} apply HigherOrder from Specialized { graph input Wrong; }",
    );
    assert_eq!(compile(&bad).unwrap_err().code, "E_SCHEMA_MISMATCH");
    let bad = source.replace(
        "function Captured revision \"1\" (graph input schema Infrastructure)",
        "function Captured revision \"1\" (graph input)",
    );
    assert_eq!(compile(&bad).unwrap_err().code, "E_SCHEMA_UNKNOWN");
    let bad = source.replace(
        "apply AtFive from Select { time instant 5; }",
        "graph Wrong {} apply AtFive from Select { graph input Wrong; }",
    );
    assert_eq!(compile(&bad).unwrap_err().code, "E_SCHEMA_MISMATCH");
    assert!(
        compile("function F revision \"1\" (time instant schema S) { return instant; }").is_err()
    );
}

#[test]
fn schema_function_identity_includes_descriptor_meaning_but_excludes_locations() {
    let source = include_str!("../examples/schema_functions.weave");
    let original = compile(source).unwrap().source_revisions;
    let shifted = compile(&format!("// Unicode λ and earlier span mentions\n{source}"))
        .unwrap()
        .source_revisions;
    assert_eq!(original, shifted);
    let changed = compile(&source.replace(
        "property \"label\" string required;",
        "property \"label\" string optional;",
    ))
    .unwrap()
    .source_revisions;
    assert_ne!(original[0].digest, changed[0].digest);
    assert_ne!(original[1].digest, changed[1].digest);
    // Unconstrained Transform's source is unchanged; its concrete graph dependencies remain in result identity.
    assert_eq!(original[2].digest, changed[2].digest);
    let duplicate =
        format!("{source}\nschema Infrastructure revision \"1\" {{ node Different {{}} }}");
    assert_eq!(compile(&duplicate).unwrap_err().code, "E_DUPLICATE");
}
