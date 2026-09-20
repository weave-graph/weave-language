//! Independent scalar boundary tests from orchestration review.
use weave_language::{compile, specialize};

#[test]
fn root_unused_closed_subexpressions_fail_without_evaluating_symbolic_operands() {
    for body in [
        "return integer_div(1,0);",
        "return integer_add(param input,integer_div(1,0));",
        "value Zero integer_sub(3,3); return integer_div(1,value Zero);",
    ] {
        let source =
            format!("function F revision \"1\" (integer input) returns integer {{{body}}}");
        let e = compile(&source).expect_err("closed division by zero in unused body");
        assert_eq!(e.code, "E_NUMERIC", "{source}");
    }
    assert!(compile("function F revision \"1\" (integer input) returns integer {return integer_div(1,param input);}").is_ok());
}

#[test]
fn root_graph_property_substitution_cannot_erase_scalar_nominality() {
    for (declaration, property_type) in [
        ("value X decimal \"0.3\";", "string"),
        ("value X \"0.3\";", "decimal"),
        ("value X time 5;", "integer"),
    ] {
        let source = format!(
            "{declaration} schema S revision \"1\" {{node N space \"s\" {{property \"x\" {property_type} required;}}}} graph G schema S {{node \"n\" type N entity \"n\" space \"s\" property \"x\" value X;}}"
        );
        assert!(compile(&source).is_err(), "nominal type erased: {source}");
    }
}

#[test]
fn root_partial_scalar_callback_preserves_nominal_unit_and_exact_value() {
    let source = r#"
function Add revision "1" (quantity input dimension "length" unit "m" revision "1", quantity offset dimension "length" unit "m" revision "1") returns quantity dimension "length" unit "m" revision "1" {return quantity_add(param input,param offset);}
apply Offset from Add {quantity offset quantity "0.1" dimension "length" unit "m" revision "1";}
function Use revision "1" (function op(quantity input dimension "length" unit "m" revision "1") returns quantity dimension "length" unit "m" revision "1") returns quantity dimension "length" unit "m" revision "1" {apply Local from op {quantity input quantity "0.2" dimension "length" unit "m" revision "1";} return value Local;}
apply Result from Use {function op Offset;}
"#;
    let r = specialize(source).unwrap();
    assert_eq!(r.values["Result"].json()["amount"], "0.3");
    assert!(r.program.commands.is_empty());
    assert_eq!(r.values.len(), 1);
    let bad = source.replace(
        "returns quantity dimension \"length\" unit \"m\" revision \"1\")",
        "returns quantity dimension \"length\" unit \"m\" revision \"2\")",
    );
    assert!(compile(&bad).is_err());
}
