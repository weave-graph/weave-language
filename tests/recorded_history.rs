use weave_contract::{Command, GraphExpression, RecordedSelection};
use weave_language::{compile, compile_artifacts};

#[test]
fn recorded_handles_defer_reads_and_pin_once_with_independent_valid_time() {
    let source = r#"recorded_handle H graph "Facts" branch "offline" known_at 20;
pin Historical from H at 7 metadata depth 2;
lens Copy from Historical {}"#;
    assert!(
        compile(r#"recorded_handle H graph "Facts" branch "offline" known_at 20;"#)
            .unwrap()
            .commands
            .is_empty()
    );
    let plan = compile(source).unwrap();
    assert_eq!(plan.version, weave_contract::VERSION);
    assert_eq!(plan.commands.len(), 2);
    let Command::Bind {
        value: GraphExpression::RecordedQuery { query, selection },
        ..
    } = &plan.commands[0]
    else {
        panic!("recorded read missing");
    };
    assert_eq!(selection, &RecordedSelection::LocalTime { unix_millis: 20 });
    assert_eq!(query.graph_id, "Facts");
    assert_eq!(query.branch_id, "offline");
    assert_eq!(query.valid_at, Some(7));
    assert!(query.revision.is_none());
    assert!(query.include_metadata);
    assert_eq!(query.max_depth, 2);
    assert!(
        matches!(&plan.commands[1], Command::Bind { value: GraphExpression::Filter { input, .. }, .. } if matches!(input.as_ref(), GraphExpression::Reference { name } if name=="Historical"))
    );
}

#[test]
fn checkpoint_observer_is_explicit_and_external_selectors_are_literal() {
    let plan = compile(
        r#"recorded_handle H graph "alias::Facts" branch "alias::main"
checkpoint "observation:exact" observer "urn:weave:replica:local";
pin Historical from H;"#,
    )
    .unwrap();
    let Command::Bind {
        value: GraphExpression::RecordedQuery { query, selection },
        ..
    } = &plan.commands[0]
    else {
        panic!("recorded read missing");
    };
    assert_eq!(query.graph_id, "alias::Facts");
    assert_eq!(query.branch_id, "alias::main");
    assert_eq!(
        selection,
        &RecordedSelection::Checkpoint {
            checkpoint: "observation:exact".into(),
            observer: "urn:weave:replica:local".into()
        }
    );
    assert!(query.valid_at.is_none());
}

#[test]
fn selectors_are_bounded_and_pure_functions_cannot_hide_recorded_reads() {
    for source in [
        r#"recorded_handle H graph "G" branch "main" known_at -1;"#,
        r#"recorded_handle H graph "G" branch "main" known_at 9223372036854775808;"#,
        r#"recorded_handle H graph "G" known_at 20;"#,
        r#"recorded_handle H graph "G" branch "main" checkpoint "c";"#,
        r#"recorded_handle H graph "G" branch "main" known_at 20 checkpoint "c" observer "o";"#,
        r#"recorded_handle H graph "G" branch "main" known_at 20; lens Bad from H {}"#,
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let source = r#"function Hidden revision "1" (graph input) {
recorded_handle H graph "G" branch "main" known_at 20; return input;
}"#;
    assert_eq!(compile(source).unwrap_err().code, "E_FUNCTION_EFFECT");
}

#[test]
fn recorded_view_artifact_keeps_selector_identity_and_fact_clock_separate() {
    let source = r#"recorded_handle H graph "G" branch "main" known_at 20;
view_template V revision "1" from H clock tick { at 7; }"#;
    let artifacts = compile_artifacts(source).unwrap();
    assert!(artifacts.program.commands.is_empty());
    let template = &artifacts.view_templates["V"];
    let GraphExpression::RecordedQuery { query, selection } = &template.expression else {
        panic!("selector missing");
    };
    assert_eq!(query.valid_at, Some(7));
    assert_eq!(selection, &RecordedSelection::LocalTime { unix_millis: 20 });
    assert_ne!(
        template.definition_digest,
        compile_artifacts(&source.replace("known_at 20", "known_at 21"))
            .unwrap()
            .view_templates["V"]
            .definition_digest
    );
    weave_contract::view_registration::validate_template(template).unwrap();
    assert_eq!(
        compile(source).unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
}
