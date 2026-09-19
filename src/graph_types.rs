//! Conservative compiler knowledge; unknown runtime descriptors are never asserted as known.
use crate::syntax::{AlgebraOperation, Diagnostic, SchemaConstraint, Span, Statement};
use std::{collections::BTreeMap, sync::Arc};
use weave_contract::GraphSchema;

#[derive(Clone, Debug, Default)]
pub(crate) enum Knowledge {
    Exact(Arc<GraphSchema>),
    Untyped,
    #[default]
    Unknown,
}
impl Knowledge {
    pub(crate) fn require(&self, expected: &GraphSchema, span: Span) -> Result<(), Diagnostic> {
        match self {
            Self::Exact(actual) if actual.as_ref() == expected => Ok(()),
            Self::Unknown => Err(Diagnostic::new(
                "E_SCHEMA_UNKNOWN",
                "Exact graph schema cannot be proven statically; no runtime schema assertion is implied",
                span.0,
                span.1,
            )),
            _ => Err(Diagnostic::new(
                "E_SCHEMA_MISMATCH",
                "Graph argument or return does not have the exact required schema descriptor",
                span.0,
                span.1,
            )),
        }
    }
}
#[derive(Default)]
pub(crate) struct State {
    pub(crate) declarations: BTreeMap<String, Arc<GraphSchema>>,
    pub(crate) graphs: BTreeMap<String, Knowledge>,
}
impl State {
    pub(crate) fn resolve(
        &self,
        constraint: &SchemaConstraint,
    ) -> Result<Arc<GraphSchema>, Diagnostic> {
        self.declarations
            .get(&constraint.name)
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(
                    "E_UNKNOWN_SCHEMA",
                    "Schema constraints require an earlier declaration",
                    constraint.span.0,
                    constraint.span.1,
                )
            })
    }
    pub(crate) fn get(&self, name: &str) -> Knowledge {
        self.graphs.get(name).cloned().unwrap_or_default()
    }
    pub(crate) fn observe(&mut self, statement: &Statement) -> Result<(), Diagnostic> {
        match statement {
            Statement::Schema {
                name,
                name_span,
                definition,
            } => {
                if self.declarations.contains_key(name) {
                    return Err(Diagnostic::new(
                        "E_DUPLICATE",
                        "Duplicate graph schema declaration",
                        name_span.0,
                        name_span.1,
                    ));
                }
                self.declarations
                    .insert(name.clone(), Arc::new(definition.clone()));
            }
            Statement::Transaction { body, .. } => {
                for child in body {
                    self.observe(child)?;
                }
            }
            Statement::Graph {
                name,
                schema,
                name_span,
                ..
            } => {
                let knowledge = if let Some(schema) = schema {
                    Knowledge::Exact(self.resolve(&SchemaConstraint {
                        name: schema.clone(),
                        span: *name_span,
                    })?)
                } else {
                    Knowledge::Untyped
                };
                self.graphs.insert(name.clone(), knowledge);
            }
            Statement::ContextValue { name, .. } => {
                self.graphs.insert(name.clone(), Knowledge::Untyped);
            }
            _ => {
                if let Some((name, knowledge)) = transfer(statement, &self.graphs) {
                    self.graphs.insert(name.into(), knowledge);
                }
            }
        }
        Ok(())
    }
}
pub(crate) fn transfer<'a>(
    statement: &'a Statement,
    graphs: &BTreeMap<String, Knowledge>,
) -> Option<(&'a str, Knowledge)> {
    let (name, source) = match statement {
        Statement::Lens { name, source, .. }
        | Statement::Bind { name, source, .. }
        | Statement::TypedContext { name, source, .. } => (name, Some(source)),
        Statement::Algebra {
            name,
            source,
            operation,
            ..
        } => (
            name,
            match operation {
                AlgebraOperation::Project { .. }
                | AlgebraOperation::Context { .. }
                | AlgebraOperation::Counterparts { .. } => Some(source),
                _ => None,
            },
        ),
        Statement::Reason { name, .. }
        | Statement::Join { name, .. }
        | Statement::Metadata { name, .. }
        | Statement::NativeService { name, .. }
        | Statement::Use { name, .. }
        | Statement::Pin { name, .. } => (name, None),
        _ => return None,
    };
    Some((
        name,
        source
            .and_then(|n| graphs.get(n))
            .cloned()
            .unwrap_or_default(),
    ))
}
