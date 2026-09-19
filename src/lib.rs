//! Experimental, deterministic front end for the Weave graph language.
//! No source program can invoke host effects: compilation only emits a plan.
pub mod context_axes;
pub mod decimal;
mod functions;
pub mod syntax;
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
/// Compile proposed surface syntax into the shared, versioned engine protocol.
/// Declarations are sequential. A lens can compose earlier lenses by adding
/// compatible filters. Graph declarations emit new-branch snapshot commits.
pub fn compile(source: &str) -> Result<Program, Diagnostic> {
    let parsed = parse(source)?;
    let source_revisions = functions::source_revisions(&parsed)?;
    let ast = functions::expand(parsed)?;
    let mut names: BTreeMap<String, Lens> = BTreeMap::new();
    let mut commands = Vec::new();
    let mut declared = BTreeSet::new();
    let mut schemas: BTreeMap<String, GraphSchema> = BTreeMap::new();
    let mut rule_sets = BTreeMap::new();
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
            Statement::NativeService {
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
            Statement::Schema { .. }
            | Statement::Transaction { .. }
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
            Statement::Schema { .. }
            | Statement::Transaction { .. }
            | Statement::Function { .. }
            | Statement::Apply { .. }
            | Statement::Rules { .. } => unreachable!(),
            Statement::Graph {
                name,
                name_span,
                items,
                schema,
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
                            name_span,
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
                                properties,
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
                                properties,
                                metadata: refs(&metadata),
                                readers: vec![],
                                derived_from: vec![],
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
                            properties,
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
                                properties,
                                metadata: refs(&metadata),
                                readers: Vec::new(),
                                derived_from: Vec::new(),
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
                let mut graph_lens = Lens::concrete(base_query(name.clone(), None));
                graph_lens.typed = Some(data.schema.is_some());
                names.insert(name.clone(), graph_lens);
                if let Some((_, remaining, members)) = &mut active_batch {
                    members.push(SnapshotCommit {
                        graph_id: name,
                        branch_id: "main".into(),
                        expected_head: None,
                        data,
                    });
                    *remaining -= 1;
                    if *remaining == 0 {
                        let (batch_id, _, commits) = active_batch.take().unwrap();
                        commands.push(Command::CommitBatch { batch_id, commits });
                    }
                } else {
                    commands.push(Command::Commit {
                        graph_id: name,
                        branch_id: "main".into(),
                        expected_head: None,
                        data,
                    });
                }
            }
            Statement::NativeService { name, service, .. } => {
                let (expression, typed) = match service {
                    syntax::NativeService::Identity { selection } => {
                        (GraphExpression::ResolveIdentity { selection }, true)
                    }
                    syntax::NativeService::Cluster { selection } => {
                        (GraphExpression::Cluster { selection }, false)
                    }
                };
                let mut lens = Lens::concrete(base_query(String::new(), None));
                lens.input = Some(expression);
                lens.typed = Some(typed);
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
    Ok(Program {
        source_revisions,
        version: VERSION.into(),
        commands,
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
