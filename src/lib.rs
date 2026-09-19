//! Experimental, deterministic front end for the Weave graph language.
//! No source program can invoke host effects: compilation only emits a plan.
pub mod syntax;
use std::collections::{BTreeMap, BTreeSet};
pub use syntax::{Diagnostic, parse};
use syntax::{Item, Metadata, Statement};
use weave_contract::{
    Command, Edge, GraphData, GraphRef, Interval, Node, Polarity, Program, QueryPlan, VERSION,
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
/// Compile proposed surface syntax into the shared, versioned engine protocol.
/// Declarations are sequential. A lens can compose earlier lenses by adding
/// compatible filters. Graph declarations emit new-branch snapshot commits.
pub fn compile(source: &str) -> Result<Program, Diagnostic> {
    let ast = parse(source)?;
    let mut names: BTreeMap<String, QueryPlan> = BTreeMap::new();
    let mut commands = Vec::new();
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
            } => (name, *name_span),
        };
        if names.contains_key(name) {
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
                names.insert(name.clone(), base_query(name.clone(), None));
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
                names.insert(name, base_query(graph, revision));
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
                let Some(mut query) = names.get(&input).cloned() else {
                    return Err(diagnostic(
                        "E_UNKNOWN_GRAPH",
                        format!("Unknown or forward graph/lens reference '{input}'"),
                        source_span,
                    ));
                };
                if query.predicate.is_some() && predicate.is_some() && query.predicate != predicate
                {
                    return Err(diagnostic(
                        "E_FILTER_CONFLICT",
                        "Composed relation filters disagree",
                        name_span,
                    ));
                }
                if query.valid_at.is_some() && valid_at.is_some() && query.valid_at != valid_at {
                    return Err(diagnostic(
                        "E_FILTER_CONFLICT",
                        "Composed valid-time filters disagree",
                        name_span,
                    ));
                }
                query.predicate = predicate.or(query.predicate);
                query.valid_at = valid_at.or(query.valid_at);
                if include_metadata {
                    query.include_metadata = true;
                    query.max_depth = max_depth;
                }
                names.insert(name, query.clone());
                commands.push(Command::Query { query });
            }
        }
    }
    Ok(Program {
        version: VERSION.into(),
        commands,
    })
}
