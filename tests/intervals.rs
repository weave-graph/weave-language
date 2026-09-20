use weave_language::{
    compile_artifacts,
    interval::{Interval, IntervalError},
    scalars::ScalarValue,
};

#[test]
fn checked_values_and_strict_deserialization_keep_private_invariants() {
    assert_eq!(Interval::new(3, Some(3)), Err(IntervalError::InvalidBounds));
    assert_eq!(Interval::new(4, Some(3)), Err(IntervalError::InvalidBounds));
    let widest = Interval::new(i64::MIN, Some(i64::MAX)).unwrap();
    assert!(widest.contains(i64::MIN));
    assert!(!widest.contains(i64::MAX));
    let open = Interval::new(i64::MAX, None).unwrap();
    assert!(open.contains(i64::MAX));
    assert_eq!(open.finite_end(), Err(IntervalError::UnboundedEnd));
    assert!(widest.meets(open));
    assert!(!widest.before(open));
    assert!(!widest.overlaps(open));
    assert_eq!(
        widest.intersection(open),
        Err(IntervalError::EmptyIntersection)
    );
    assert!(open.within(Interval::new(i64::MIN, None).unwrap()));
    for malformed in [
        r#"{"start":2,"end":1}"#,
        r#"{"start":1,"end":1}"#,
        r#"{"start":0}"#,
        r#"{"end":null}"#,
        r#"{"start":0,"end":null,"end":3}"#,
        r#"{"start":0,"start":1,"end":3}"#,
        r#"{"start":0,"end":1,"extra":true}"#,
        r#"{"start":0.0,"end":1}"#,
        r#"{"start":0,"end":"1"}"#,
        r#"{"start":0,"end":9223372036854775808}"#,
    ] {
        assert!(
            serde_json::from_str::<Interval>(malformed).is_err(),
            "{malformed}"
        );
    }
    let encoded = serde_json::to_string(&open).unwrap();
    assert_eq!(encoded, r#"{"start":9223372036854775807,"end":null}"#);
    assert_eq!(serde_json::from_str::<Interval>(&encoded).unwrap(), open);
    assert!(
        serde_json::from_str::<ScalarValue>(r#"{"type":"interval","value":{"start":5,"end":4}}"#)
            .is_err()
    );
}

#[test]
fn half_open_relations_distinguish_touching_precedence_containment_and_intersection() {
    let a = Interval::new(-3, Some(2)).unwrap();
    let b = Interval::new(2, Some(5)).unwrap();
    let c = Interval::new(3, None).unwrap();
    assert!(a.meets(b));
    assert!(!a.before(b));
    assert!(a.before(c));
    assert!(b.overlaps(c));
    assert!(!a.overlaps(b));
    assert_eq!(
        b.intersection(c).unwrap(),
        Interval::new(3, Some(5)).unwrap()
    );
    assert_eq!(c.intersection(b), b.intersection(c));
    assert!(a.within(a));
    assert!(!c.within(b));
    assert!(a.contains(-3));
    assert!(!a.contains(2));
}

#[test]
fn source_typed_callbacks_partial_values_and_literals_preserve_intervals() {
    let source = r#"function Window revision "1" (time start, time end) returns interval { return interval(param start,param end); }
function Identity revision "1" (interval input) returns interval { return param input; }
function Through revision "1" (interval input,function callback(interval input) returns interval) returns interval {
 apply Kept from callback { interval input param input; } return value Kept;
}
apply FromZero from Window { time start 0; }
apply Ten from FromZero { time end 10; }
apply ThroughIdentity from Through { function callback Identity; }
apply Selected from ThroughIdentity { interval input value Ten; }
value Start interval_start(value Selected);
value End interval_end(value Selected);
value Inside interval_contains(value Selected,time 9);
value Outside interval_contains(value Selected,time 10);
graph G { node "n" entity "e" space "s" property "window" value Selected;
 attachment "window" on graph key "window" literal value Selected valid 0 until infinity;
}"#;
    let output = compile_artifacts(source).unwrap();
    assert_eq!(output.values["Ten"], output.values["Selected"]);
    assert_eq!(output.values["Start"], ScalarValue::Time(0));
    assert_eq!(output.values["End"], ScalarValue::Time(10));
    assert_eq!(output.values["Inside"], ScalarValue::Boolean(true));
    assert_eq!(output.values["Outside"], ScalarValue::Boolean(false));
    assert_eq!(
        output.values["Selected"].json(),
        serde_json::json!({"start":0,"end":10})
    );
    let weave_contract::Command::Commit { data, .. } = &output.program.commands[0] else {
        panic!()
    };
    assert_eq!(
        data.nodes[0].properties["window"],
        output.values["Selected"].json()
    );
    assert!(
        matches!(&data.attachments[0].value,weave_contract::MetadataValue::Literal{value} if value==&output.values["Selected"].json())
    );
    let formatted = weave_language::format_source(source).unwrap();
    assert_eq!(
        compile_artifacts(&formatted)
            .unwrap()
            .fingerprint()
            .unwrap(),
        output.fingerprint().unwrap()
    );
    assert_eq!(
        weave_language::format_source(&formatted).unwrap(),
        formatted
    );
}

#[test]
fn unused_constant_failures_are_checked_without_evaluating_symbolic_parameters() {
    for (source, code) in [
        ("value I interval(time 3,time 3);", "E_INTERVAL_BOUNDS"),
        (
            "value I interval_end(interval_open(time 3));",
            "E_INTERVAL_UNBOUNDED",
        ),
        (
            "value I interval_intersection(interval(time 0,time 1),interval(time 1,time 2));",
            "E_INTERVAL_EMPTY",
        ),
        ("value I interval(0,time 1);", "E_SCALAR_TYPE"),
        (
            "value I interval_contains(interval_open(time 0),0);",
            "E_SCALAR_TYPE",
        ),
        (
            "function F revision \"1\" (time input) returns interval { return interval_intersection(interval_open(param input),interval(time 4,time 3)); }",
            "E_INTERVAL_BOUNDS",
        ),
        (
            "function F revision \"1\" (interval input) returns time { value Bad interval_end(interval_open(time 0)); return interval_start(param input); }",
            "E_INTERVAL_UNBOUNDED",
        ),
    ] {
        assert_eq!(
            compile_artifacts(source).unwrap_err().code,
            code,
            "{source}"
        );
    }
    let symbolic = r#"function Intersect revision "1" (interval input,interval other) returns interval {
 return interval_intersection(param input,param other);
}"#;
    assert!(
        compile_artifacts(symbolic)
            .unwrap()
            .program
            .commands
            .is_empty()
    );
    let invalid = format!(
        "{symbolic} apply R from Intersect {{ interval input interval(time 0,time 2); interval other interval(time 2,time 3); }}"
    );
    let error = compile_artifacts(&invalid).unwrap_err();
    assert_eq!(error.code, "E_INTERVAL_EMPTY");
    assert!(!error.trace.is_empty());
}

#[test]
fn imported_interval_callbacks_keep_identity_and_original_error_spans() {
    use weave_language::modules::{SourceModule, content_digest, link};
    let source = r#"module "windows" revision "1";
function Keep revision "1" (interval input) returns interval { return param input; }
function Via revision "1" (interval input,function callback(interval input) returns interval) returns interval {
 apply R from callback { interval input param input; } return value R;
}"#;
    let units = [SourceModule {
        id: "windows",
        revision: "1",
        source,
    }];
    let entry = format!(
        "import w module \"windows\" revision \"1\" sha256 {:?}; apply R from w::Via {{function callback w::Keep; interval input interval(time 0,time 10);}}",
        content_digest(source)
    );
    let a = link("entry", &entry, &units)
        .unwrap()
        .compile_artifacts()
        .unwrap();
    let b = link(
        "entry",
        &entry
            .replace("import w ", "import renamed ")
            .replace("w::", "renamed::"),
        &units,
    )
    .unwrap()
    .compile_artifacts()
    .unwrap();
    assert_eq!(a.fingerprint().unwrap(), b.fingerprint().unwrap());
    assert_eq!(
        a.values["R"].json(),
        serde_json::json!({"start":0,"end":10})
    );
    assert!(a.program.commands.is_empty());
    let broken = "module \"windows\" revision \"1\";\n// λ preserves byte offsets\nfunction Bad revision \"1\" (interval input) returns interval { return interval(time 2,time 1); }";
    let entry = format!(
        "import w module \"windows\" revision \"1\" sha256 {:?};",
        content_digest(broken)
    );
    let linked = link(
        "entry",
        &entry,
        &[SourceModule {
            id: "windows",
            revision: "1",
            source: broken,
        }],
    )
    .unwrap();
    let error = linked.compile_artifacts().unwrap_err();
    assert_eq!(error.code, "E_INTERVAL_BOUNDS");
    assert_eq!(error.source_id, "module:windows@1");
    assert!(broken[error.start..error.end].starts_with("interval("));
    assert_eq!(error.import_trace.len(), 1);
}

#[test]
fn intervals_do_not_erase_schema_or_callback_obligations() {
    for scalar in ["string", "integer", "boolean", "decimal", "float"] {
        let source = format!(
            "value I interval(time 0,time 1); schema S revision \"1\" {{node N space \"s\" {{property \"x\" {scalar} required;}}}} graph G schema S {{node \"n\" type N entity \"n\" space \"s\" property \"x\" value I;}}"
        );
        assert_eq!(
            compile_artifacts(&source).unwrap_err().code,
            "E_SCALAR_TYPE"
        );
    }
    let mismatch = r#"function Keep revision "1" (time input) returns time {return param input;}
function Via revision "1" (function callback(interval input) returns interval) returns interval {
 apply R from callback {interval input interval(time 0,time 1);} return value R;
}
apply R from Via {function callback Keep;}"#;
    assert!(compile_artifacts(mismatch).is_err());
    let result = compile_artifacts(include_str!("../examples/intervals.weave")).unwrap();
    assert_eq!(result.program.version, "0.19.0");
    assert!(result.handler_templates.is_empty() && result.view_templates.is_empty());
}
