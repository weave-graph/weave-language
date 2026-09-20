use crate::scalars::{
    Budget, LiteralExpr, ScalarExpr, ScalarExpression, ScalarReturn, ScalarType, ScalarValue,
};
// Hygienic, bounded specialization of pure graph-function values.
use crate::graph_types::{self, Knowledge};
use crate::syntax::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use weave_contract::GraphSchema;

#[derive(Clone)]
struct Definition {
    parameters: Vec<FunctionParameter>,
    body: Vec<Statement>,
    output: String,
    output_span: Span,
    scalar_return: Option<ScalarReturn>,
    callbacks: BTreeMap<String, Signature>,
    parameter_schemas: BTreeMap<String, Arc<GraphSchema>>,
    output_schema: Option<Arc<GraphSchema>>,
    captured: BTreeMap<String, Arc<Closure>>,
}
#[derive(Clone)]
struct Closure {
    definition: Arc<Definition>,
    bound: BTreeMap<String, Value>,
}
#[derive(Clone)]
enum Value {
    Graph(String),
    Scalar(ScalarValue),
    Function(Arc<Closure>),
}
impl Value {
    fn kind(&self) -> ParameterKind {
        match self {
            Self::Graph(_) => ParameterKind::Graph,
            Self::Scalar(v) => scalar_kind(&v.value_type()),
            Self::Function(_) => ParameterKind::Function,
        }
    }
}
fn scalar_kind(t: &ScalarType) -> ParameterKind {
    match t {
        ScalarType::String => ParameterKind::String,
        ScalarType::Time => ParameterKind::Time,
        t => ParameterKind::Scalar(t.clone()),
    }
}
fn scalar_type(k: &ParameterKind) -> Option<ScalarType> {
    match k {
        ParameterKind::String => Some(ScalarType::String),
        ParameterKind::Time => Some(ScalarType::Time),
        ParameterKind::Scalar(t) => Some(t.clone()),
        _ => None,
    }
}
fn argument_scalar(
    value: &ArgumentValue,
    budget: &mut Budget,
    span: Span,
) -> Result<Option<ScalarExpr>, Diagnostic> {
    let size = match value {
        ArgumentValue::Scalar { value, .. }
        | ArgumentValue::String(StringExpr::Value(value))
        | ArgumentValue::Time(TimeExpr::Value(value)) => value.retained_bytes(),
        ArgumentValue::String(StringExpr::Literal(v)) => v.len() + 64,
        _ => 64,
    };
    budget.charge(size, span)?;
    Ok(match value {
        ArgumentValue::Scalar { value, .. } => Some(value.clone()),
        ArgumentValue::String(StringExpr::Value(e)) | ArgumentValue::Time(TimeExpr::Value(e)) => {
            Some(e.clone())
        }
        ArgumentValue::String(StringExpr::Literal(v)) => Some(ScalarExpr {
            span: (0, 0),
            expression: ScalarExpression::Literal(ScalarValue::String(v.clone())),
        }),
        ArgumentValue::Time(TimeExpr::Literal(v)) => Some(ScalarExpr {
            span: (0, 0),
            expression: ScalarExpression::Literal(ScalarValue::Time(*v)),
        }),
        ArgumentValue::String(StringExpr::Parameter(n))
        | ArgumentValue::Time(TimeExpr::Parameter(n)) => Some(ScalarExpr {
            span: (0, 0),
            expression: ScalarExpression::Parameter(n.clone()),
        }),
        _ => None,
    })
}
fn marker_matches(a: &ArgumentValue, t: &ScalarType) -> bool {
    match a {
        ArgumentValue::Scalar { kind, .. } => match t {
            ScalarType::Boolean => kind == "boolean",
            ScalarType::Integer => kind == "integer",
            ScalarType::Decimal => kind == "decimal",
            ScalarType::Quantity(_) => kind == "quantity",
            _ => false,
        },
        ArgumentValue::String(_) => *t == ScalarType::String,
        ArgumentValue::Time(_) => *t == ScalarType::Time,
        _ => false,
    }
}
fn substitute(
    e: &mut ScalarExpr,
    scope: &BTreeMap<String, String>,
    values: &BTreeMap<String, Value>,
    budget: &mut Budget,
) -> Result<(), Diagnostic> {
    match &mut e.expression {
        ScalarExpression::Parameter(n) => {
            let Some(Value::Scalar(v)) = values.get(n) else {
                return Err(error(
                    "E_FUNCTION_SCOPE",
                    "Expected scalar parameter",
                    e.span,
                ));
            };
            budget.charge(v.size(), e.span)?;
            e.expression = ScalarExpression::Literal(v.clone());
        }
        ScalarExpression::Value(n) => *n = renamed(n, scope, e.span)?,
        ScalarExpression::Convert { input, .. } => substitute(input, scope, values, budget)?,
        ScalarExpression::Call { arguments, .. } => {
            for a in arguments {
                substitute(a, scope, values, budget)?;
            }
        }
        _ => (),
    }
    Ok(())
}
fn error(code: &str, message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(code, message, span.0, span.1)
}
pub(crate) fn declaration(statement: &Statement) -> (&str, Span) {
    match statement {
        Statement::Value {
            name, name_span, ..
        }
        | Statement::ModuleHeader {
            name, name_span, ..
        }
        | Statement::Import {
            name, name_span, ..
        }
        | Statement::LiveHandle {
            name, name_span, ..
        }
        | Statement::Pin {
            name, name_span, ..
        }
        | Statement::ContextSchema {
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
        | Statement::Rules {
            name, name_span, ..
        }
        | Statement::Reason {
            name, name_span, ..
        }
        | Statement::Function {
            name, name_span, ..
        }
        | Statement::Apply {
            name, name_span, ..
        }
        | Statement::Algebra {
            name, name_span, ..
        }
        | Statement::Transaction {
            name, name_span, ..
        }
        | Statement::Metadata {
            name, name_span, ..
        }
        | Statement::Schema {
            name, name_span, ..
        }
        | Statement::Join {
            name, name_span, ..
        }
        | Statement::Bind {
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
        } => (name, *name_span),
    }
}
fn renamed(name: &str, scope: &BTreeMap<String, String>, span: Span) -> Result<String, Diagnostic> {
    scope.get(name).cloned().ok_or_else(|| {
        error(
            "E_FUNCTION_SCOPE",
            format!("Function input '{name}' is not declared"),
            span,
        )
    })
}
fn string_value(
    value: &mut StringExpr,
    values: &BTreeMap<String, Value>,
    span: Span,
) -> Result<(), Diagnostic> {
    if let StringExpr::Parameter(name) = value {
        *value = match values.get(name) {
            Some(Value::Scalar(ScalarValue::String(s))) => StringExpr::Literal(s.clone()),
            _ => {
                return Err(error(
                    "E_PARAMETER_TYPE",
                    format!("'{name}' is not a string function parameter"),
                    span,
                ));
            }
        };
    }
    Ok(())
}
fn time_value(
    value: &mut TimeExpr,
    values: &BTreeMap<String, Value>,
    span: Span,
) -> Result<(), Diagnostic> {
    if let TimeExpr::Parameter(name) = value {
        *value = match values.get(name) {
            Some(Value::Scalar(ScalarValue::Time(t))) => TimeExpr::Literal(*t),
            _ => {
                return Err(error(
                    "E_PARAMETER_TYPE",
                    format!("'{name}' is not a time function parameter"),
                    span,
                ));
            }
        };
    }
    Ok(())
}
struct Expander {
    scalar_values: BTreeMap<String, ScalarValue>,
    scalar_budget: Budget,
    functions: BTreeMap<String, Arc<Closure>>,
    rule_modules: BTreeSet<String>,
    graphs: BTreeMap<String, BTreeSet<String>>,
    schemas: graph_types::State,
    reserved: BTreeSet<String>,
    output: Vec<Statement>,
    counter: usize,
    bytes: usize,
}
impl Expander {
    fn fresh(&mut self, hint: &str) -> String {
        loop {
            self.counter += 1;
            let id = format!("__weave_function_{}_{}", self.counter, hint);
            if self.reserved.insert(id.clone()) {
                return id;
            }
        }
    }
    fn resolve_statement(&mut self, s: &mut Statement) -> Result<(), Diagnostic> {
        match s {
            Statement::Lens {
                predicate,
                valid_at,
                ..
            } => {
                if let Some(StringExpr::Value(e)) = predicate {
                    let v = e.evaluate(
                        &BTreeMap::new(),
                        &self.scalar_values,
                        &mut self.scalar_budget,
                    )?;
                    let ScalarValue::String(v) = v else {
                        return Err(error("E_PARAMETER_TYPE", "Relation needs String", e.span));
                    };
                    *predicate = Some(StringExpr::Literal(v));
                }
                if let Some(TimeExpr::Value(e)) = valid_at {
                    let v = e.evaluate(
                        &BTreeMap::new(),
                        &self.scalar_values,
                        &mut self.scalar_budget,
                    )?;
                    let ScalarValue::Time(v) = v else {
                        return Err(error(
                            "E_PARAMETER_TYPE",
                            "Time selector needs Time",
                            e.span,
                        ));
                    };
                    *valid_at = Some(TimeExpr::Literal(v));
                }
            }
            Statement::Transaction { body, .. } => {
                for s in body {
                    self.resolve_statement(s)?;
                }
            }
            Statement::Graph { items, schema, .. } => {
                let descriptor = schema
                    .as_ref()
                    .and_then(|n| self.schemas.declarations.get(n));
                for item in items {
                    if let Item::Attachment {
                        value,
                        literal: Some(literal),
                        ..
                    } = item
                    {
                        *value = weave_contract::MetadataValue::Literal {
                            value: literal.resolve(&self.scalar_values, &mut self.scalar_budget)?,
                        };
                        continue;
                    }
                    let (properties, expected) = match item {
                        Item::Node {
                            properties,
                            type_id,
                            ..
                        } => (
                            Some(properties),
                            descriptor
                                .and_then(|d| type_id.as_ref().and_then(|t| d.nodes.get(t)))
                                .map(|s| &s.properties),
                        ),
                        Item::Edge {
                            properties,
                            type_id,
                            ..
                        }
                        | Item::Structural {
                            properties,
                            type_id,
                            ..
                        } => (
                            Some(properties),
                            descriptor
                                .and_then(|d| type_id.as_ref().and_then(|t| d.edges.get(t)))
                                .map(|s| &s.properties),
                        ),
                        Item::Claim { properties, .. } => (Some(properties), None),
                        _ => (None, None),
                    };
                    if let Some(properties) = properties {
                        for (key, literal) in properties {
                            let value = if let LiteralExpr::Scalar(expr) = literal {
                                let v = expr.evaluate(
                                    &BTreeMap::new(),
                                    &self.scalar_values,
                                    &mut self.scalar_budget,
                                )?;
                                if let Some(property) = expected.and_then(|p| p.get(key)) {
                                    use weave_contract::ScalarType as T;
                                    let ty = match &property.value_type {
                                        T::String => Some(ScalarType::String),
                                        T::Integer => Some(ScalarType::Integer),
                                        T::Boolean => Some(ScalarType::Boolean),
                                        T::Decimal => Some(ScalarType::Decimal),
                                        T::Quantity(u) => Some(ScalarType::Quantity(u.clone())),
                                        T::Float => None,
                                    };
                                    if ty.as_ref() != Some(&v.value_type()) {
                                        return Err(error(
                                            "E_SCALAR_TYPE",
                                            "Scalar value does not have the declared property type",
                                            expr.span,
                                        ));
                                    }
                                }
                                v.json()
                            } else {
                                literal.resolve(&self.scalar_values, &mut self.scalar_budget)?
                            };
                            *literal = LiteralExpr::Json(value);
                        }
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }
    fn insert_scalar(
        &mut self,
        name: String,
        value: ScalarValue,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.scalar_budget.binding(&value, span)?;
        self.scalar_values.insert(name, value);
        Ok(())
    }
    fn emit(&mut self, mut statement: Statement) -> Result<(), Diagnostic> {
        self.resolve_statement(&mut statement)?;
        let (name, span) = declaration(&statement);
        let name = name.to_owned();
        self.bytes = self.bytes.saturating_add(
            serde_json::to_vec(&statement)
                .expect("AST serializes")
                .len(),
        );
        if self.output.len() >= 10_000 || self.bytes > 4_194_304 {
            return Err(error(
                "E_EXPANSION_BUDGET",
                "Function expansion exceeds the bounded compiler profile",
                span,
            ));
        }
        let pending = match &statement {
            Statement::Lens {
                source,
                predicate,
                valid_at,
                ..
            } => {
                let mut p = self.graphs.get(source).cloned().unwrap_or_default();
                if let Some(StringExpr::Parameter(n)) = predicate {
                    p.insert(n.clone());
                }
                if let Some(TimeExpr::Parameter(n)) = valid_at {
                    p.insert(n.clone());
                }
                p
            }
            Statement::Bind {
                source, bindings, ..
            } => {
                let mut p = self.graphs.get(source).cloned().unwrap_or_default();
                for b in bindings {
                    p.remove(&b.name);
                }
                p
            }
            _ => BTreeSet::new(),
        };
        if !matches!(
            statement,
            Statement::Rules { .. }
                | Statement::Schema { .. }
                | Statement::ContextSchema { .. }
                | Statement::LiveHandle { .. }
                | Statement::Transaction { .. }
        ) {
            self.graphs.insert(name, pending);
        }
        if let Statement::Transaction { body, .. } = &statement {
            for s in body {
                self.graphs.insert(declaration(s).0.into(), BTreeSet::new());
            }
        }
        if let Statement::Rules { name, .. } = &statement {
            self.rule_modules.insert(name.clone());
        }
        self.schemas.observe(&statement)?;
        self.output.push(statement);
        Ok(())
    }
    fn process(&mut self, statement: Statement, depth: usize) -> Result<(), Diagnostic> {
        if depth > 32 {
            return Err(error(
                "E_EXPANSION_BUDGET",
                "Function application depth exceeds 32",
                declaration(&statement).1,
            ));
        }
        self.scalar_budget.step(declaration(&statement).1)?;
        match statement {
            Statement::Value {
                name,
                name_span,
                value,
            } => {
                let v = value.evaluate(
                    &BTreeMap::new(),
                    &self.scalar_values,
                    &mut self.scalar_budget,
                )?;
                self.insert_scalar(name, v, name_span)
            }
            Statement::Function {
                name,
                name_span,
                revision,
                parameters,
                output_schema,
                scalar_return,
                body,
                output,
                output_span,
            } => {
                if revision.is_empty() {
                    return Err(error(
                        "E_FUNCTION_REVISION",
                        "Function revision must be nonempty",
                        name_span,
                    ));
                }
                let mut scope = BTreeSet::new();
                for p in &parameters {
                    if !scope.insert(p.name.clone()) {
                        return Err(error("E_DUPLICATE", "Duplicate function parameter", p.span));
                    }
                }
                for s in &body {
                    let (n, span) = declaration(s);
                    if !scope.insert(n.into()) {
                        return Err(error("E_DUPLICATE", "Duplicate function-local name", span));
                    }
                }
                if scalar_return.is_none() && !scope.contains(&output) {
                    return Err(error(
                        "E_FUNCTION_RETURN",
                        "Function must return a declared graph value",
                        output_span,
                    ));
                }
                let mut referenced = BTreeSet::new();
                for s in &body {
                    if let Statement::Apply {
                        function,
                        arguments,
                        ..
                    } = s
                    {
                        referenced.insert(function.clone());
                        for a in arguments {
                            if let ArgumentValue::Function(f) = &a.value {
                                referenced.insert(f.clone());
                            }
                        }
                    }
                }
                let captured = referenced
                    .into_iter()
                    .filter_map(|n| self.functions.get(&n).cloned().map(|f| (n, f)))
                    .collect();
                let parameter_schemas = parameters
                    .iter()
                    .filter_map(|p| p.schema.as_ref().map(|s| (p.name.clone(), s)))
                    .map(|(name, s)| self.schemas.resolve(s).map(|schema| (name, schema)))
                    .collect::<Result<_, _>>()?;
                let output_schema = output_schema
                    .as_ref()
                    .map(|s| self.schemas.resolve(s))
                    .transpose()?;
                let callbacks = parameters
                    .iter()
                    .filter_map(|p| p.callback.as_ref().map(|c| (&p.name, c)))
                    .map(|(n, c)| {
                        let mut schemas = BTreeMap::new();
                        if let Some(s) = &c.input.schema {
                            schemas.insert("input".into(), self.schemas.resolve(s)?);
                        }
                        let output = c
                            .output_schema
                            .as_ref()
                            .map(|s| self.schemas.resolve(s))
                            .transpose()?
                            .map(Knowledge::Exact)
                            .unwrap_or_default();
                        Ok((
                            n.clone(),
                            Signature {
                                parameters: [("input".into(), c.input.kind.clone())].into(),
                                schemas,
                                output,
                                scalar_output: c.output_type.clone(),
                                callbacks: BTreeMap::new(),
                            },
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>, Diagnostic>>()?;
                let definition = Definition {
                    scalar_return,
                    callbacks,
                    parameter_schemas,
                    output_schema,
                    parameters,
                    body,
                    output,
                    output_span,
                    captured,
                };
                for statement in &definition.body {
                    if let Statement::Reason {
                        rule_set,
                        rule_span,
                        ..
                    } = statement
                        && !self.rule_modules.contains(rule_set)
                    {
                        return Err(error(
                            "E_UNKNOWN_RULES",
                            format!("Unknown rule module '{rule_set}'"),
                            *rule_span,
                        ));
                    }
                }
                validate_definition(&definition, &mut self.scalar_budget)?;
                self.functions.insert(
                    name,
                    Arc::new(Closure {
                        definition: Arc::new(definition),
                        bound: BTreeMap::new(),
                    }),
                );
                Ok(())
            }
            Statement::Apply {
                name,
                name_span,
                function,
                function_span,
                arguments,
            } => {
                let closure = self.functions.get(&function).cloned().ok_or_else(|| {
                    error(
                        "E_UNKNOWN_FUNCTION",
                        format!("Unknown function '{function}'"),
                        function_span,
                    )
                })?;
                for value in closure.bound.values() {
                    if let Value::Scalar(v) = value {
                        self.scalar_budget.charge(v.size(), name_span)?;
                    }
                }
                let mut bound = closure.bound.clone();
                for argument in arguments {
                    let parameter = closure
                        .definition
                        .parameters
                        .iter()
                        .find(|p| p.name == argument.name)
                        .ok_or_else(|| {
                            error(
                                "E_UNKNOWN_PARAMETER",
                                format!("Unknown function parameter '{}'", argument.name),
                                argument.span,
                            )
                        })?;
                    if bound.contains_key(&argument.name) {
                        return Err(error(
                            "E_DUPLICATE",
                            format!("Parameter '{}' already bound", argument.name),
                            argument.span,
                        ));
                    }
                    let value = match argument.value {
                        ArgumentValue::Graph(graph) => {
                            let pending = self.graphs.get(&graph).ok_or_else(|| {
                                error(
                                    "E_UNKNOWN_GRAPH",
                                    format!("Unknown graph value '{graph}'"),
                                    argument.span,
                                )
                            })?;
                            if !pending.is_empty() {
                                return Err(error(
                                    "E_UNBOUND_PARAMETER",
                                    "A graph argument must be fully bound",
                                    argument.span,
                                ));
                            }
                            if let Some(expected) =
                                closure.definition.parameter_schemas.get(&parameter.name)
                            {
                                self.schemas
                                    .get(&graph)
                                    .require(expected, argument.value_span)?;
                            }
                            let capture = self.fresh("capture");
                            self.emit(Statement::Lens {
                                name: capture.clone(),
                                name_span: argument.span,
                                source: graph,
                                source_span: argument.span,
                                predicate: None,
                                valid_at: None,
                                include_metadata: false,
                                max_depth: 8,
                            })?;
                            Value::Graph(capture)
                        }
                        ArgumentValue::Function(function) => {
                            let f = self.functions.get(&function).cloned().ok_or_else(|| {
                                error(
                                    "E_UNKNOWN_FUNCTION",
                                    format!("Unknown function value '{function}'"),
                                    argument.span,
                                )
                            })?;
                            callback_matches(
                                &remaining(&f),
                                closure.definition.callbacks.get(&parameter.name),
                                argument.value_span,
                            )?;
                            Value::Function(f)
                        }
                        other => {
                            let expr = argument_scalar(
                                &other,
                                &mut self.scalar_budget,
                                argument.value_span,
                            )?
                            .ok_or_else(|| {
                                error(
                                    "E_PARAMETER_TYPE",
                                    "Expected scalar value",
                                    argument.value_span,
                                )
                            })?;
                            let v = expr.evaluate(
                                &BTreeMap::new(),
                                &self.scalar_values,
                                &mut self.scalar_budget,
                            )?;
                            if !marker_matches(&other, &v.value_type()) {
                                return Err(error(
                                    "E_PARAMETER_TYPE",
                                    "Argument marker differs from scalar type",
                                    argument.value_span,
                                ));
                            }
                            Value::Scalar(v)
                        }
                    };
                    if value.kind() != parameter.kind {
                        return Err(error(
                            "E_PARAMETER_TYPE",
                            format!(
                                "Parameter '{}' requires {:?}",
                                parameter.name, parameter.kind
                            ),
                            argument.span,
                        ));
                    }
                    bound.insert(argument.name, value);
                }
                let applied = Closure {
                    definition: closure.definition.clone(),
                    bound,
                };
                if applied.bound.len() != applied.definition.parameters.len() {
                    self.functions.insert(name, Arc::new(applied));
                    return Ok(());
                }
                let mut scope = BTreeMap::new();
                for (n, f) in &applied.definition.captured {
                    let alias = self.fresh("function");
                    self.functions.insert(alias.clone(), f.clone());
                    scope.insert(n.clone(), alias);
                }
                for (n, v) in &applied.bound {
                    match v {
                        Value::Graph(g) => {
                            scope.insert(n.clone(), g.clone());
                        }
                        Value::Function(f) => {
                            let alias = self.fresh("function");
                            self.functions.insert(alias.clone(), f.clone());
                            scope.insert(n.clone(), alias);
                        }
                        _ => {}
                    }
                }
                for mut statement in applied.definition.body.clone() {
                    let old = declaration(&statement).0.to_owned();
                    let fresh = self.fresh(&old);
                    specialize(
                        &mut statement,
                        &fresh,
                        &scope,
                        &applied.bound,
                        &mut self.scalar_budget,
                    )?;
                    self.process(statement, depth + 1).map_err(|mut e| {
                        e.trace.push(function_span);
                        e
                    })?;
                    scope.insert(old, fresh);
                }
                if let Some(result) = &applied.definition.scalar_return {
                    self.scalar_budget
                        .charge(result.value.retained_bytes(), result.value.span)?;
                    let mut expr = result.value.clone();
                    substitute(&mut expr, &scope, &applied.bound, &mut self.scalar_budget)?;
                    let value = expr
                        .evaluate(
                            &BTreeMap::new(),
                            &self.scalar_values,
                            &mut self.scalar_budget,
                        )
                        .map_err(|mut e| {
                            e.trace.push(function_span);
                            e
                        })?;
                    if value.value_type() != result.value_type {
                        return Err(error(
                            "E_SCALAR_RETURN",
                            "Scalar return type differs",
                            expr.span,
                        ));
                    }
                    return self.insert_scalar(name, value, name_span);
                }
                let source = renamed(
                    &applied.definition.output,
                    &scope,
                    applied.definition.output_span,
                )?;
                if self.functions.contains_key(&source) {
                    return Err(error(
                        "E_FUNCTION_RETURN",
                        "Graph functions must return a graph, not an unapplied function",
                        applied.definition.output_span,
                    ));
                }
                if let Some(expected) = &applied.definition.output_schema {
                    self.schemas
                        .get(&source)
                        .require(expected, applied.definition.output_span)?;
                }
                self.emit(Statement::Lens {
                    name,
                    name_span,
                    source,
                    source_span: applied.definition.output_span,
                    predicate: None,
                    valid_at: None,
                    include_metadata: false,
                    max_depth: 8,
                })
            }
            other => self.emit(other),
        }
    }
}
fn specialize(
    statement: &mut Statement,
    name: &str,
    scope: &BTreeMap<String, String>,
    values: &BTreeMap<String, Value>,
    budget: &mut Budget,
) -> Result<(), Diagnostic> {
    match statement {
        Statement::Value { name: n, value, .. } => {
            *n = name.into();
            substitute(value, scope, values, budget)?;
        }
        Statement::Lens {
            name: n,
            source,
            source_span,
            predicate,
            valid_at,
            ..
        } => {
            *n = name.into();
            *source = renamed(source, scope, *source_span)?;
            if let Some(v) = predicate {
                if let StringExpr::Value(e) = v {
                    substitute(e, scope, values, budget)?;
                } else {
                    string_value(v, values, *source_span)?;
                }
            }
            if let Some(v) = valid_at {
                if let TimeExpr::Value(e) = v {
                    substitute(e, scope, values, budget)?;
                } else {
                    time_value(v, values, *source_span)?;
                }
            }
        }
        Statement::Reason {
            name: n,
            source,
            source_span,
            ..
        }
        | Statement::Bind {
            name: n,
            source,
            source_span,
            ..
        }
        | Statement::Metadata {
            name: n,
            source,
            source_span,
            ..
        } => {
            *n = name.into();
            *source = renamed(source, scope, *source_span)?;
        }
        Statement::Join {
            name: n,
            left,
            left_span,
            right,
            right_span,
            ..
        } => {
            *n = name.into();
            *left = renamed(left, scope, *left_span)?;
            *right = renamed(right, scope, *right_span)?;
        }
        Statement::Algebra {
            name: n,
            source,
            source_span,
            operation,
            ..
        } => {
            *n = name.into();
            *source = renamed(source, scope, *source_span)?;
            if let AlgebraOperation::Union { right, right_span }
            | AlgebraOperation::Diff { right, right_span }
            | AlgebraOperation::Distance {
                right, right_span, ..
            }
            | AlgebraOperation::Transform {
                right, right_span, ..
            } = operation
            {
                *right = renamed(right, scope, *right_span)?;
            }
        }
        Statement::Apply {
            name: n,
            function,
            function_span,
            arguments,
            ..
        } => {
            *n = name.into();
            *function = renamed(function, scope, *function_span)?;
            for a in arguments {
                match &mut a.value {
                    ArgumentValue::Graph(g) | ArgumentValue::Function(g) => {
                        *g = renamed(g, scope, a.span)?
                    }
                    ArgumentValue::String(StringExpr::Value(e))
                    | ArgumentValue::Time(TimeExpr::Value(e))
                    | ArgumentValue::Scalar { value: e, .. } => {
                        substitute(e, scope, values, budget)?
                    }
                    ArgumentValue::String(v) => string_value(v, values, a.span)?,
                    ArgumentValue::Time(v) => time_value(v, values, a.span)?,
                }
            }
        }
        _ => {
            return Err(error(
                "E_FUNCTION_EFFECT",
                "Pure functions cannot declare sources, writes, schemas or nested functions",
                declaration(statement).1,
            ));
        }
    }
    Ok(())
}
#[derive(Clone)]
struct Signature {
    parameters: BTreeMap<String, ParameterKind>,
    schemas: BTreeMap<String, Arc<GraphSchema>>,
    output: Knowledge,
    scalar_output: Option<ScalarType>,
    callbacks: BTreeMap<String, Signature>,
}
fn remaining(closure: &Closure) -> Signature {
    Signature {
        scalar_output: closure
            .definition
            .scalar_return
            .as_ref()
            .map(|r| r.value_type.clone()),
        callbacks: closure.definition.callbacks.clone(),
        parameters: closure
            .definition
            .parameters
            .iter()
            .filter(|p| !closure.bound.contains_key(&p.name))
            .map(|p| (p.name.clone(), p.kind.clone()))
            .collect(),
        schemas: closure
            .definition
            .parameter_schemas
            .iter()
            .filter(|(n, _)| !closure.bound.contains_key(*n))
            .map(|(n, schema)| (n.clone(), schema.clone()))
            .collect(),
        output: closure
            .definition
            .output_schema
            .clone()
            .map(Knowledge::Exact)
            .unwrap_or_default(),
    }
}
fn unary(signature: &Signature) -> bool {
    signature.parameters.len() == 1
        && signature.parameters.get("input") == Some(&ParameterKind::Graph)
        && signature.scalar_output.is_none()
}
fn callback_matches(
    actual: &Signature,
    expected: Option<&Signature>,
    span: Span,
) -> Result<(), Diagnostic> {
    if let Some(expected) = expected {
        if actual.parameters != expected.parameters
            || actual.scalar_output != expected.scalar_output
            || actual.schemas != expected.schemas
        {
            return Err(error(
                "E_FUNCTION_SIGNATURE",
                "Callback requires exact remaining parameter types and schemas",
                span,
            ));
        }
        match (&actual.output, &expected.output) {
            (Knowledge::Exact(a), Knowledge::Exact(b)) if a == b => (),
            (Knowledge::Unknown, Knowledge::Unknown) => (),
            _ => {
                return Err(error(
                    "E_FUNCTION_SIGNATURE",
                    "Callback result schema differs or is unknown",
                    span,
                ));
            }
        }
    } else if !unary(actual) {
        return Err(error(
            "E_FUNCTION_SIGNATURE",
            "Function argument must have one remaining graph input and graph result",
            span,
        ));
    }
    Ok(())
}
fn validate_definition(definition: &Definition, budget: &mut Budget) -> Result<(), Diagnostic> {
    if definition.scalar_return.is_some() {
        for p in &definition.parameters {
            if p.kind == ParameterKind::Graph
                || (p.kind == ParameterKind::Function
                    && p.callback.as_ref().is_none_or(|c| {
                        c.input.kind == ParameterKind::Graph || c.output_type.is_none()
                    }))
            {
                return Err(error(
                    "E_FUNCTION_SCOPE",
                    "Scalar-returning functions cannot accept graph inputs or graph callbacks",
                    p.span,
                ));
            }
        }
        for statement in &definition.body {
            if !matches!(statement, Statement::Value { .. } | Statement::Apply { .. }) {
                return Err(error(
                    "E_FUNCTION_EFFECT",
                    "Scalar-returning functions cannot emit graph operations",
                    declaration(statement).1,
                ));
            }
        }
    }
    let mut scope: BTreeMap<_, _> = definition
        .captured
        .keys()
        .map(|n| (n.clone(), n.clone()))
        .collect();
    let mut functions: BTreeMap<_, _> = definition
        .captured
        .iter()
        .map(|(n, c)| (n.clone(), remaining(c)))
        .collect();
    let mut graphs = BTreeSet::new();
    let mut graph_schemas = BTreeMap::new();
    let mut scalar_params = BTreeMap::new();
    let mut scalar_values = BTreeMap::new();
    let mut constants = BTreeMap::new();
    for p in &definition.parameters {
        match &p.kind {
            ParameterKind::Graph => {
                scope.insert(p.name.clone(), p.name.clone());
                graphs.insert(p.name.clone());
                graph_schemas.insert(
                    p.name.clone(),
                    definition
                        .parameter_schemas
                        .get(&p.name)
                        .cloned()
                        .map(Knowledge::Exact)
                        .unwrap_or_default(),
                );
            }
            ParameterKind::Function => {
                scope.insert(p.name.clone(), p.name.clone());
                functions.insert(
                    p.name.clone(),
                    definition
                        .callbacks
                        .get(&p.name)
                        .cloned()
                        .unwrap_or(Signature {
                            parameters: [("input".into(), ParameterKind::Graph)].into(),
                            schemas: BTreeMap::new(),
                            output: Knowledge::Unknown,
                            scalar_output: None,
                            callbacks: BTreeMap::new(),
                        }),
                );
            }
            k => {
                scalar_params.insert(p.name.clone(), scalar_type(k).expect("scalar parameter"));
            }
        }
    }
    for statement in &definition.body {
        let mut checked = statement.clone();
        let mut failed = None;
        crate::syntax::scalar_expressions(&mut checked, &mut |e| {
            if failed.is_none() {
                failed = e.check_constants(&constants, budget).err();
            }
        });
        if let Some(e) = failed {
            return Err(e);
        }
        let (name, span) = declaration(statement);
        let name = name.to_owned();

        if let Statement::Lens {
            predicate,
            valid_at,
            source_span,
            ..
        } = &statement
        {
            let pairs = [
                predicate
                    .as_ref()
                    .map(|v| (ArgumentValue::String(v.clone()), ScalarType::String)),
                valid_at
                    .as_ref()
                    .map(|v| (ArgumentValue::Time(v.clone()), ScalarType::Time)),
            ];
            for (arg, expected) in pairs.into_iter().flatten() {
                let mut expr = argument_scalar(&arg, budget, *source_span)?.expect("scalar");
                if expr.span == (0, 0) {
                    expr.span = *source_span;
                }
                if expr.infer(&scalar_params, &scalar_values)? != expected {
                    return Err(error(
                        "E_PARAMETER_TYPE",
                        "Lens scalar selector type differs",
                        expr.span,
                    ));
                }
            }
        }
        let require_graph = |name: &str, span| {
            if graphs.contains(name) {
                Ok(())
            } else {
                Err(error(
                    if scope.contains_key(name) {
                        "E_PARAMETER_TYPE"
                    } else {
                        "E_FUNCTION_SCOPE"
                    },
                    format!("'{name}' is not a graph value"),
                    span,
                ))
            }
        };
        match &statement {
            Statement::Value { value, .. } => {
                scalar_values.insert(name.clone(), value.infer(&scalar_params, &scalar_values)?);
                if let Some(v) = value.check_constants(&constants, budget)? {
                    budget.binding(&v, value.span)?;
                    constants.insert(name.clone(), v);
                }
            }
            Statement::Apply {
                function,
                function_span,
                arguments,
                ..
            } => {
                let mut signature = functions.get(function).cloned().ok_or_else(|| {
                    error(
                        "E_FUNCTION_SCOPE",
                        format!("Unknown function '{function}'"),
                        *function_span,
                    )
                })?;
                if definition.scalar_return.is_some()
                    && (signature.scalar_output.is_none()
                        || signature
                            .parameters
                            .values()
                            .any(|k| *k == ParameterKind::Graph))
                {
                    return Err(error(
                        "E_FUNCTION_EFFECT",
                        "Scalar-returning functions cannot specialize graph functions",
                        *function_span,
                    ));
                }
                for a in arguments {
                    let expected = signature.parameters.remove(&a.name).ok_or_else(|| {
                        error(
                            "E_UNKNOWN_PARAMETER",
                            format!("Unknown or repeated argument '{}'", a.name),
                            a.span,
                        )
                    })?;
                    let actual = match &a.value {
                        ArgumentValue::Graph(g) => {
                            require_graph(g, a.span)?;
                            if let Some(expected) = signature.schemas.get(&a.name) {
                                graph_schemas
                                    .get(g)
                                    .cloned()
                                    .unwrap_or_default()
                                    .require(expected, a.value_span)?;
                            }
                            ParameterKind::Graph
                        }
                        ArgumentValue::Function(f) => {
                            let signature = functions.get(f).ok_or_else(|| {
                                error(
                                    "E_UNKNOWN_FUNCTION",
                                    format!("Unknown function '{f}'"),
                                    a.span,
                                )
                            })?;
                            callback_matches(
                                signature,
                                functions
                                    .get(function)
                                    .and_then(|s| s.callbacks.get(&a.name)),
                                a.value_span,
                            )?;
                            ParameterKind::Function
                        }
                        other => {
                            let mut expr = argument_scalar(other, budget, a.value_span)?
                                .expect("scalar argument");
                            if expr.span == (0, 0) {
                                expr.span = a.value_span;
                            }
                            let ty = expr.infer(&scalar_params, &scalar_values)?;
                            if !marker_matches(other, &ty) {
                                return Err(error(
                                    "E_PARAMETER_TYPE",
                                    "Argument marker differs from scalar type",
                                    a.value_span,
                                ));
                            }
                            scalar_kind(&ty)
                        }
                    };
                    if actual != expected {
                        return Err(error(
                            "E_PARAMETER_TYPE",
                            "Function argument has the wrong declared type",
                            a.span,
                        ));
                    }
                }
                if signature.parameters.is_empty() {
                    if let Some(ty) = signature.scalar_output {
                        scalar_values.insert(name.clone(), ty);
                    } else {
                        graphs.insert(name.clone());
                        graph_schemas.insert(name.clone(), signature.output);
                    }
                } else {
                    functions.insert(name.clone(), signature);
                }
            }
            Statement::Reason {
                source,
                source_span,
                ..
            }
            | Statement::Lens {
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
            } => {
                require_graph(source, *source_span)?;
                graphs.insert(name.clone());
            }
            Statement::Join {
                left,
                left_span,
                right,
                right_span,
                ..
            } => {
                require_graph(left, *left_span)?;
                require_graph(right, *right_span)?;
                graphs.insert(name.clone());
            }
            Statement::Algebra {
                source,
                source_span,
                operation,
                ..
            } => {
                require_graph(source, *source_span)?;
                if let AlgebraOperation::Union { right, right_span }
                | AlgebraOperation::Diff { right, right_span }
                | AlgebraOperation::Distance {
                    right, right_span, ..
                }
                | AlgebraOperation::Transform {
                    right, right_span, ..
                } = operation
                {
                    require_graph(right, *right_span)?;
                }
                graphs.insert(name.clone());
            }
            _ => {
                return Err(error(
                    "E_FUNCTION_EFFECT",
                    "Function body contains an effect",
                    span,
                ));
            }
        }
        if let Some((name, knowledge)) = graph_types::transfer(statement, &graph_schemas) {
            graph_schemas.insert(name.into(), knowledge);
        }
        scope.insert(name.clone(), name);
    }
    if let Some(result) = &definition.scalar_return {
        result.value.check_constants(&constants, budget)?;
        if result.value.infer(&scalar_params, &scalar_values)? != result.value_type {
            return Err(error(
                "E_SCALAR_RETURN",
                "Scalar return type differs",
                result.value.span,
            ));
        }
        return Ok(());
    }
    if !graphs.contains(&definition.output) {
        return Err(error(
            "E_FUNCTION_RETURN",
            "Function must return a graph value",
            definition.output_span,
        ));
    }
    if let Some(expected) = &definition.output_schema {
        graph_schemas
            .get(&definition.output)
            .cloned()
            .unwrap_or_default()
            .require(expected, definition.output_span)?;
    }
    Ok(())
}
pub(crate) fn expand(
    program: Program,
) -> Result<(Program, BTreeMap<String, ScalarValue>), Diagnostic> {
    let exported: BTreeSet<String> = program
        .statements
        .iter()
        .map(|s| declaration(s).0.to_owned())
        .collect();
    let mut reserved = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for s in &program.statements {
        let (n, span) = declaration(s);
        if !matches!(s, Statement::Schema { .. }) && !seen.insert(n.to_owned()) {
            return Err(error(
                "E_DUPLICATE",
                format!("Duplicate declaration '{n}'"),
                span,
            ));
        }
        reserved.insert(n.into());
        if let Statement::Transaction { body, .. } = s {
            for s in body {
                reserved.insert(declaration(s).0.into());
            }
        }
    }
    let mut expander = Expander {
        scalar_values: BTreeMap::new(),
        scalar_budget: Budget::default(),
        functions: BTreeMap::new(),
        rule_modules: BTreeSet::new(),
        graphs: BTreeMap::new(),
        schemas: graph_types::State::default(),
        reserved,
        output: Vec::new(),
        counter: 0,
        bytes: 0,
    };
    for statement in program.statements {
        expander.process(statement, 0)?;
    }
    let values = expander
        .scalar_values
        .into_iter()
        .filter(|(n, _)| exported.contains(n))
        .collect();
    Ok((
        Program {
            statements: expander.output,
        },
        values,
    ))
}

/// Remove source locations only at known AST metadata positions. User literals,
/// schema fields and properties (including keys named `span`) remain semantic.
fn normalized_statement(statement: &Statement) -> serde_json::Value {
    fn normalize(value: &mut serde_json::Value) {
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
        ] {
            map.remove(key);
        }
        if let Some(schema) = map
            .get_mut("output_schema")
            .and_then(serde_json::Value::as_object_mut)
        {
            schema.remove("span");
        }
        if let Some(operation) = map
            .get_mut("operation")
            .and_then(serde_json::Value::as_object_mut)
        {
            operation.remove("right_span");
        }
        if let Some(body) = map
            .get_mut("body")
            .and_then(serde_json::Value::as_array_mut)
        {
            for s in body {
                normalize(s);
            }
        }
        for collection in ["parameters", "arguments", "bindings"] {
            if let Some(values) = map
                .get_mut(collection)
                .and_then(serde_json::Value::as_array_mut)
            {
                for value in values {
                    if let Some(fields) = value.as_object_mut() {
                        fields.remove("span");
                        fields.remove("value_span");
                        if let Some(schema) = fields
                            .get_mut("schema")
                            .and_then(serde_json::Value::as_object_mut)
                        {
                            schema.remove("span");
                        }
                    }
                }
            }
        }
        if let Some(items) = map
            .get_mut("items")
            .and_then(serde_json::Value::as_array_mut)
        {
            for item in items {
                if let Some(fields) = item.as_object_mut() {
                    fields.remove("id_span");
                }
            }
        }
        if let Some(op) = map
            .get_mut("operation")
            .and_then(serde_json::Value::as_object_mut)
        {
            op.remove("right_span");
        }
    }
    let mut statement = statement.clone();
    crate::syntax::scalar_expressions(&mut statement, &mut |e| e.spans(&mut |s| *s = (0, 0)));
    crate::syntax::callback_constraints(&mut statement, &mut |s| s.span = (0, 0));
    if let Statement::Function { parameters, .. } = &mut statement {
        for p in parameters {
            if let Some(c) = &mut p.callback {
                c.input.span = (0, 0);
            }
        }
    }
    let mut value = serde_json::to_value(&statement).expect("AST serializes");
    normalize(&mut value);
    value
}
pub(crate) fn source_revisions(
    program: &Program,
) -> Result<Vec<weave_contract::SourceRevision>, Diagnostic> {
    let mut schemas = graph_types::State::default();
    program
        .statements
        .iter()
        .filter_map(|statement| {
            if matches!(statement, Statement::Schema { .. }) {
                return schemas.observe(statement).err().map(Err);
            }
            if let Statement::Rules {
                name,
                name_span,
                definition,
            } = statement
            {
                Some(
                    weave_contract::identity::source_fingerprint(definition)
                        .map(|digest| weave_contract::SourceRevision {
                            name: name.clone(),
                            revision: definition.revision.clone(),
                            digest,
                        })
                        .map_err(|e| error(&e.code, e.message, *name_span)),
                )
            } else if let Statement::Function {
                name,
                name_span,
                revision,
                parameters,
                output_schema,
                ..
            } = statement
            {
                let input_schemas = parameters.iter().filter_map(|p| p.schema.as_ref().map(|schema| (p.name.clone(), schema)))
                    .map(|(name, constraint)| schemas.resolve(constraint).map(|schema| (name, schema.as_ref().clone())))
                    .collect::<Result<BTreeMap<_, _>, _>>();
                let input_schemas = match input_schemas { Ok(s) => s, Err(e) => return Some(Err(e)) };
                let output_schema = match output_schema.as_ref().map(|s| schemas.resolve(s)).transpose() { Ok(s) => s, Err(e) => return Some(Err(e)) };
                let callback_schemas = parameters.iter().filter_map(|p|p.callback.as_ref().map(|c|(&p.name,c))).map(|(n,c)|{
                    let input=c.input.schema.as_ref().map(|s|schemas.resolve(s)).transpose()?;
                    let output=c.output_schema.as_ref().map(|s|schemas.resolve(s)).transpose()?;
                    Ok((n.clone(),serde_json::json!({"input":input.as_deref(),"output":output.as_deref()})))
                }).collect::<Result<BTreeMap<_,_>,Diagnostic>>();
                let callback_schemas=match callback_schemas{Ok(v)=>v,Err(e)=>return Some(Err(e))};
                let mut normalized = normalized_statement(statement);
                if !input_schemas.is_empty() || output_schema.is_some() {
                    normalized = serde_json::json!({"definition":normalized,"input_schemas":input_schemas,"output_schema":output_schema.as_deref()});
                }
                if !callback_schemas.is_empty(){normalized=serde_json::json!({"definition":normalized,"callback_schemas":callback_schemas});}
                Some(
                    weave_contract::identity::source_fingerprint(&normalized)
                        .map(|digest| weave_contract::SourceRevision {
                            name: name.clone(),
                            revision: revision.clone(),
                            digest,
                        })
                        .map_err(|e| error(&e.code, e.message, *name_span)),
                )
            } else {
                None
            }
        })
        .collect()
}
