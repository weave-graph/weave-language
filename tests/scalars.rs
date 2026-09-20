use weave_contract::Command;
use weave_language::{compile, scalars::ScalarValue, specialize};
#[test]
fn arithmetic_values_lower_to_exact_schema_checked_properties() {
    let result = specialize(include_str!(
        "../docs/proposals/scalar-functions/fixtures/arithmetic.weave"
    ))
    .unwrap();
    assert_eq!(result.values["Total"].json(), "0.3");
    assert_eq!(
        result.values["ExactInteger"],
        ScalarValue::Integer(9007199254740994)
    );
    assert_eq!(result.values["Flag"], ScalarValue::Boolean(true));
    assert_eq!(result.values["Text"].json(), "length ✓");
    assert_eq!(result.values["Length"].json()["amount"], "2.5");
    assert_eq!(result.program.commands.len(), 1);
    let Command::Commit { data, .. } = &result.program.commands[0] else {
        panic!("authored commit")
    };
    assert_eq!(data.nodes[0].properties["total"], "0.3");
    assert_eq!(data.nodes[0].properties["count"], 9007199254740994_i64);
    assert!(weave_contract::validate_schema_graph(data).is_empty());
}
#[test]
fn scalar_higher_order_partial_application_has_no_runtime_commands() {
    let result = specialize(include_str!(
        "../docs/proposals/scalar-functions/fixtures/higher_order.weave"
    ))
    .unwrap();
    assert_eq!(result.values["HigherOrder"], ScalarValue::Integer(42));
    assert_eq!(result.values["HigherOrder"], result.values["Direct"]);
    assert_eq!(result.values["Reference"], result.values["Direct"]);
    assert!(result.program.commands.is_empty());
    assert_eq!(
        result.values.len(),
        3,
        "internal local scalar names are not exports"
    );
}
#[test]
fn graph_return_schema_obligations_survive_scalar_local_bindings() {
    let source = include_str!("../docs/proposals/scalar-functions/fixtures/typed_graph.weave");
    assert!(compile(source).is_ok());
    let bad = source.replace("time instant 5;", "time instant value Wrong;");
    assert_eq!(
        compile(&format!("value Wrong 5;\n{bad}")).unwrap_err().code,
        "E_PARAMETER_TYPE"
    );
}
fn code(source: &str) -> String {
    compile(source).unwrap_err().code
}
#[test]
fn exact_types_cannot_be_laundered_through_argument_markers_or_results() {
    assert_eq!(
        code(
            "function F revision \"1\" (decimal input) returns decimal { return param input; } apply A from F {decimal input \"0.1\";}"
        ),
        "E_PARAMETER_TYPE"
    );
    assert_eq!(
        code("function F revision \"1\" (integer input) returns integer { return true; }"),
        "E_SCALAR_RETURN"
    );
    assert_eq!(
        code(
            "value I 5; function F revision \"1\" (time input) returns time {return param input;} apply A from F {time input value I;}"
        ),
        "E_PARAMETER_TYPE"
    );
    assert_eq!(code("value D 0.1;"), "E_SCALAR_TYPE");
}
#[test]
fn quantity_nominality_applies_to_parameters_returns_and_arithmetic() {
    let f = "function F revision \"1\" (quantity input dimension \"length\" unit \"metre\" revision \"1\") returns quantity dimension \"length\" unit \"metre\" revision \"1\" {return param input;}";
    let bad = format!(
        "{f} apply A from F {{ quantity input quantity \"2\" dimension \"length\" unit \"metre\" revision \"2\"; }}"
    );
    assert_eq!(code(&bad), "E_PARAMETER_TYPE");
    assert_eq!(
        code(&f.replace(
            "returns quantity dimension \"length\" unit \"metre\" revision \"1\"",
            "returns quantity dimension \"length\" unit \"cm\" revision \"1\""
        )),
        "E_SCALAR_RETURN"
    );
}
#[test]
fn unused_definitions_are_symbolic_and_concrete_numeric_errors_have_call_trace() {
    let f = "function Divide revision \"1\" (decimal input, decimal denominator) returns decimal {return decimal_div(param input,param denominator);}";
    assert!(compile(f).is_ok());
    let s = format!(
        "{f} apply Bad from Divide {{decimal input decimal \"1\";decimal denominator decimal \"0\";}}"
    );
    let error = compile(&s).unwrap_err();
    assert_eq!(error.code, "E_NUMERIC");
    assert!(s[error.start..error.end].starts_with("decimal_div"));
    assert_eq!(error.trace.len(), 1);
}
#[test]
fn finite_integer_and_decimal_failures_never_fallback_to_float() {
    for expr in [
        "integer_add(9223372036854775807,1)",
        "integer_div(3,2)",
        "integer_div(1,0)",
        "integer_div(-9223372036854775808,-1)",
        "decimal_div(decimal \"1\",decimal \"3\")",
        "decimal_add(decimal \"999999999999999999\",decimal \"1\")",
    ] {
        assert_eq!(
            code(&format!("value Failure {expr};")),
            "E_NUMERIC",
            "{expr}"
        );
    }
    let r=specialize("value Negative integer_div(-8,2); value Less decimal_lt(decimal \"-999999999999999999\",decimal \"0.000000000000000001\");").unwrap();
    assert_eq!(r.values["Negative"], ScalarValue::Integer(-4));
    assert_eq!(r.values["Less"], ScalarValue::Boolean(true));
}
#[test]
fn scalar_only_work_and_allocations_are_bounded() {
    let mut s = "value A0 \"abcdefgh\";".to_string();
    for i in 1..25 {
        s += &format!(
            "value A{i} string_concat(value A{},value A{});",
            i - 1,
            i - 1
        );
    }
    assert_eq!(code(&s), "E_SCALAR_BUDGET");
    let mut expr = "param input".to_string();
    for _ in 0..25 {
        expr = format!("integer_add(1,{expr})");
    }
    let mut s =
        format!("function Work revision \"1\" (integer input) returns integer {{return {expr};}}");
    for i in 0..2100 {
        s += &format!("apply A{i} from Work {{integer input 1;}}");
    }
    assert_eq!(code(&s), "E_SCALAR_BUDGET");
}
#[test]
fn scalar_results_have_separate_identity_and_preserve_literal_object_keys() {
    let a = specialize("value A 1;").unwrap();
    let b = specialize("value A 2;").unwrap();
    assert_eq!(a.program, b.program);
    assert_ne!(a.fingerprint().unwrap(), b.fingerprint().unwrap());
    let p=compile("value X decimal \"0.3\"; graph G {node \"n\" entity \"n\" space \"s\" property \"data\" {\"value\":value X,\"span\":[1,2],\"param\":\"unchanged\"};}").unwrap();
    let Command::Commit { data, .. } = &p.commands[0] else {
        panic!()
    };
    assert_eq!(
        data.nodes[0].properties["data"],
        serde_json::json!({"value":"0.3","span":[1,2],"param":"unchanged"})
    );
}
#[test]
fn scalar_functions_cannot_capture_entry_values_or_runtime_graph_contents() {
    assert_eq!(
        code("value Secret 1; function F revision \"1\" () returns integer {return value Secret;}"),
        "E_SCALAR_UNKNOWN"
    );
    assert_eq!(
        code("function F revision \"1\" (graph input) returns integer {return param input;}"),
        "E_FUNCTION_SCOPE"
    );
}
#[test]
fn explicit_callback_signature_checks_result_and_remaining_name() {
    let f = "function F revision \"1\" (integer input) returns boolean {return true;} function Use revision \"1\" (function transform(integer input) returns integer) returns integer { apply A from transform {integer input 1;} return value A;} apply B from Use {function transform F;}";
    assert_eq!(code(f), "E_FUNCTION_SIGNATURE");
}
#[test]
fn imported_runtime_numeric_error_keeps_original_body_and_application_locations() {
    use weave_language::modules::*;
    let dependency = "module \"math\" revision \"1\"; function Divide revision \"1\" (decimal input) returns decimal {return decimal_div(decimal \"1\",param input);}";
    let entry = format!(
        "import m module \"math\" revision \"1\" sha256 \"{}\"; apply A from m::Divide {{decimal input decimal \"0\";}}",
        content_digest(dependency)
    );
    let link = link(
        "entry",
        &entry,
        &[SourceModule {
            id: "math",
            revision: "1",
            source: dependency,
        }],
    )
    .unwrap();
    let error = link.specialize().unwrap_err();
    assert_eq!(error.source_id, "module:math@1");
    assert!(dependency[error.start..error.end].starts_with("decimal_div"));
    assert_eq!(error.application_trace[0].source_id, "entry");
}

