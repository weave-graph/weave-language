//! Hygienic, bounded specialization of pure graph-function values.
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
    String(String),
    Time(i64),
    Function(Arc<Closure>),
}
impl Value {
    fn kind(&self) -> ParameterKind {
        match self {
            Self::Graph(_) => ParameterKind::Graph,
            Self::String(_) => ParameterKind::String,
            Self::Time(_) => ParameterKind::Time,
            Self::Function(_) => ParameterKind::Function,
        }
    }
}
fn error(code: &str, message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(code, message, span.0, span.1)
}
fn declaration(statement: &Statement) -> (&str, Span) {
    match statement {
        Statement::LiveHandle {
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
            Some(Value::String(s)) => StringExpr::Literal(s.clone()),
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
            Some(Value::Time(t)) => TimeExpr::Literal(*t),
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
    fn emit(&mut self, statement: Statement) -> Result<(), Diagnostic> {
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
        match statement {
            Statement::Function {
                name,
                name_span,
                revision,
                parameters,
                output_schema,
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
                if !scope.contains(&output) {
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
                let definition = Definition {
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
                validate_definition(&definition)?;
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
                            let remaining: Vec<_> = f
                                .definition
                                .parameters
                                .iter()
                                .filter(|p| !f.bound.contains_key(&p.name))
                                .collect();
                            if remaining.len() != 1
                                || remaining[0].name != "input"
                                || remaining[0].kind != ParameterKind::Graph
                            {
                                return Err(error(
                                    "E_FUNCTION_SIGNATURE",
                                    "Function argument must have exactly one remaining graph parameter named 'input'",
                                    argument.span,
                                ));
                            }
                            Value::Function(f)
                        }
                        ArgumentValue::String(StringExpr::Literal(s)) => Value::String(s),
                        ArgumentValue::Time(TimeExpr::Literal(t)) => Value::Time(t),
                        _ => {
                            return Err(error(
                                "E_FUNCTION_SCOPE",
                                "Value parameters are only available inside function bodies",
                                argument.span,
                            ));
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
                    specialize(&mut statement, &fresh, &scope, &applied.bound)?;
                    self.process(statement, depth + 1)?;
                    scope.insert(old, fresh);
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
) -> Result<(), Diagnostic> {
    match statement {
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
                string_value(v, values, *source_span)?;
            }
            if let Some(v) = valid_at {
                time_value(v, values, *source_span)?;
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
}
fn remaining(closure: &Closure) -> Signature {
    Signature {
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
}
fn validate_definition(definition: &Definition) -> Result<(), Diagnostic> {
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
    let mut values = BTreeMap::new();
    for p in &definition.parameters {
        match p.kind {
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
                    Signature {
                        parameters: [("input".into(), ParameterKind::Graph)].into(),
                        schemas: BTreeMap::new(),
                        output: Knowledge::Unknown,
                    },
                );
            }
            ParameterKind::String => {
                values.insert(p.name.clone(), Value::String(String::new()));
            }
            ParameterKind::Time => {
                values.insert(p.name.clone(), Value::Time(0));
            }
        }
    }
    for mut statement in definition.body.clone() {
        let (name, span) = declaration(&statement);
        let name = name.to_owned();
        specialize(&mut statement, &name, &scope, &values)?;
        let require_graph = |name: &str, span| {
            if graphs.contains(name) {
                Ok(())
            } else {
                Err(error(
                    "E_PARAMETER_TYPE",
                    format!("'{name}' is not a graph value"),
                    span,
                ))
            }
        };
        match &statement {
            Statement::Apply {
                function,
                function_span,
                arguments,
                ..
            } => {
                let mut signature = functions.get(function).cloned().ok_or_else(|| {
                    error(
                        "E_UNKNOWN_FUNCTION",
                        format!("Unknown function '{function}'"),
                        *function_span,
                    )
                })?;
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
                            if !unary(signature) {
                                return Err(error(
                                    "E_FUNCTION_SIGNATURE",
                                    "Function argument must have one graph input parameter",
                                    a.span,
                                ));
                            }
                            ParameterKind::Function
                        }
                        ArgumentValue::String(_) => ParameterKind::String,
                        ArgumentValue::Time(_) => ParameterKind::Time,
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
                    graphs.insert(name.clone());
                    graph_schemas.insert(name.clone(), signature.output);
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
        if let Some((name, knowledge)) = graph_types::transfer(&statement, &graph_schemas) {
            graph_schemas.insert(name.into(), knowledge);
        }
        scope.insert(name.clone(), name);
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
pub(crate) fn expand(program: Program) -> Result<Program, Diagnostic> {
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
    Ok(Program {
        statements: expander.output,
    })
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
    let mut value = serde_json::to_value(statement).expect("AST serializes");
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
                let mut normalized = normalized_statement(statement);
                if !input_schemas.is_empty() || output_schema.is_some() {
                    normalized = serde_json::json!({"definition":normalized,"input_schemas":input_schemas,"output_schema":output_schema.as_deref()});
                }
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
