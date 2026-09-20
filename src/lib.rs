//! Experimental, deterministic front end for the Weave graph language.
//! No source program can invoke host effects: compilation only emits a plan.
pub mod context_axes;
pub mod decimal;
mod format;
mod functions;
mod graph_types;
pub mod modules;
pub mod scalars;
pub mod syntax;
pub use format::format_source;
use std::collections::{BTreeMap, BTreeSet};
use syntax::{AlgebraOperation, BindingValue, Item, Metadata, Statement, StringExpr, TimeExpr};
pub use syntax::{Diagnostic, parse};
pub use weave_contract::quantity;
use weave_contract::{
    Assertion, GraphProfile, MetadataAttachment, MetadataHost, SnapshotCommit, StructuralEdge,
};
use weave_contract::{
    Command, Edge, GraphData, GraphExpression, GraphRef, GraphSchema, Interval, JoinMatch, Node,
    Polarity, Program, QueryPlan, VERSION,
};

fn diagnostic(code: &str, message: impl Into<String>, span: syntax::Span) -> Diagnostic {
    Diagnostic::new(code, message, span.0, span.1)
}
fn refs(metadata: &[Metadata]) -> Vec<GraphRef> {
    metadata
        .iter()
        .map(|m| GraphRef {
            graph_id: m.graph.clone(),
            revision: m.revision.clone(),
        })
        .collect()
}
fn base_query(graph_id: String, revision: Option<String>) -> QueryPlan {
    QueryPlan {
        graph_id,
        revision,
        branch_id: "main".into(),
        predicate: None,
        from: None,
        to: None,
        valid_at: None,
        include_metadata: false,
        max_depth: 8,
    }
}
#[derive(Clone)]
struct Lens {
    query: QueryPlan,
    input: Option<GraphExpression>,
    typed: Option<bool>,
    relation: Option<String>,
    time: Option<String>,
}
impl Lens {
    fn concrete(query: QueryPlan) -> Self {
        Self {
            query,
            input: None,
            typed: None,
            relation: None,
            time: None,
        }
    }
    fn expression(&self) -> GraphExpression {
        match &self.input {
            Some(input) if self.query.predicate.is_some() || self.query.valid_at.is_some() => {
                GraphExpression::Filter {
                    input: Box::new(input.clone()),
                    predicate: self.query.predicate.clone(),
                    valid_at: self.query.valid_at,
                }
            }
            Some(input) => input.clone(),
            None => GraphExpression::Query {
                query: self.query.clone(),
            },
        }
    }
    fn emit(&mut self, name: &str, commands: &mut Vec<Command>) {
        if self.relation.is_none() && self.time.is_none() {
            commands.push(Command::Bind {
                name: name.into(),
                value: self.expression(),
            });
            self.input = Some(GraphExpression::Reference { name: name.into() });
        }
    }
}
fn emit_snapshot(
    commit: SnapshotCommit,
    names: &mut BTreeMap<String, Lens>,
    commands: &mut Vec<Command>,
    active_batch: &mut Option<(String, usize, Vec<SnapshotCommit>)>,
) {
    let mut query = base_query(commit.graph_id.clone(), None);
    query.branch_id = commit.branch_id.clone();
    let mut lens = Lens::concrete(query);
    lens.typed = Some(commit.data.schema.is_some());
    names.insert(commit.graph_id.clone(), lens);
    if let Some((_, remaining, members)) = active_batch {
        members.push(commit);
        *remaining -= 1;
        if *remaining == 0 {
            let (batch_id, _, commits) = active_batch.take().unwrap();
            commands.push(Command::CommitBatch { batch_id, commits });
        }
    } else {
        commands.push(Command::Commit {
            graph_id: commit.graph_id,
            branch_id: commit.branch_id,
            expected_head: commit.expected_head,
            data: commit.data,
        });
    }
}
/// Compile proposed surface syntax into the shared, versioned engine protocol.
/// Declarations are sequential. A lens can compose earlier lenses by adding
/// compatible filters. Graph declarations emit new-branch snapshot commits.
pub fn compile(source: &str) -> Result<Program, Diagnostic> {
    compile_parsed(parse(source)?)
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct SpecializedProgram {
    pub program: Program,
    pub values: BTreeMap<String, scalars::ScalarValue>,
}
impl SpecializedProgram {
    pub fn fingerprint(&self) -> Result<String, Diagnostic> {
        weave_contract::identity::source_fingerprint(&serde_json::json!({"profile":"weave-source-specialization-v1","program":self.program,"values":self.values})).map_err(|e|diagnostic(&e.code,e.message,(0,0)))
    }
}
pub fn specialize(source: &str) -> Result<SpecializedProgram, Diagnostic> {
    specialize_parsed(parse(source)?)
}
pub(crate) fn compile_parsed(parsed: syntax::Program) -> Result<Program, Diagnostic> {
    Ok(specialize_parsed(parsed)?.program)
}
pub(crate) fn specialize_parsed(parsed: syntax::Program) -> Result<SpecializedProgram, Diagnostic> {
    reject_artifacts(&parsed)?;
    let artifacts = compile_artifacts_parsed(parsed)?;
    Ok(SpecializedProgram {
        program: artifacts.program,
        values: artifacts.values,
    })
}
pub(crate) fn reject_artifacts(parsed: &syntax::Program) -> Result<(), Diagnostic> {
    if let Some(Statement::ViewTemplate { name_span, .. }) = parsed
        .statements
        .iter()
        .find(|s| matches!(s, Statement::ViewTemplate { .. }))
    {
        return Err(diagnostic(
            "E_HOST_ARTIFACT_REQUIRED",
            "View templates require the complete artifact API or view-plan",
            *name_span,
        ));
    }
    Ok(())
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct CompiledArtifacts {
    pub program: Program,
    pub values: BTreeMap<String, scalars::ScalarValue>,
    pub view_templates: BTreeMap<String, weave_contract::CompiledViewTemplate>,
}
impl CompiledArtifacts {
    pub fn fingerprint(&self) -> Result<String, Diagnostic> {
        weave_contract::identity::source_fingerprint(
            &serde_json::json!({"profile":"weave-compiled-artifacts-v1","artifacts":self}),
        )
        .map_err(|e| diagnostic(&e.code, e.message, (0, 0)))
    }
}
pub fn compile_artifacts(source: &str) -> Result<CompiledArtifacts, Diagnostic> {
    compile_artifacts_parsed(parse(source)?)
}
pub(crate) fn compile_artifacts_parsed(
    parsed: syntax::Program,
) -> Result<CompiledArtifacts, Diagnostic> {
    for statement in &parsed.statements {
        if let Statement::Import { name_span, .. } | Statement::ModuleHeader { name_span, .. } =
            statement
        {
            return Err(diagnostic(
                "E_MODULE_RESOLUTION",
                "Module units require the explicit supplied-source linking API",
                *name_span,
            ));
        }
    }
    let source_revisions = functions::source_revisions(&parsed)?;
    let (ast, scalar_values) = functions::expand(parsed)?;
    let mut names: BTreeMap<String, Lens> = BTreeMap::new();
    let mut commands = Vec::new();
    let mut view_templates = BTreeMap::new();
    let mut artifact_charge = 0usize;
    let mut declared = BTreeSet::new();
    let mut schemas: BTreeMap<String, GraphSchema> = BTreeMap::new();
    let mut rule_sets = BTreeMap::new();
    let mut context_schemas = BTreeMap::new();
    let mut handles = BTreeMap::new();
    let mut statements = Vec::new();
    let mut batches = BTreeMap::new();
    let mut batch_names = BTreeSet::new();
    for statement in ast.statements {
        match statement {
            Statement::Transaction {
                name,
                name_span,
                body,
            } => {
                if !batch_names.insert(name.clone()) {
                    return Err(diagnostic(
                        "E_DUPLICATE",
                        "Duplicate transaction ID",
                        name_span,
                    ));
                }
                batches.insert(statements.len(), (name, body.len()));
                statements.extend(body);
            }
            other => statements.push(other),
        }
    }
    let mut active_batch: Option<(String, usize, Vec<SnapshotCommit>)> = None;
    for (statement_index, statement) in statements.into_iter().enumerate() {
        if let Some((name, count)) = batches.remove(&statement_index) {
            active_batch = Some((name, count, Vec::new()));
        }

        if let Statement::ContextSchema {
            name,
            name_span,
            definition,
        } = statement
        {
            if context_schemas.insert(name, definition).is_some() {
                return Err(diagnostic(
                    "E_DUPLICATE",
                    "Duplicate context schema",
                    name_span,
                ));
            }
            continue;
        }
        if let Statement::Rules {
            name,
            name_span,
            definition,
        } = statement
        {
            if rule_sets.insert(name, definition).is_some() {
                return Err(diagnostic(
                    "E_DUPLICATE",
                    "Duplicate rule module",
                    name_span,
                ));
            }
            continue;
        }
        if let Statement::Schema {
            name,
            name_span,
            definition,
        } = statement
        {
            if schemas.contains_key(&name) {
                return Err(diagnostic(
                    "E_DUPLICATE",
                    format!("Duplicate schema '{name}'"),
                    name_span,
                ));
            }
            let empty = GraphData {
                schema: Some(definition.clone()),
                ..GraphData::default()
            };
            if let Some(error) = weave_contract::validate_schema_graph(&empty).first() {
                return Err(diagnostic(&error.code, &error.message, name_span));
            }
            schemas.insert(name, definition);
            continue;
        }
        let (name, name_span) = match &statement {
            Statement::ViewTemplate {
                name, name_span, ..
            }
            | Statement::LiveHandle {
                name, name_span, ..
            }
            | Statement::Pin {
                name, name_span, ..
            }
            | Statement::ContextValue {
                name, name_span, ..
            }
            | Statement::TypedContext {
                name, name_span, ..
            }
            | Statement::NativeService {
                name, name_span, ..
            }
            | Statement::Reason {
                name, name_span, ..
            }
            | Statement::Algebra {
                name, name_span, ..
            }
            | Statement::Graph {
                name, name_span, ..
            }
            | Statement::Use {
                name, name_span, ..
            }
            | Statement::Lens {
                name, name_span, ..
            }
            | Statement::Bind {
                name, name_span, ..
            }
            | Statement::Join {
                name, name_span, ..
            }
            | Statement::Metadata {
                name, name_span, ..
            } => (name, *name_span),
            Statement::ModuleHeader { .. }
            | Statement::Import { .. }
            | Statement::ContextSchema { .. }
            | Statement::Schema { .. }
            | Statement::Transaction { .. }
            | Statement::Value { .. }
            | Statement::Function { .. }
            | Statement::Apply { .. }
            | Statement::Rules { .. } => unreachable!(),
        };
        if !declared.insert(name.clone()) {
            return Err(diagnostic(
                "E_DUPLICATE",
                format!("Duplicate graph or lens '{name}'"),
                name_span,
            ));
        }
        match statement {
            Statement::LiveHandle {
                name,
                graph,
                branch,
                ..
            } => {
                let mut query = base_query(graph, None);
                query.branch_id = branch;
                handles.insert(name, query);
            }
            Statement::ViewTemplate {
                name,
                name_span,
                revision,
                source,
                source_span,
                clock,
                predicate,
                valid_at,
            } => {
                let mut query = handles.get(&source).cloned().ok_or_else(|| {
                    diagnostic(
                        "E_VIEW_TEMPLATE",
                        "Template requires a declared live handle, not a captured graph value",
                        source_span,
                    )
                })?;
                query.predicate = match predicate {
                    None => None,
                    Some(StringExpr::Literal(v)) => Some(v),
                    _ => {
                        return Err(diagnostic(
                            "E_VIEW_TEMPLATE",
                            "Template relation must be concrete",
                            name_span,
                        ));
                    }
                };
                query.valid_at = match valid_at {
                    None => None,
                    Some(TimeExpr::Literal(v)) => Some(v),
                    _ => {
                        return Err(diagnostic(
                            "E_VIEW_TEMPLATE",
                            "Template time must be concrete",
                            name_span,
                        ));
                    }
                };
                let charge = 8192usize
                    .saturating_add(
                        6 * (name.len()
                            + revision.len()
                            + query.graph_id.len()
                            + query.branch_id.len()
                            + query.predicate.as_ref().map_or(0, String::len)),
                    )
                    .saturating_add(
                        source_revisions
                            .iter()
                            .map(|r| {
                                256usize.saturating_add(
                                    6 * (r.name.len() + r.revision.len() + r.digest.len()),
                                )
                            })
                            .sum::<usize>(),
                    );
                artifact_charge = artifact_charge.saturating_add(charge);
                if view_templates.len() >= 16
                    || charge > 1024 * 1024
                    || artifact_charge > 4 * 1024 * 1024
                {
                    return Err(diagnostic(
                        "E_BUDGET",
                        "Compiled view artifact budget exceeded",
                        name_span,
                    ));
                }
                let template = weave_contract::view_registration::seal_template(
                    weave_contract::CompiledViewTemplate {
                        format: weave_contract::view_registration::VIEW_TEMPLATE_FORMAT.into(),
                        protocol: VERSION.into(),
                        name: name.clone(),
                        revision,
                        expression: GraphExpression::Query { query },
                        clock,
                        source_revisions: source_revisions.clone(),
                        definition_digest: String::new(),
                    },
                )
                .map_err(|e| diagnostic(&e.code, e.message, name_span))?;
                view_templates.insert(name, template);
            }
            Statement::Pin {
                name,
                source,
                source_span,
                valid_at,
                metadata_depth,
                ..
            } => {
                let query = handles.get(&source).ok_or_else(|| {
                    diagnostic(
                        "E_HANDLE_TYPE",
                        "Pin requires a declared live graph handle",
                        source_span,
                    )
                })?;
                let mut query = query.clone();
                query.valid_at = valid_at;
                query.include_metadata = metadata_depth.is_some();
                query.max_depth = metadata_depth.unwrap_or(8);
                let mut value = Lens::concrete(query);
                value.emit(&name, &mut commands);
                names.insert(name, value);
            }
            Statement::ModuleHeader { .. }
            | Statement::Import { .. }
            | Statement::ContextSchema { .. }
            | Statement::Schema { .. }
            | Statement::Transaction { .. }
            | Statement::Value { .. }
            | Statement::Function { .. }
            | Statement::Apply { .. }
            | Statement::Rules { .. } => unreachable!(),
            Statement::ContextValue {
                name,
                name_span,
                schema,
                schema_span,
                attribution,
                axes,
            } => {
                let schema = context_schemas.get(&schema).ok_or_else(|| {
                    diagnostic("E_CONTEXT_SCHEMA", "Unknown context schema", schema_span)
                })?;
                let mut values = BTreeMap::new();
                for axis in axes {
                    if !schema.axes.contains_key(&axis.name) {
                        return Err(diagnostic(
                            "E_CONTEXT_AXIS",
                            "Undeclared context axis",
                            axis.name_span,
                        ));
                    }
                    schema
                        .validate_value(&axis.name, &axis.value)
                        .map_err(|_| {
                            diagnostic(
                                "E_CONTEXT_VALUE",
                                "Axis value does not match its exact declared type",
                                axis.value_span,
                            )
                        })?;
                    values.insert(axis.name, axis.value);
                }
                let definition = weave_contract::context_axes::ContextDefinition {
                    schema: schema.clone(),
                    values,
                };
                definition.validate().map_err(|_| {
                    diagnostic(
                        "E_CONTEXT_VALUE",
                        "Context requires a total bounded assignment",
                        name_span,
                    )
                })?;
                let data:GraphData=serde_json::from_value(serde_json::json!({
                    "profile":"explicit",
                    "nodes":[{"id":"context","entity_id":name,"space_id":"weave:context"}],
                    "structural_edges":[{"id":"descriptor","from":"context","to":"context","predicate":"weave:context:definition"}],
                    "assertions":[{"id":"definition","edge_id":"descriptor","source":attribution,"valid_time":{"start":i64::MIN},"polarity":"positive","properties":{"weave.context":definition}}]
                })).expect("compiler constructs valid descriptor shape");
                emit_snapshot(
                    SnapshotCommit {
                        graph_id: name,
                        branch_id: "main".into(),
                        expected_head: None,
                        data,
                    },
                    &mut names,
                    &mut commands,
                    &mut active_batch,
                );
            }
            Statement::TypedContext {
                name,
                source,
                source_span,
                reference,
                schema,
                schema_span,
                ..
            } => {
                let schema = context_schemas.get(&schema).ok_or_else(|| {
                    diagnostic("E_CONTEXT_SCHEMA", "Unknown context schema", schema_span)
                })?;
                let input = names.get(&source).ok_or_else(|| {
                    diagnostic(
                        "E_UNKNOWN_GRAPH",
                        "Unknown typed-context input",
                        source_span,
                    )
                })?;
                if input.relation.is_some() || input.time.is_some() {
                    return Err(diagnostic(
                        "E_UNBOUND_PARAMETER",
                        "Typed context input must be fully bound",
                        source_span,
                    ));
                }
                let mut lens = Lens::concrete(base_query(String::new(), None));
                lens.typed = input.typed;
                lens.input = Some(GraphExpression::TypedContext {
                    input: Box::new(input.expression()),
                    reference,
                    expected_schema: schema.clone(),
                });
                lens.emit(&name, &mut commands);
                names.insert(name, lens);
            }
            Statement::Graph {
                branch,
                expected_head,
                name,
                name_span,
                items,
                schema,
                schema_span,
                profile,
            } => {
                let mut ids = BTreeSet::new();
                let mut node_ids = BTreeSet::new();
                for item in &items {
                    let (id, id_span) = match item {
                        Item::Structural { id, id_span, .. }
                        | Item::Claim { id, id_span, .. }
                        | Item::Node { id, id_span, .. }
                        | Item::Edge { id, id_span, .. }
                        | Item::Attachment { id, id_span, .. } => (id, *id_span),
                    };
                    if !ids.insert(id.clone()) {
                        return Err(diagnostic(
                            "E_DUPLICATE",
                            format!("Duplicate node or edge '{id}'"),
                            id_span,
                        ));
                    }
                    if matches!(item, Item::Node { .. }) {
                        node_ids.insert(id.clone());
                    }
                }
                let mut data = GraphData {
                    profile,
                    ..GraphData::default()
                };
                if let Some(schema_name) = schema {
                    data.schema = Some(schemas.get(&schema_name).cloned().ok_or_else(|| {
                        diagnostic(
                            "E_UNKNOWN_SCHEMA",
                            format!("Unknown schema '{schema_name}'"),
                            schema_span.unwrap_or(name_span),
                        )
                    })?);
                }
                for item in items {
                    match item {
                        Item::Structural {
                            id,
                            id_span,
                            type_id,
                            from,
                            to,
                            predicate,
                            metadata,
                            properties,
                        } => {
                            if data.profile != GraphProfile::Explicit {
                                return Err(diagnostic(
                                    "E_ASSERTION_PROFILE",
                                    "Structural relations require an explicit graph",
                                    id_span,
                                ));
                            }
                            if !node_ids.contains(&from) || !node_ids.contains(&to) {
                                return Err(diagnostic(
                                    "E_ENDPOINT",
                                    "Structural relation requires both endpoint nodes",
                                    id_span,
                                ));
                            }
                            data.structural_edges.push(StructuralEdge {
                                id,
                                type_id,
                                from,
                                to,
                                predicate,
                                metadata: refs(&metadata),
                                properties: properties
                                    .into_iter()
                                    .map(|(k, v)| (k, v.into_json()))
                                    .collect(),
                                readers: Vec::new(),
                            });
                        }
                        Item::Claim {
                            id,
                            id_span,
                            edge_id,
                            source,
                            context,
                            negative,
                            valid_from,
                            valid_to,
                            metadata,
                            properties,
                        } => {
                            if data.profile != GraphProfile::Explicit {
                                return Err(diagnostic(
                                    "E_ASSERTION_PROFILE",
                                    "Claims require an explicit graph",
                                    id_span,
                                ));
                            }
                            let valid_time = Interval {
                                start: valid_from,
                                end: valid_to,
                            };
                            if !valid_time.valid() {
                                return Err(diagnostic(
                                    "E_INTERVAL",
                                    "Claim interval must be nonempty and half-open",
                                    id_span,
                                ));
                            }
                            if source.is_empty() {
                                return Err(diagnostic(
                                    "E_ASSERTION_SOURCE",
                                    "Claim requires a nonempty source label",
                                    id_span,
                                ));
                            }
                            data.assertions.push(Assertion {
                                id,
                                edge_id,
                                source,
                                context,
                                valid_time,
                                polarity: if negative {
                                    Polarity::Negative
                                } else {
                                    Polarity::Positive
                                },
                                properties: properties
                                    .into_iter()
                                    .map(|(k, v)| (k, v.into_json()))
                                    .collect(),
                                metadata: refs(&metadata),
                                readers: vec![],
                                derived_from: vec![],
                                derived_nodes: vec![],
                                derivations: vec![],
                            });
                        }
                        Item::Attachment {
                            context,
                            id,
                            id_span,
                            host,
                            key,
                            value,
                            literal: _,
                            valid_from,
                            valid_to,
                            required,
                        } => {
                            let valid_time = Interval {
                                start: valid_from,
                                end: valid_to,
                            };
                            if !valid_time.valid() {
                                return Err(diagnostic(
                                    "E_INTERVAL",
                                    "Metadata interval must be nonempty",
                                    id_span,
                                ));
                            }
                            data.attachments.push(MetadataAttachment {
                                context,
                                id,
                                host,
                                key,
                                value,
                                valid_time,
                                origin: None,
                                readers: Vec::new(),
                                required,
                                schema_revision: None,
                            });
                        }
                        Item::Node {
                            id,
                            id_span: _,
                            type_id,
                            entity,
                            space,
                            metadata,
                            properties,
                        } => data.nodes.push(Node {
                            derived_nodes: Vec::new(),
                            derived_from: Vec::new(),
                            context_scope: None,
                            id,
                            type_id,
                            entity_id: entity,
                            space_id: space,
                            properties: properties
                                .into_iter()
                                .map(|(k, v)| (k, v.into_json()))
                                .collect(),
                            metadata: refs(&metadata),
                            readers: Vec::new(),
                        }),
                        Item::Edge {
                            id,
                            id_span,
                            type_id,
                            from,
                            to,
                            predicate,
                            negative,
                            valid_from,
                            valid_to,
                            metadata,
                            properties,
                        } => {
                            if !node_ids.contains(&from) || !node_ids.contains(&to) {
                                return Err(diagnostic(
                                    "E_ENDPOINT",
                                    format!(
                                        "Edge '{id}' requires both endpoint nodes in this graph"
                                    ),
                                    id_span,
                                ));
                            }
                            let valid_time = Interval {
                                start: valid_from,
                                end: valid_to,
                            };
                            if !valid_time.valid() {
                                return Err(diagnostic(
                                    "E_INTERVAL",
                                    "Valid interval must be nonempty and half-open",
                                    id_span,
                                ));
                            }
                            if data.profile == GraphProfile::Explicit {
                                return Err(diagnostic(
                                    "E_ASSERTION_PROFILE",
                                    "Explicit graph requires separate structural relation and claim declarations",
                                    id_span,
                                ));
                            }
                            data.edges.push(Edge {
                                structural_ref: None,
                                assertion_properties: BTreeMap::new(),
                                assertion_source: None,
                                assertion_context: None,
                                id,
                                type_id,
                                predicate,
                                from,
                                to,
                                valid_time,
                                polarity: if negative {
                                    Polarity::Negative
                                } else {
                                    Polarity::Positive
                                },
                                properties: properties
                                    .into_iter()
                                    .map(|(k, v)| (k, v.into_json()))
                                    .collect(),
                                metadata: refs(&metadata),
                                readers: Vec::new(),
                                derived_from: Vec::new(),
                                derived_nodes: Vec::new(),
                                derivations: Vec::new(),
                            });
                        }
                    }
                }
                for attachment in &data.attachments {
                    let exists = match &attachment.host {
                        MetadataHost::Node { id } => data.nodes.iter().any(|n| &n.id == id),
                        MetadataHost::Edge { id } => {
                            data.edges.iter().any(|e| &e.id == id)
                                || data.structural_edges.iter().any(|e| &e.id == id)
                        }
                        MetadataHost::Assertion { id } => {
                            data.assertions.iter().any(|a| &a.id == id)
                        }
                        MetadataHost::Entity { id } => {
                            data.nodes.iter().any(|n| &n.entity_id == id)
                        }
                        MetadataHost::Graph => true,
                    };
                    if !exists {
                        return Err(diagnostic(
                            "E_ATTACHMENT_HOST",
                            format!(
                                "Attachment '{}' has no host in this snapshot",
                                attachment.id
                            ),
                            name_span,
                        ));
                    }
                }
                if let Some(error) = weave_contract::validate_schema_graph(&data).first() {
                    return Err(diagnostic(&error.code, &error.message, name_span));
                }
                emit_snapshot(
                    SnapshotCommit {
                        graph_id: name,
                        branch_id: branch,
                        expected_head,
                        data,
                    },
                    &mut names,
                    &mut commands,
                    &mut active_batch,
                );
            }
            Statement::NativeService { name, service, .. } => {
                let (expression, typed) = match service {
                    syntax::NativeService::Accepted { selection } => {
                        (GraphExpression::AcceptedGraph { selection }, None)
                    }
                    syntax::NativeService::Current {
                        view_id,
                        definition_digest,
                        valid_at,
                    } => {
                        let time = match valid_at {
                            None => weave_contract::ViewReadTime::Fixed,
                            Some(TimeExpr::Literal(valid_at)) => {
                                weave_contract::ViewReadTime::Tick { valid_at }
                            }
                            _ => {
                                return Err(diagnostic(
                                    "E_PARAMETER_TYPE",
                                    "View time must be concrete",
                                    (0, 0),
                                ));
                            }
                        };
                        (
                            GraphExpression::CurrentView {
                                selection: weave_contract::CurrentViewSelection {
                                    view_id,
                                    definition_digest,
                                    time,
                                },
                            },
                            None,
                        )
                    }
                    syntax::NativeService::Identity { selection } => {
                        (GraphExpression::ResolveIdentity { selection }, Some(true))
                    }
                    syntax::NativeService::Cluster { selection } => {
                        (GraphExpression::Cluster { selection }, Some(false))
                    }
                };
                let mut lens = Lens::concrete(base_query(String::new(), None));
                lens.input = Some(expression);
                lens.typed = typed;
                lens.emit(&name, &mut commands);
                names.insert(name, lens);
            }
            Statement::Reason {
                name,
                name_span: _,
                source,
                source_span,
                rule_set,
                rule_span,
            } => {
                let lens = names.get(&source).ok_or_else(|| {
                    diagnostic(
                        "E_UNKNOWN_GRAPH",
                        format!("Unknown graph '{source}'"),
                        source_span,
                    )
                })?;
                if lens.relation.is_some() || lens.time.is_some() {
                    return Err(diagnostic(
                        "E_UNBOUND_PARAMETER",
                        "Rule inputs must be fully bound",
                        source_span,
                    ));
                }
                let rules = rule_sets.get(&rule_set).cloned().ok_or_else(|| {
                    diagnostic(
                        "E_UNKNOWN_RULES",
                        format!("Unknown rule module '{rule_set}'"),
                        rule_span,
                    )
                })?;
                let mut output = Lens::concrete(base_query(String::new(), None));
                output.typed = lens.typed;
                output.input = Some(GraphExpression::Reason {
                    input: Box::new(lens.expression()),
                    rules,
                });
                output.emit(&name, &mut commands);
                names.insert(name, output);
            }
            Statement::Algebra {
                name,
                name_span: _,
                source,
                source_span,
                operation,
            } => {
                let get = |name: &str, span| -> Result<&Lens, Diagnostic> {
                    let lens = names.get(name).ok_or_else(|| {
                        diagnostic(
                            "E_UNKNOWN_GRAPH",
                            format!("Unknown graph/lens '{name}'"),
                            span,
                        )
                    })?;
                    if lens.relation.is_some() || lens.time.is_some() {
                        return Err(diagnostic(
                            "E_UNBOUND_PARAMETER",
                            "Algebra inputs must be fully bound",
                            span,
                        ));
                    }
                    Ok(lens)
                };
                let source_lens = get(&source, source_span)?;
                let mut typed = source_lens.typed;
                let input = Box::new(source_lens.expression());
                let is_union = matches!(&operation, AlgebraOperation::Union { .. });
                let is_distance = matches!(&operation, AlgebraOperation::Distance { .. });
                let expression = match operation {
                    AlgebraOperation::Union { right, right_span }
                    | AlgebraOperation::Diff { right, right_span } => {
                        let right_lens = get(&right, right_span)?;
                        if typed.is_some()
                            && right_lens.typed.is_some()
                            && typed != right_lens.typed
                        {
                            return Err(diagnostic(
                                "E_SCHEMA_ALGEBRA",
                                "Typed and untyped graph values require an explicit mapping",
                                right_span,
                            ));
                        }
                        typed = if typed == right_lens.typed {
                            typed
                        } else {
                            None
                        };
                        let right = Box::new(right_lens.expression());
                        if is_union {
                            GraphExpression::Union { left: input, right }
                        } else {
                            GraphExpression::Diff {
                                before: input,
                                after: right,
                            }
                        }
                    }
                    AlgebraOperation::Distance {
                        left_assertion,
                        right,
                        right_span,
                        right_assertion,
                        valid_at,
                    }
                    | AlgebraOperation::Transform {
                        left_assertion,
                        right,
                        right_span,
                        right_assertion,
                        valid_at,
                    } => {
                        typed = Some(true);
                        let left = weave_contract::GeometryOperand {
                            input,
                            assertion_id: left_assertion,
                        };
                        let right = weave_contract::GeometryOperand {
                            input: Box::new(get(&right, right_span)?.expression()),
                            assertion_id: right_assertion,
                        };
                        let operation = if is_distance {
                            weave_contract::GeometryOperation::Distance { left, right }
                        } else {
                            weave_contract::GeometryOperation::Transform {
                                input: left,
                                mapping: right,
                            }
                        };
                        GraphExpression::Geometry {
                            operation,
                            valid_at,
                        }
                    }
                    AlgebraOperation::ProjectAxes {
                        assertion_id,
                        axes,
                        projection_revision,
                        valid_at,
                    } => {
                        typed = Some(true);
                        GraphExpression::Geometry {
                            operation: weave_contract::GeometryOperation::ProjectAxes {
                                input: weave_contract::GeometryOperand {
                                    input,
                                    assertion_id,
                                },
                                axes: axes.try_into().map_err(|_| {
                                    diagnostic(
                                        "E_GEOMETRY_AXIS",
                                        "Projection requires exactly three axes",
                                        source_span,
                                    )
                                })?,
                                projection_revision,
                            },
                            valid_at,
                        }
                    }
                    AlgebraOperation::Counterparts { selection } => {
                        GraphExpression::Counterparts { input, selection }
                    }
                    AlgebraOperation::Explain => {
                        typed = Some(true);
                        GraphExpression::Explain { input }
                    }
                    AlgebraOperation::Context { selection } => {
                        GraphExpression::Context { input, selection }
                    }
                    AlgebraOperation::Project { node_ids, edge_ids } => GraphExpression::Project {
                        input,
                        node_ids,
                        edge_ids,
                    },
                    AlgebraOperation::Support {
                        predicate,
                        from,
                        to,
                        valid_at,
                    } => {
                        typed = Some(true);
                        GraphExpression::Support {
                            input,
                            predicate,
                            from,
                            to,
                            valid_at,
                        }
                    }
                };
                let mut lens = Lens::concrete(base_query(String::new(), None));
                lens.input = Some(expression);
                lens.typed = typed;
                lens.emit(&name, &mut commands);
                names.insert(name, lens);
            }
            Statement::Use {
                name,
                name_span: _,
                graph,
                revision,
            } => {
                names.insert(name, Lens::concrete(base_query(graph, revision)));
            }
            Statement::Lens {
                name,
                name_span,
                source: input,
                source_span,
                predicate,
                valid_at,
                include_metadata,
                max_depth,
            } => {
                let Some(mut lens) = names.get(&input).cloned() else {
                    return Err(diagnostic(
                        "E_UNKNOWN_GRAPH",
                        format!("Unknown or forward graph/lens reference '{input}'"),
                        source_span,
                    ));
                };
                if let Some(expression) = predicate {
                    if lens.query.predicate.is_some() || lens.relation.is_some() {
                        if !matches!(&expression,StringExpr::Literal(v) if Some(v)==lens.query.predicate.as_ref())
                        {
                            return Err(diagnostic(
                                "E_FILTER_CONFLICT",
                                "Composed relation filters disagree",
                                name_span,
                            ));
                        }
                    } else {
                        match expression {
                            StringExpr::Literal(v) => lens.query.predicate = Some(v),
                            StringExpr::Parameter(p) => lens.relation = Some(p),
                            StringExpr::Value(_) => unreachable!("resolved scalar selector"),
                        }
                    }
                }
                if let Some(expression) = valid_at {
                    if lens.query.valid_at.is_some() || lens.time.is_some() {
                        if !matches!(&expression,TimeExpr::Literal(v) if Some(*v)==lens.query.valid_at)
                        {
                            return Err(diagnostic(
                                "E_FILTER_CONFLICT",
                                "Composed valid-time filters disagree",
                                name_span,
                            ));
                        }
                    } else {
                        match expression {
                            TimeExpr::Literal(v) => lens.query.valid_at = Some(v),
                            TimeExpr::Parameter(p) => lens.time = Some(p),
                            TimeExpr::Value(_) => unreachable!("resolved scalar selector"),
                        }
                    }
                }
                if lens.relation.is_some() && lens.relation == lens.time {
                    return Err(diagnostic(
                        "E_PARAMETER_TYPE",
                        "One parameter cannot be both string and time",
                        name_span,
                    ));
                }
                if include_metadata {
                    if lens.input.is_some() {
                        return Err(diagnostic(
                            "E_METADATA_VALUE",
                            "Materialized graph values retain resolved metadata; further metadata traversal requires a stored graph query",
                            name_span,
                        ));
                    }
                    lens.query.include_metadata = true;
                    lens.query.max_depth = max_depth;
                }
                lens.emit(&name, &mut commands);
                names.insert(name, lens);
            }
            Statement::Join {
                name,
                name_span: _,
                left,
                left_span,
                right,
                right_span,
                predicate,
            } => {
                let get = |name: &str, span| -> Result<GraphExpression, Diagnostic> {
                    let Some(lens) = names.get(name) else {
                        return Err(diagnostic(
                            "E_UNKNOWN_GRAPH",
                            format!("Unknown graph/lens '{name}'"),
                            span,
                        ));
                    };
                    if lens.relation.is_some() || lens.time.is_some() {
                        return Err(diagnostic(
                            "E_UNBOUND_PARAMETER",
                            "Join inputs must be fully bound",
                            span,
                        ));
                    }
                    Ok(lens.expression())
                };
                let left_typed = names.get(&left).and_then(|lens| lens.typed);
                let right_typed = names.get(&right).and_then(|lens| lens.typed);
                if left_typed.is_some() && right_typed.is_some() && left_typed != right_typed {
                    return Err(diagnostic(
                        "E_SCHEMA_JOIN",
                        "Cannot join statically known typed and untyped graphs",
                        left_span,
                    ));
                }
                let expression = GraphExpression::Join {
                    left: Box::new(get(&left, left_span)?),
                    right: Box::new(get(&right, right_span)?),
                    output_predicate: predicate,
                    match_on: JoinMatch::EntitySpaceToFrom,
                };
                let mut lens = Lens::concrete(base_query(String::new(), None));
                lens.input = Some(expression);
                lens.typed = if left_typed == right_typed {
                    left_typed
                } else {
                    None
                };
                lens.emit(&name, &mut commands);
                names.insert(name, lens);
            }
            Statement::Metadata {
                name,
                name_span: _,
                source: input,
                source_span,
                host,
                key,
            } => {
                let Some(source_lens) = names.get(&input) else {
                    return Err(diagnostic(
                        "E_UNKNOWN_GRAPH",
                        format!("Unknown graph/lens '{input}'"),
                        source_span,
                    ));
                };
                if source_lens.relation.is_some() || source_lens.time.is_some() {
                    return Err(diagnostic(
                        "E_UNBOUND_PARAMETER",
                        "Metadata source must be fully bound",
                        source_span,
                    ));
                }
                let mut lens = Lens::concrete(base_query(String::new(), None));
                lens.input = Some(GraphExpression::Metadata {
                    input: Box::new(source_lens.expression()),
                    host,
                    key,
                });
                lens.emit(&name, &mut commands);
                names.insert(name, lens);
            }
            Statement::Bind {
                name,
                name_span: _,
                source: input,
                source_span,
                bindings,
            } => {
                let Some(mut lens) = names.get(&input).cloned() else {
                    return Err(diagnostic(
                        "E_UNKNOWN_GRAPH",
                        format!("Unknown lens '{input}'"),
                        source_span,
                    ));
                };
                let mut seen = BTreeSet::new();
                for binding in bindings {
                    if !seen.insert(binding.name.clone()) {
                        return Err(diagnostic(
                            "E_DUPLICATE",
                            format!("Parameter '{}' bound twice", binding.name),
                            binding.span,
                        ));
                    }
                    if lens.relation.as_ref() == Some(&binding.name) {
                        let BindingValue::String(value) = binding.value else {
                            return Err(diagnostic(
                                "E_PARAMETER_TYPE",
                                "Relation parameter requires a string",
                                binding.span,
                            ));
                        };
                        lens.query.predicate = Some(value);
                        lens.relation = None;
                    } else if lens.time.as_ref() == Some(&binding.name) {
                        let BindingValue::Time(value) = binding.value else {
                            return Err(diagnostic(
                                "E_PARAMETER_TYPE",
                                "Valid-time parameter requires time",
                                binding.span,
                            ));
                        };
                        lens.query.valid_at = Some(value);
                        lens.time = None;
                    } else {
                        return Err(diagnostic(
                            "E_UNKNOWN_PARAMETER",
                            format!("Unknown or already bound parameter '{}'", binding.name),
                            binding.span,
                        ));
                    }
                }
                lens.emit(&name, &mut commands);
                names.insert(name, lens);
            }
        }
    }
    Ok(CompiledArtifacts {
        view_templates,
        values: scalar_values,
        program: Program {
            source_revisions,
            version: VERSION.into(),
            commands,
        },
    })
}

/// Discover validated source schema descriptors without executing a plan.
pub fn describe(source: &str) -> Result<Vec<GraphSchema>, Diagnostic> {
    compile(source)?;
    Ok(parse(source)?
        .statements
        .into_iter()
        .filter_map(|s| {
            if let Statement::Schema { definition, .. } = s {
                Some(definition)
            } else {
                None
            }
        })
        .collect())
}

/// Canonical structural plan identity, including checked source function revisions.
/// It preserves command order and does not assert general algebraic equivalence.
pub fn fingerprint(source: &str) -> Result<String, Diagnostic> {
    let plan = compile(source)?;
    let schemas = parse(source)?
        .statements
        .into_iter()
        .filter_map(|s| match s {
            Statement::Schema { definition, .. } => Some(definition),
            _ => None,
        })
        .collect::<Vec<_>>();
    weave_contract::identity::program_fingerprint(&plan, &schemas)
        .map_err(|d| diagnostic(&d.code, d.message, (0, source.len())))
}
