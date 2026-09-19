//! Experimental, deterministic front end for the Weave graph language.
//! No source program can invoke host effects: compilation only emits a plan.
pub mod syntax;
use std::collections::{BTreeMap, BTreeSet};
use syntax::{BindingValue, Item, Metadata, Statement, StringExpr, TimeExpr};
pub use syntax::{Diagnostic, parse};
use weave_contract::{
    Command, Edge, GraphData, GraphRef, Interval, JoinMatch, Node, Polarity, Program, QueryPlan,
    VERSION,
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
    relation: Option<String>,
    time: Option<String>,
}
impl Lens {
    fn concrete(query: QueryPlan) -> Self {
        Self {
            query,
            relation: None,
            time: None,
        }
    }
    fn emit(&self, commands: &mut Vec<Command>) {
        if self.relation.is_none() && self.time.is_none() {
            commands.push(Command::Query {
                query: self.query.clone(),
            });
        }
    }
}
/// Compile proposed surface syntax into the shared, versioned engine protocol.
/// Declarations are sequential. A lens can compose earlier lenses by adding
/// compatible filters. Graph declarations emit new-branch snapshot commits.
pub fn compile(source: &str) -> Result<Program, Diagnostic> {
    let ast = parse(source)?;
    let mut names: BTreeMap<String, Lens> = BTreeMap::new();
    let mut commands = Vec::new();
    let mut declared = BTreeSet::new();
    for statement in ast.statements {
        let (name, name_span) = match &statement {
            Statement::Graph {
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
            } => (name, *name_span),
        };
        if !declared.insert(name.clone()) {
            return Err(diagnostic(
                "E_DUPLICATE",
                format!("Duplicate graph or lens '{name}'"),
                name_span,
            ));
        }
        match statement {
            Statement::Graph { name, items, .. } => {
                let mut ids = BTreeSet::new();
                let mut node_ids = BTreeSet::new();
                for item in &items {
                    let (id, id_span) = match item {
                        Item::Node { id, id_span, .. } | Item::Edge { id, id_span, .. } => {
                            (id, *id_span)
                        }
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
                let mut data = GraphData::default();
                for item in items {
                    match item {
                        Item::Node {
                            id,
                            id_span: _,
                            entity,
                            space,
                            metadata,
                        } => data.nodes.push(Node {
                            id,
                            entity_id: entity,
                            space_id: space,
                            properties: BTreeMap::new(),
                            metadata: refs(&metadata),
                            readers: Vec::new(),
                        }),
                        Item::Edge {
                            id,
                            id_span,
                            from,
                            to,
                            predicate,
                            valid_from,
                            valid_to,
                            metadata,
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
                            data.edges.push(Edge {
                                id,
                                predicate,
                                from,
                                to,
                                valid_time,
                                polarity: Polarity::Positive,
                                properties: BTreeMap::new(),
                                metadata: refs(&metadata),
                                readers: Vec::new(),
                                derived_from: Vec::new(),
                            });
                        }
                    }
                }
                names.insert(name.clone(), Lens::concrete(base_query(name.clone(), None)));
                commands.push(Command::Commit {
                    graph_id: name,
                    branch_id: "main".into(),
                    expected_head: None,
                    data,
                });
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
                    lens.query.include_metadata = true;
                    lens.query.max_depth = max_depth;
                }
                lens.emit(&mut commands);
                names.insert(name, lens);
            }
            Statement::Join {
                name: _,
                name_span: _,
                left,
                left_span,
                right,
                right_span,
                predicate,
            } => {
                let get = |name: &str, span| -> Result<QueryPlan, Diagnostic> {
                    let Some(lens) = names.get(name) else {
                        return Err(diagnostic(
                            "E_UNKNOWN_GRAPH",
                            format!(
                                "Unknown graph/lens '{name}'; join outputs cannot yet be reused"
                            ),
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
                    Ok(lens.query.clone())
                };
                commands.push(Command::Join {
                    left: get(&left, left_span)?,
                    right: get(&right, right_span)?,
                    output_predicate: predicate,
                    match_on: JoinMatch::EntitySpaceToFrom,
                });
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
                lens.emit(&mut commands);
                names.insert(name, lens);
            }
        }
    }
    Ok(Program {
        version: VERSION.into(),
        commands,
    })
}
