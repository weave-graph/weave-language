use super::*;
use crate::syntax::{AlgebraOperation, ArgumentValue, SchemaConstraint};

pub(super) struct Resolver<'a> {
    aliases: BTreeMap<&'a str, &'a str>,
    exports: &'a BTreeMap<String, BTreeMap<String, Export>>,
    own: Option<&'a BTreeMap<String, Export>>,
}
impl<'a> Resolver<'a> {
    pub(super) fn new(
        _id: &str,
        imports: &'a [Import],
        exports: &'a BTreeMap<String, BTreeMap<String, Export>>,
        own: Option<&'a BTreeMap<String, Export>>,
    ) -> Self {
        Self {
            aliases: imports
                .iter()
                .map(|i| (i.alias.as_str(), i.module_id.as_str()))
                .collect(),
            exports,
            own,
        }
    }
    fn reference(
        &self,
        name: &mut String,
        span: Span,
        expected: Kind,
        locals: &BTreeSet<String>,
    ) -> Result<(), Diagnostic> {
        if name.starts_with(PREFIX) {
            return Err(issue(
                "E_MODULE_NAME",
                "Authored references cannot use compiler linkage prefix",
                span,
            ));
        }
        let export = if name.contains("::") {
            let mut parts = name.split("::");
            let alias = parts.next().unwrap();
            let member = parts.next().unwrap();
            if parts.next().is_some() {
                return Err(issue(
                    "E_MODULE_NAME",
                    "Only direct alias::member references are supported",
                    span,
                ));
            }
            let module = self
                .aliases
                .get(alias)
                .ok_or_else(|| issue("E_MODULE_ALIAS", "Unknown import alias", span))?;
            Some(
                self.exports
                    .get(*module)
                    .and_then(|e| e.get(member))
                    .ok_or_else(|| {
                        issue("E_MODULE_EXPORT", "Unknown exported declaration", span)
                    })?,
            )
        } else if locals.contains(name) {
            None
        } else {
            self.own.and_then(|e| e.get(name))
        };
        if let Some(export) = export {
            if export.kind != expected {
                return Err(issue(
                    "E_MODULE_KIND",
                    "Imported declaration has the wrong kind",
                    span,
                ));
            }
            *name = export.symbol.clone();
        }
        Ok(())
    }
    fn constraint(&self, c: &mut Option<SchemaConstraint>) -> Result<(), Diagnostic> {
        if let Some(c) = c {
            self.reference(&mut c.name, c.span, Kind::Schema, &BTreeSet::new())?;
        }
        Ok(())
    }
    fn local_graph(name: &str, span: Span) -> Result<(), Diagnostic> {
        if name.starts_with(PREFIX) {
            return Err(issue(
                "E_MODULE_NAME",
                "Authored references cannot use compiler linkage prefix",
                span,
            ));
        }
        if name.contains("::") {
            Err(issue(
                "E_MODULE_KIND",
                "Pure modules do not export runtime graph values",
                span,
            ))
        } else {
            Ok(())
        }
    }
    pub(super) fn statement(
        &self,
        statement: &mut Statement,
        top: bool,
        locals: &BTreeSet<String>,
    ) -> Result<(), Diagnostic> {
        let (original, span) = declaration(statement);
        let original = original.to_owned();
        if original.starts_with(PREFIX) {
            return Err(issue(
                "E_MODULE_NAME",
                "Compiler linkage prefix is reserved",
                span,
            ));
        }
        match statement {
            Statement::Schema {
                name, definition, ..
            } => {
                if top && let Some(export) = self.own.and_then(|e| e.get(&original)) {
                    *name = export.symbol.clone();
                    definition.id = export.identity.clone();
                }
            }
            Statement::ContextSchema {
                name, definition, ..
            } => {
                if top && let Some(export) = self.own.and_then(|e| e.get(&original)) {
                    *name = export.symbol.clone();
                    definition.reference.id = export.identity.clone();
                }
            }
            Statement::Rules {
                name, definition, ..
            } => {
                if top && let Some(export) = self.own.and_then(|e| e.get(&original)) {
                    *name = export.symbol.clone();
                    definition.id = export.identity.clone();
                }
            }
            Statement::Function {
                name,
                parameters,
                output_schema,
                body,
                ..
            } => {
                if top && let Some(export) = self.own.and_then(|e| e.get(&original)) {
                    *name = export.symbol.clone();
                }
                let mut scope = BTreeSet::new();
                for parameter in parameters {
                    if parameter.name.contains("::") || parameter.name.starts_with(PREFIX) {
                        return Err(issue(
                            "E_MODULE_NAME",
                            "Function-local identifiers cannot be qualified or reserved",
                            parameter.span,
                        ));
                    }
                    scope.insert(parameter.name.clone());
                    self.constraint(&mut parameter.schema)?;
                }
                self.constraint(output_schema)?;
                for child in body {
                    let name = declaration(child).0.to_owned();
                    self.statement(child, false, &scope)?;
                    scope.insert(name);
                }
            }
            Statement::Graph {
                schema: Some(name),
                schema_span,
                ..
            } => {
                self.reference(
                    name,
                    schema_span.unwrap_or(span),
                    Kind::Schema,
                    &BTreeSet::new(),
                )?;
            }
            Statement::ContextValue {
                schema,
                schema_span,
                ..
            }
            | Statement::TypedContext {
                schema,
                schema_span,
                ..
            } => self.reference(schema, *schema_span, Kind::ContextSchema, &BTreeSet::new())?,
            Statement::Transaction { body, .. } => {
                for child in body {
                    self.statement(child, true, locals)?;
                }
            }
            Statement::Apply {
                function,
                function_span,
                arguments,
                ..
            } => {
                self.reference(function, *function_span, Kind::Function, locals)?;
                for argument in arguments {
                    match &mut argument.value {
                        ArgumentValue::Function(name) => {
                            self.reference(name, argument.value_span, Kind::Function, locals)?
                        }
                        ArgumentValue::Graph(name) => Self::local_graph(name, argument.value_span)?,
                        _ => (),
                    }
                }
            }
            Statement::Reason {
                source,
                source_span,
                rule_set,
                rule_span,
                ..
            } => {
                Self::local_graph(source, *source_span)?;
                self.reference(rule_set, *rule_span, Kind::Rules, &BTreeSet::new())?;
            }
            Statement::Join {
                left,
                left_span,
                right,
                right_span,
                ..
            } => {
                Self::local_graph(left, *left_span)?;
                Self::local_graph(right, *right_span)?;
            }
            Statement::Lens {
                source,
                source_span,
                ..
            }
            | Statement::Bind {
                source,
                source_span,
                ..
            }
            | Statement::Metadata {
                source,
                source_span,
                ..
            }
            | Statement::Pin {
                source,
                source_span,
                ..
            } => Self::local_graph(source, *source_span)?,
            Statement::Algebra {
                source,
                source_span,
                operation,
                ..
            } => {
                Self::local_graph(source, *source_span)?;
                match operation {
                    AlgebraOperation::Union { right, right_span }
                    | AlgebraOperation::Diff { right, right_span }
                    | AlgebraOperation::Distance {
                        right, right_span, ..
                    }
                    | AlgebraOperation::Transform {
                        right, right_span, ..
                    } => Self::local_graph(right, *right_span)?,
                    _ => (),
                }
            }
            _ => (),
        }
        Ok(())
    }
}
// Only known AST metadata fields are offset. Literal JSON and schema maps are not traversed.
pub(super) fn shift(statement: &mut Statement, base: usize) {
    fn span(value: &mut serde_json::Value, base: usize) {
        if let Some(pair) = value.as_array_mut() {
            for number in pair {
                if let Some(n) = number.as_u64() {
                    *number = serde_json::json!(n + base as u64);
                }
            }
        }
    }
    fn ast(value: &mut serde_json::Value, base: usize) {
        let Some(map) = value.as_object_mut() else {
            return;
        };
        for key in [
            "name_span",
            "source_span",
            "rule_span",
            "left_span",
            "right_span",
            "function_span",
            "output_span",
            "schema_span",
            "digest_span",
        ] {
            if let Some(v) = map.get_mut(key) {
                span(v, base);
            }
        }
        if let Some(body) = map
            .get_mut("body")
            .and_then(serde_json::Value::as_array_mut)
        {
            for child in body {
                ast(child, base);
            }
        }
        for key in ["parameters", "arguments", "bindings"] {
            if let Some(values) = map.get_mut(key).and_then(serde_json::Value::as_array_mut) {
                for value in values {
                    if let Some(fields) = value.as_object_mut() {
                        for key in ["span", "value_span"] {
                            if let Some(v) = fields.get_mut(key) {
                                span(v, base);
                            }
                        }
                        if let Some(s) = fields
                            .get_mut("schema")
                            .and_then(serde_json::Value::as_object_mut)
                            && let Some(v) = s.get_mut("span")
                        {
                            span(v, base);
                        }
                    }
                }
            }
        }
        if let Some(s) = map
            .get_mut("output_schema")
            .and_then(serde_json::Value::as_object_mut)
            && let Some(v) = s.get_mut("span")
        {
            span(v, base);
        }
        if let Some(op) = map
            .get_mut("operation")
            .and_then(serde_json::Value::as_object_mut)
            && let Some(v) = op.get_mut("right_span")
        {
            span(v, base);
        }
    }
    let mut json = serde_json::to_value(&*statement).expect("AST serializable");
    ast(&mut json, base);
    *statement = serde_json::from_value(json).expect("offset AST remains well typed");
}
