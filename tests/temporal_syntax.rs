use weave_language::modules::{SourceModule, content_digest, link};
use weave_language::syntax::{Statement, TemporalRelation};
use weave_language::{compile, compile_artifacts, format_source, parse};

#[test]
fn grammar_records_graph_scalar_and_relation_spans() {
    let source = "window W from Input during interval(time 1, time 9);\nsequence S from W to Next overlaps during interval_open(time 3);";
    let ast = parse(source).unwrap();
    let Statement::Temporal {
        name,
        source: input,
        source_span,
        window,
        sequence,
        ..
    } = &ast.statements[0]
    else {
        panic!("temporal AST")
    };
    assert_eq!(name, "W");
    assert_eq!(input, "Input");
    assert_eq!(&source[source_span.0..source_span.1], "Input");
    assert_eq!(
        &source[window.span.0..window.span.1],
        "interval(time 1, time 9)"
    );
    assert!(sequence.is_none());
    let Statement::Temporal {
        sequence: Some(s),
        window,
        ..
    } = &ast.statements[1]
    else {
        panic!("sequence AST")
    };
    assert_eq!(s.relation, TemporalRelation::Overlaps);
    assert_eq!(s.right, "Next");
    assert_eq!(&source[s.relation_span.0..s.relation_span.1], "overlaps");
    assert_eq!(&source[s.right_span.0..s.right_span.1], "Next");
    assert_eq!(
        &source[window.span.0..window.span.1],
        "interval_open(time 3)"
    );
    for relation in ["before", "meets", "overlaps", "within"] {
        parse(&format!(
            "sequence S from A to B {relation} during interval(time 0,time 1);"
        ))
        .unwrap();
    }
    let bad = "sequence S from A to B after during interval(time 0,time 1);";
    let e = parse(bad).unwrap_err();
    assert_eq!(e.code, "E_TEMPORAL_RELATION");
    assert_eq!(&bad[e.start..e.end], "after");
}
#[test]
fn unused_body_types_and_closed_interval_errors_are_checked() {
    let source = include_str!("../docs/proposals/temporal-fixtures/windows.weave");
    let value = compile_artifacts(source).unwrap();
    assert!(value.program.commands.is_empty());
    for (expression, code) in [
        ("time 0", "E_SCALAR_TYPE"),
        ("0", "E_SCALAR_TYPE"),
        ("interval(time 5,time 5)", "E_INTERVAL_BOUNDS"),
    ] {
        let source = format!(
            "function F revision \"1\" (graph input) {{ window W from input during {expression}; return W; }}"
        );
        let e = compile(&source).unwrap_err();
        assert_eq!(e.code, code, "{source}");
        assert_eq!(&source[e.start..e.end], expression);
    }
    let missing = "function F revision \"1\" (graph input, interval span) {sequence S from input to Missing before during param span; return S;}";
    let e = compile(missing).unwrap_err();
    assert_eq!(e.code, "E_FUNCTION_SCOPE");
    assert_eq!(&missing[e.start..e.end], "Missing");
    let scalar = "function F revision \"1\" (interval span) returns integer {window W from Ghost during param span; return 1;}";
    assert_eq!(compile(scalar).unwrap_err().code, "E_FUNCTION_EFFECT");
}
#[test]
fn instantiated_operators_emit_typed_plans_and_complete_sdk_artifacts() {
    let source = "function F revision \"1\" (graph input,interval span){window W from input during param span;return W;} apply Partial from F{interval span interval(time 0,time 10);} graph G{} apply Result from Partial{graph input G;}";
    let artifact = compile_artifacts(source).unwrap();
    assert_eq!(artifact.program.version, "0.19.0");
    assert!(
        serde_json::to_string(&artifact.program)
            .unwrap()
            .contains("\"kind\":\"window\"")
    );
    for (suffix, kind) in [
        ("window W from G during interval(time 0,time 10);", "window"),
        (
            "sequence S from G to G before during interval(time 0,time 10);",
            "sequence",
        ),
    ] {
        let source = format!("graph G{{}} {suffix}");
        let program = compile(&source).unwrap();
        let encoded = serde_json::to_value(&program).unwrap();
        let expression = &encoded["commands"].as_array().unwrap().last().unwrap()["value"];
        assert_eq!(expression["kind"], kind);
        assert_eq!(
            expression["window"],
            serde_json::json!({"start":0,"end":10})
        );
        let bytes=weave_language::sdk::compile_request(&serde_json::to_vec(&serde_json::json!({"format":"weave-compiler-request/1","entry_id":"main","source":source,"modules":[]})).unwrap());
        let response: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["artifacts"]["program"], encoded);
    }
}
#[test]
fn exact_schema_transfer_and_mismatch_are_checked_in_unused_definitions() {
    let schema = "schema S revision \"1\"{node N space \"s\" {}}";
    let source = format!(
        "{schema} function W revision \"1\"(graph input schema S,interval span) returns graph schema S{{window X from input during param span;return X;}} function Q revision \"1\"(graph input schema S,graph other schema S,interval span) returns graph schema S{{sequence X from input to other within during param span;return X;}}"
    );
    assert!(compile(&source).unwrap().commands.is_empty());
    let bad = format!(
        "{schema} schema T revision \"1\"{{node N space \"other\"{{}}}} function Q revision \"1\"(graph input schema S,graph other schema T,interval span){{sequence X from input to other before during param span;return X;}}"
    );
    assert_eq!(compile(&bad).unwrap_err().code, "E_SCHEMA_MISMATCH");
}
#[test]
fn original_module_spans_and_no_qualified_graph_exports() {
    let module = "module \"temporal\" revision \"1\";\nfunction F revision \"1\"(graph input){window W from input during time 0;return W;}";
    let entry = format!(
        "import t module \"temporal\" revision \"1\" sha256 {:?};",
        content_digest(module)
    );
    let units = [SourceModule {
        id: "temporal",
        revision: "1",
        source: module,
    }];
    let e = link("main", &entry, &units).unwrap().compile().unwrap_err();
    assert_eq!(e.source_id, "module:temporal@1");
    assert_eq!(e.code, "E_SCALAR_TYPE");
    assert_eq!(&module[e.start..e.end], "time 0");
    let forged = "function F revision \"1\"(graph input,interval span){sequence X from input to __weave_module_fake_Output before during param span;return X;}";
    let e = link("main", forged, &[]).err().unwrap();
    assert_eq!(e.code, "E_MODULE_NAME");
    assert_eq!(&forged[e.start..e.end], "__weave_module_fake_Output");
    let qualified = "function F revision \"1\"(graph input,interval span){window X from t::Output during param span;return X;}";
    assert_eq!(
        link("main", qualified, &[]).err().unwrap().code,
        "E_MODULE_KIND"
    );
}
#[test]
fn formatter_and_function_identities_ignore_layout_but_preserve_literals() {
    let source = "// ✓ temporal\nfunction W revision \"1\"(graph input,interval span){window Result from input during param span;return Result;}\nfunction Q revision \"1\"(graph input,graph next,interval span){sequence Result from input to next meets during param span;return Result;}";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("// ✓ temporal"));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
    assert_eq!(compile(source).unwrap(), compile(&formatted).unwrap());
    let module = "module \"temporal\" revision \"1\";function W revision \"1\"(graph input,interval span){window X from input during param span;return X;}";
    let mut outputs = vec![];
    for alias in ["first", "renamed"] {
        let source = format!(
            "import {alias} module \"temporal\" revision \"1\" sha256 {:?}; apply Bound from {alias}::W {{interval span interval(time 0,time 10);}} graph G{{node \"n\" entity \"first::W\" space \"s\" property \"relation_span\" \"first::W\";}} apply Result from Bound {{graph input G;}}",
            content_digest(module)
        );
        outputs.push(
            link(
                "main",
                &source,
                &[SourceModule {
                    id: "temporal",
                    revision: "1",
                    source: module,
                }],
            )
            .unwrap()
            .compile()
            .unwrap(),
        );
    }
    assert_eq!(outputs[0], outputs[1]);
    let json = serde_json::to_value(&outputs[0]).unwrap();
    assert_eq!(
        json["commands"][0]["data"]["nodes"][0]["properties"]["relation_span"],
        "first::W"
    );
}

#[test]
fn temporal_handler_recipe_is_inert_and_artifact_complete() {
    let source = r#"function Clip revision "1" (graph input){
        window W from input during interval(time -9223372036854775808,time 9223372036854775807);
        return W;
    }
    handler ClipEvent revision "1" using Clip {
        input event graph "Events" branch "main" metadata depth 2;
        on "graph.accepted", "graph.committed";
        output slot "clipped";
        replay pinned;
    }"#;
    let artifacts = compile_artifacts(source).unwrap();
    assert!(artifacts.program.commands.is_empty());
    let handler = &artifacts.handler_templates["ClipEvent"];
    assert_eq!(handler.protocol, "0.19.0");
    assert!(
        handler
            .recipe
            .bindings
            .iter()
            .any(|b| matches!(b.value, weave_contract::GraphExpression::Window { .. }))
    );
    assert!(compile(source).is_err());
    let formatted = format_source(source).unwrap();
    assert_eq!(
        serde_json::to_value(&artifacts).unwrap(),
        serde_json::to_value(compile_artifacts(&formatted).unwrap()).unwrap()
    );
}
