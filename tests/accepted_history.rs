use weave_contract::*;
use weave_language::{
    compile, format_source,
    modules::{SourceModule, content_digest, link},
};
#[test]
fn local_accepted_time_and_exact_decision_are_literal_pinned_reads() {
    let plan = compile(
        r#"accepted_history Old view "team" accepted_at 25 at 7;
accepted_history Exact view "team" decision "decision:old" observer "local";
lens Copy from Old {}"#,
    )
    .unwrap();
    assert_eq!(plan.commands.len(), 3);
    let Command::Bind {
        value: GraphExpression::Filter {
            input, valid_at, ..
        },
        ..
    } = &plan.commands[0]
    else {
        panic!("filter missing")
    };
    assert_eq!(*valid_at, Some(7));
    assert!(
        matches!(input.as_ref(),GraphExpression::AcceptedHistory {view_id,selection:AcceptedSelection::LocalTime {unix_millis:25}} if view_id=="team")
    );
    let Command::Bind {
        value:
            GraphExpression::AcceptedHistory {
                selection:
                    AcceptedSelection::Decision {
                        observer,
                        decision_id,
                    },
                ..
            },
        ..
    } = &plan.commands[1]
    else {
        panic!("exact cut missing")
    };
    assert_eq!(observer, "local");
    assert_eq!(decision_id, "decision:old");
    assert!(
        matches!(&plan.commands[2],Command::Bind {value:GraphExpression::Reference {name},..} if name=="Old")
    );
}
#[test]
fn ranges_are_named_complete_half_open_collections_with_separate_fact_time() {
    let source = r#"recorded_range R graph "alias::Facts" branch "offline" observer "local" between 5 and 20 limit 100 at 7;
accepted_range A view "alias::team" observer "local" between 10 and 50 limit 20 at 8;"#;
    let plan = compile(source).unwrap();
    assert_eq!(compile(&format_source(source).unwrap()).unwrap(), plan);
    let Command::RecordedRange {
        query,
        observer,
        interval,
        limit,
        name,
    } = &plan.commands[0]
    else {
        panic!("range missing")
    };
    assert_eq!(name, "R");
    assert_eq!(query.graph_id, "alias::Facts");
    assert_eq!(query.branch_id, "offline");
    assert_eq!(query.valid_at, Some(7));
    assert_eq!(observer, "local");
    assert_eq!(
        interval,
        &Interval {
            start: 5,
            end: Some(20)
        }
    );
    assert_eq!(*limit, 100);
    assert!(
        matches!(&plan.commands[1],Command::AcceptedRange {view_id,valid_at:Some(8),..} if view_id=="alias::team")
    );
}
#[test]
fn invalid_cuts_ranges_and_hidden_reads_fail_before_runtime() {
    for source in [
        r#"accepted_history X view "team" accepted_at -1;"#,
        r#"accepted_history X view "team" decision "d";"#,
        r#"recorded_range R graph "G" branch "main" observer "local" between 10 and 10 limit 1;"#,
        r#"accepted_range R view "team" observer "local" between 0 and 20 limit 1001;"#,
        r#"accepted_range R view "team" observer "local" between 0 and 20 limit -1;"#,
        r#"accepted_history X view "team" accepted_at 9223372036854775808;"#,
        r#"recorded_range R graph "G" branch "main" observer "local" between 0 and 20 limit 1; lens Bad from R {}"#,
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    for effect in [
        r#"accepted_history X view "team" accepted_at 10;"#,
        r#"accepted_range R view "team" observer "local" between 0 and 20 limit 1;"#,
    ] {
        let source =
            format!("function Hidden revision \"1\" (graph input) {{{effect} return input;}}");
        assert_eq!(compile(&source).unwrap_err().code, "E_FUNCTION_EFFECT");
        let module = format!("module \"bad\" revision \"1\"; {effect}");
        let entry = format!(
            "import unused module \"bad\" revision \"1\" sha256 {:?};",
            content_digest(&module)
        );
        let error = match link(
            "entry",
            &entry,
            &[SourceModule {
                id: "bad",
                revision: "1",
                source: &module,
            }],
        ) {
            Err(e) => e,
            Ok(_) => panic!("hidden module effect accepted"),
        };
        assert_eq!(error.code, "E_MODULE_EFFECT");
        assert_eq!(error.source_id, "module:bad@1");
    }
}