#[test]
fn scalar_attachment_payload_and_explicit_conversion_use_existing_wire() {
    let s = r#"value Q quantity_convert(quantity "3" dimension "length" unit "m" revision "1", {"from":{"dimension_id":"length","unit_id":"m","revision":"1"},"to":{"dimension_id":"length","unit_id":"third_m","revision":"1"},"numerator":decimal "1","denominator":decimal "3"}); graph G {node "n" entity "n" space "s"; attachment "a" on node "n" key "amount" literal value Q valid 0 until infinity;}"#;
    let output = specialize(s).unwrap();
    assert_eq!(output.values["Q"].json()["amount"], "1");
    let Command::Commit { data, .. } = &output.program.commands[0] else {
        panic!()
    };
    let weave_contract::MetadataValue::Literal { value } = &data.attachments[0].value else {
        panic!()
    };
    assert_eq!(value, &output.values["Q"].json());
}
#[test]
fn callback_schemas_bind_complete_descriptors_and_ignore_source_whitespace() {
    let s = r#"schema S revision "1" {node N {property "x" string required;}}
function F revision "1" (graph input schema S, function map(graph input schema S) returns graph schema S) returns graph schema S {apply Result from map {graph input input;} return Result;}"#;
    let a = compile(s).unwrap();
    let b = compile(&s.replace(" ", "  ").replace(";", ";\n// a comment\n")).unwrap();
    assert_eq!(a.source_revisions, b.source_revisions);
    let c = compile(&s.replace("property \"x\" string", "property \"x\" integer")).unwrap();
    assert_ne!(a.source_revisions, c.source_revisions);
}
#[test]
fn scalar_expression_spans_do_not_change_source_manifest() {
    let a = "function F revision \"1\" (decimal input) returns decimal {value X decimal_add(param input,decimal \"0.1\");return value X;} apply A from F {decimal input decimal \"0.2\";}";
    let b = a.replace(" ", "\n ");
    assert_eq!(
        compile(a).unwrap().source_revisions,
        compile(&b).unwrap().source_revisions
    );
}
#[test]
fn callback_schema_errors_point_to_original_imported_schema_reference() {
    use weave_language::modules::*;
    let dependency = "module \"math\" revision \"1\"; function F revision \"1\" (function map(graph input schema Missing) returns graph) {apply A from map {} return A;}";
    let entry = format!(
        "import m module \"math\" revision \"1\" sha256 \"{}\";",
        content_digest(dependency)
    );
    let e = link(
        "entry",
        &entry,
        &[SourceModule {
            id: "math",
            revision: "1",
            source: dependency,
        }],
    )
    .unwrap()
    .compile()
    .unwrap_err();
    assert_eq!(e.source_id, "module:math@1");
    assert_eq!(&dependency[e.start..e.end], "Missing");
}
#[test]
fn compiler_local_scalar_names_are_not_authored_value_aliases() {
    assert_eq!(
        code("value Probe value __weave_function_1_Secret;"),
        "E_FUNCTION_SCOPE"
    );
}

#[test]
fn unused_scalar_functions_cannot_hide_graph_captures_or_operations() {
    for source in [
        "function F revision \"1\" (graph input) returns integer {return 1;}",
        "function F revision \"1\" (function op) returns integer {return 1;}",
        "function F revision \"1\" (function op(graph input) returns integer) returns integer {return 1;}",
        "function G revision \"1\" (graph input) {return input;} function F revision \"1\" () returns integer {apply R from G {} return 1;}",
        "function F revision \"1\" () returns integer {lens R from Hidden {} return 1;}",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let result=specialize("function F revision \"1\" (integer input) returns integer {return param input;} apply A from F {integer input 3;}").unwrap();
    assert!(result.program.commands.is_empty());
}
