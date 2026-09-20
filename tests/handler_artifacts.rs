use weave_contract::{GraphExpression, HandlerEventType};
use weave_language::{
    compile_artifacts,
    modules::{SourceModule, content_digest, link},
};

const FUNCTION: &str = r#"function Select revision "1" (graph input, string relation) {
  lens Selected from input { match relation param relation; }
  return Selected;
}
apply WarningRecipe from Select { string relation "warning"; }"#;
const HANDLER: &str = r#"handler Warning revision "1" using WarningRecipe {
 input event graph "Installation" branch "main" metadata depth 4;
 on "graph.committed", "graph.accepted";
 output slot "warnings";
 replay pinned;
}"#;
fn source() -> String {
    format!("{FUNCTION}\n{HANDLER}")
}

#[test]
fn recipe_is_seeded_only_by_event_and_complete_outputs_keep_artifacts() {
    let source = source();
    let a = compile_artifacts(&source).unwrap();
    assert!(a.program.commands.is_empty());
    assert!(a.values.is_empty());
    let h = &a.handler_templates["Warning"];
    weave_contract::handler_registration::validate_handler_template(h).unwrap();
    assert_eq!(
        h.event_types,
        vec![
            HandlerEventType::GraphAccepted,
            HandlerEventType::GraphCommitted
        ]
    );
    assert_eq!(h.input.graph_id, "Installation");
    assert_eq!(h.output_slot, "warnings");
    assert!(
        matches!(&h.recipe.bindings[0].value, GraphExpression::Reference {name} if name == "$event")
    );
    assert!(
        h.source_revisions
            .iter()
            .any(|r| r.name.starts_with("weave:handler-template:"))
    );
    for result in [
        weave_language::compile(&source).map(|_| ()),
        weave_language::specialize(&source).map(|_| ()),
        weave_language::fingerprint(&source).map(|_| ()),
        weave_language::describe(&source).map(|_| ()),
    ] {
        assert_eq!(result.unwrap_err().code, "E_HOST_ARTIFACT_REQUIRED");
    }
    let linked = link("entry", &source, &[]).unwrap();
    assert_eq!(
        linked.compile().unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
    assert_eq!(
        linked.specialize().unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
    assert_eq!(
        linked.fingerprint().unwrap_err().code,
        "E_HOST_ARTIFACT_REQUIRED"
    );
    assert_eq!(
        linked.compile_artifacts().unwrap().handler_templates["Warning"],
        *h
    );
}

#[test]
fn normalized_identity_keeps_selectors_and_ignores_presentation_or_prior_expansion() {
    let a = compile_artifacts(&source()).unwrap();
    let formatted = weave_language::format_source(&format!("// comment\n{}", source())).unwrap();
    assert_eq!(
        a.fingerprint().unwrap(),
        compile_artifacts(&formatted)
            .unwrap()
            .fingerprint()
            .unwrap()
    );
    let changed_order = source().replace(
        "\"graph.committed\", \"graph.accepted\"",
        "\"graph.accepted\", \"graph.committed\"",
    );
    assert_eq!(
        a.fingerprint().unwrap(),
        compile_artifacts(&changed_order)
            .unwrap()
            .fingerprint()
            .unwrap()
    );
    let extra = format!(
        "{FUNCTION} graph Empty {{}} apply Earlier from WarningRecipe {{ graph input Empty; }} {HANDLER}"
    );
    assert_eq!(
        a.handler_templates["Warning"],
        compile_artifacts(&extra).unwrap().handler_templates["Warning"]
    );
    for changed in [
        source().replace("Installation", "Other"),
        source().replace("depth 4", "depth 3"),
        source().replace("slot \"warnings\"", "slot \"other\""),
        source().replace("relation \"warning\"", "relation \"alert\""),
    ] {
        assert_ne!(
            a.handler_templates["Warning"].definition_digest,
            compile_artifacts(&changed).unwrap().handler_templates["Warning"].definition_digest
        );
    }
}

#[test]
fn captures_signatures_and_authored_fake_events_are_rejected() {
    let captured = format!(
        r#"function Pair revision "1" (graph input, graph other) {{ union R from input with other; return R; }}
graph G {{}}
apply WarningRecipe from Pair {{ graph other G; }} {HANDLER}"#
    );
    assert_eq!(
        compile_artifacts(&captured).unwrap_err().code,
        "E_HANDLER_CAPTURE"
    );
    let missing = format!(
        "{FUNCTION} {}",
        HANDLER.replace("using WarningRecipe", "using Select")
    );
    assert_eq!(
        compile_artifacts(&missing).unwrap_err().code,
        "E_HANDLER_SIGNATURE"
    );
    for changed in [
        source().replace("graph.accepted", "MetaGraphRebound"),
        source().replace("graph.accepted", "graph.committed"),
        source().replace("\"graph.committed\", ", ""),
    ] {
        assert_eq!(
            compile_artifacts(&changed).unwrap_err().code,
            "E_HANDLER_EVENT"
        );
    }
    assert_eq!(
        compile_artifacts(&source().replace("depth 4", "depth 9"))
            .unwrap_err()
            .code,
        "E_HANDLER_INPUT"
    );
    let unused =
        format!(r#"function Never revision "1" (graph input) {{ {HANDLER} return input; }}"#);
    assert_eq!(
        compile_artifacts(&unused).unwrap_err().code,
        "E_FUNCTION_EFFECT"
    );
}

#[test]
fn higher_order_scalar_specialization_and_rule_declarations_remain_pure() {
    let s = format!(
        r#"rules W revision "1" {{ rule Warn {{ when "bad"(x, y); yield "warning"(x, y); }} }}
function Infer revision "1" (graph input) {{ reason R from input using W; return R; }}
function Transform revision "1" (graph input, function callback) {{ apply R from callback {{ graph input input; }} return R; }}
apply WarningRecipe from Transform {{ function callback Infer; }} {HANDLER}"#
    );
    let a = compile_artifacts(&s).unwrap();
    assert!(a.program.commands.is_empty());
    assert!(
        a.handler_templates["Warning"]
            .recipe
            .bindings
            .iter()
            .any(|b| matches!(&b.value, GraphExpression::Reason { .. }))
    );
    weave_contract::handler_registration::validate_handler_template(
        &a.handler_templates["Warning"],
    )
    .unwrap();
}

#[test]
fn linked_recipes_preserve_pinned_module_manifests_and_literal_selectors() {
    let m = format!(r#"module "tools" revision "1"; {FUNCTION}"#);
    // Imported modules contain pure definitions only; partial application is an entry operation.
    let m = m.split("apply WarningRecipe").next().unwrap();
    let entry = |alias: &str| {
        format!(
            r#"import {alias} module "tools" revision "1" sha256 "{}";
apply WarningRecipe from {alias}::Select {{ string relation "alias::warning"; }} {}"#,
            content_digest(m),
            HANDLER
                .replace("Installation", "alias::Installation")
                .replace("\"main\"", "\"alias::main\"")
                .replace("slot \"warnings\"", "slot \"alias::warnings\"")
        )
    };
    let units = [SourceModule {
        id: "tools",
        revision: "1",
        source: m,
    }];
    let a = link("entry", &entry("first"), &units)
        .unwrap()
        .compile_artifacts()
        .unwrap();
    let b = link("entry", &entry("renamed"), &units)
        .unwrap()
        .compile_artifacts()
        .unwrap();
    assert_eq!(a.fingerprint().unwrap(), b.fingerprint().unwrap());
    let h = &a.handler_templates["Warning"];
    assert_eq!(h.input.graph_id, "alias::Installation");
    assert_eq!(h.input.branch_id, "alias::main");
    assert_eq!(h.output_slot, "alias::warnings");
    assert!(
        h.source_revisions
            .iter()
            .any(|r| r.digest == format!("sha256:{}", content_digest(m)))
    );
    weave_contract::handler_registration::validate_handler_template(h).unwrap();
}

#[test]
fn combined_host_artifact_budget_and_legacy_shape_are_explicit() {
    let plain = compile_artifacts("value Answer 42;").unwrap();
    assert!(
        serde_json::to_value(&plain)
            .unwrap()
            .get("handler_templates")
            .is_none()
    );
    let mut s = source();
    s.push_str("live_handle Head graph \"G\" branch \"main\";");
    for i in 0..16 {
        s.push_str(&format!(
            "view_template V{i} revision \"1\" from Head clock fixed {{}}"
        ));
    }
    assert_eq!(compile_artifacts(&s).unwrap_err().code, "E_BUDGET");
}

#[test]
fn hidden_reads_and_imported_effects_keep_original_module_locations() {
    let module = r#"module "contexts" revision "1";
context_schema World revision "1" { axis "label" string; }
function Read revision "1" (graph input) {
 typed_context Selected from input graph "C" revision "r" schema World;
 return Selected;
}"#;
    let entry = format!(
        r#"import c module "contexts" revision "1" sha256 "{}"; {}"#,
        content_digest(module),
        HANDLER.replace("using WarningRecipe", "using c::Read")
    );
    let error = match link(
        "entry",
        &entry,
        &[SourceModule {
            id: "contexts",
            revision: "1",
            source: module,
        }],
    ) {
        Err(error) => error,
        Ok(_) => panic!("imported hidden read accepted"),
    };
    assert_eq!(error.code, "E_FUNCTION_EFFECT");
    assert_eq!(error.source_id, "module:contexts@1");
    assert_eq!(&module[error.start..error.end], "Selected");
    let effect = format!(
        r#"module "effects" revision "1"; function WarningRecipe revision "1" (graph input) {{ return input; }} {HANDLER}"#
    );
    let entry = format!(
        r#"import e module "effects" revision "1" sha256 "{}";"#,
        content_digest(&effect)
    );
    assert!(
        link(
            "entry",
            &entry,
            &[SourceModule {
                id: "effects",
                revision: "1",
                source: &effect
            }]
        )
        .is_err()
    );
}

#[test]
fn schema_obligations_and_nested_graph_captures_are_not_erased() {
    let constrained = format!(
        r#"schema Exact revision "1" {{ node N space "s" {{}} }}
function WarningRecipe revision "1" (graph input schema Exact) returns graph schema Exact {{ return input; }} {HANDLER}"#
    );
    assert_eq!(
        compile_artifacts(&constrained).unwrap_err().code,
        "E_HANDLER_SCHEMA"
    );
    let nested = format!(
        r#"function Pair revision "1" (graph input, graph other) {{ union R from input with other; return R; }}
graph G {{}}
apply Bound from Pair {{ graph other G; }}
function Wrapper revision "1" (graph input) {{ apply R from Bound {{ graph input input; }} return R; }}
{}"#,
        HANDLER.replace("using WarningRecipe", "using Wrapper")
    );
    assert_eq!(
        compile_artifacts(&nested).unwrap_err().code,
        "E_HANDLER_CAPTURE"
    );
    let wrong = format!(
        r#"function WarningRecipe revision "1" (integer input) returns integer {{ return param input; }} {HANDLER}"#
    );
    assert_eq!(
        compile_artifacts(&wrong).unwrap_err().code,
        "E_HANDLER_SIGNATURE"
    );
}

#[test]
fn expanded_handler_binding_limit_is_explicit_and_shared_artifacts_remain_complete() {
    let mut body = String::new();
    for i in 0..260 {
        body.push_str(&format!("lens L{i} from input {{}}\n"));
    }
    let source = format!(
        r#"function WarningRecipe revision "1" (graph input) {{ {body} return L259; }} {HANDLER}"#
    );
    assert_eq!(compile_artifacts(&source).unwrap_err().code, "E_BUDGET");
    let source = format!(
        "{} value Answer 42; live_handle Head graph \"Installation\" branch \"main\"; view_template Visible revision \"1\" from Head clock fixed {{}}",
        self::source()
    );
    let complete = compile_artifacts(&source).unwrap();
    assert!(complete.program.commands.is_empty());
    assert_eq!(complete.handler_templates.len(), 1);
    assert_eq!(complete.view_templates.len(), 1);
    assert_eq!(complete.values["Answer"].json(), 42);
    assert!(complete.fingerprint().is_ok());
}
