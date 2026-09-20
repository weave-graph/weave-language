use crate::scalars::{
    LiteralExpr, ScalarExpr, ScalarExpression, ScalarReturn, ScalarType as ValueType, ScalarValue,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use weave_contract::{
    EdgeSchema, EntitySpace, GraphProfile, GraphSchema, MetadataHost, MetadataValue, NodeSchema,
    PropertySchema, ScalarType,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub start: usize,
    pub end: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<Span>,
}
impl Diagnostic {
    pub(crate) fn new(code: &str, message: impl Into<String>, start: usize, end: usize) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            start,
            end,
            trace: vec![],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub statements: Vec<Statement>,
}
pub type Span = (usize, usize);
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Statement {
    Value {
        name: String,
        name_span: Span,
        value: ScalarExpr,
    },
    ModuleHeader {
        name: String,
        name_span: Span,
        revision: String,
    },
    Import {
        name: String,
        name_span: Span,
        module_id: String,
        revision: String,
        digest: String,
        digest_span: Span,
    },
    LiveHandle {
        name: String,
        name_span: Span,
        graph: String,
        branch: String,
    },
    Pin {
        name: String,
        name_span: Span,
        source: String,
        source_span: Span,
        valid_at: Option<i64>,
        metadata_depth: Option<u32>,
    },
    ContextSchema {
        name: String,
        name_span: Span,
        definition: weave_contract::context_axes::ContextSchema,
    },
    ContextValue {
        name: String,
        name_span: Span,
        schema: String,
        schema_span: Span,
        attribution: String,
        axes: Vec<ContextAxisBinding>,
    },
    TypedContext {
        name: String,
        name_span: Span,
        source: String,
        source_span: Span,
        reference: weave_contract::GraphRef,
        schema: String,
        schema_span: Span,
    },
    NativeService {
        name: String,
        name_span: Span,
        service: NativeService,
    },
    Rules {
        name: String,
        name_span: Span,
        definition: weave_contract::RuleSet,
    },
    Reason {
        name: String,
        name_span: Span,
        source: String,
        source_span: Span,
        rule_set: String,
        rule_span: Span,
    },
    Function {
        name: String,
        name_span: Span,
        revision: String,
        parameters: Vec<FunctionParameter>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_schema: Option<SchemaConstraint>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scalar_return: Option<ScalarReturn>,
        body: Vec<Statement>,
        output: String,
        output_span: Span,
    },
    Apply {
        name: String,
        name_span: Span,
        function: String,
        function_span: Span,
        arguments: Vec<Argument>,
    },
    Algebra {
        name: String,
        name_span: Span,
        source: String,
        source_span: Span,
        operation: AlgebraOperation,
    },
    Transaction {
        name: String,
        name_span: Span,
        body: Vec<Statement>,
    },
    Metadata {
        name: String,
        name_span: Span,
        source: String,
        source_span: Span,
        host: MetadataHost,
        key: String,
    },
    Schema {
        name: String,
        name_span: Span,
        definition: GraphSchema,
    },
    Join {
        name: String,
        name_span: Span,
        left: String,
        left_span: Span,
        right: String,
        right_span: Span,
        predicate: String,
    },
    Bind {
        name: String,
        name_span: Span,
        source: String,
        source_span: Span,
        bindings: Vec<Binding>,
    },
    Graph {
        name: String,
        name_span: Span,
        items: Vec<Item>,
        schema: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        schema_span: Option<Span>,
        profile: GraphProfile,
        branch: String,
        expected_head: Option<String>,
    },
    Use {
        name: String,
        name_span: Span,
        graph: String,
        revision: Option<String>,
    },
    Lens {
        name: String,
        name_span: Span,
        source: String,
        source_span: Span,
        predicate: Option<StringExpr>,
        valid_at: Option<TimeExpr>,
        include_metadata: bool,
        max_depth: u32,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextAxisBinding {
    pub name: String,
    pub name_span: Span,
    pub value: serde_json::Value,
    pub value_span: Span,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "service", rename_all = "snake_case")]
pub enum NativeService {
    Identity {
        selection: weave_contract::IdentityResolve,
    },
    Cluster {
        selection: weave_contract::ClusterRequest,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SchemaConstraint {
    pub name: String,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FunctionParameter {
    pub name: String,
    pub span: Span,
    pub kind: ParameterKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaConstraint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback: Option<CallbackSignature>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CallbackSignature {
    pub input: Box<FunctionParameter>,
    pub output_schema: Option<SchemaConstraint>,
    pub output_type: Option<ValueType>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterKind {
    Graph,
    String,
    Time,
    Function,
    Scalar(ValueType),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Argument {
    pub name: String,
    pub span: Span,
    pub value_span: Span,
    pub value: ArgumentValue,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ArgumentValue {
    Graph(String),
    Function(String),
    String(StringExpr),
    Time(TimeExpr),
    Scalar { kind: String, value: ScalarExpr },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operator", rename_all = "snake_case")]
pub enum AlgebraOperation {
    Distance {
        left_assertion: String,
        right: String,
        right_span: Span,
        right_assertion: String,
        valid_at: i64,
    },
    Transform {
        left_assertion: String,
        right: String,
        right_span: Span,
        right_assertion: String,
        valid_at: i64,
    },
    ProjectAxes {
        assertion_id: String,
        axes: Vec<usize>,
        projection_revision: String,
        valid_at: i64,
    },
    Explain,
    Counterparts {
        selection: weave_contract::CounterpartSelection,
    },
    Context {
        selection: weave_contract::ContextSelection,
    },
    Union {
        right: String,
        right_span: Span,
    },
    Diff {
        right: String,
        right_span: Span,
    },
    Project {
        node_ids: Vec<String>,
        edge_ids: Vec<String>,
    },
    Support {
        predicate: String,
        from: EntitySpace,
        to: EntitySpace,
        valid_at: i64,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StringExpr {
    Value(ScalarExpr),
    Literal(String),
    Parameter(String),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TimeExpr {
    Value(ScalarExpr),
    Literal(i64),
    Parameter(String),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    pub name: String,
    pub span: Span,
    pub value: BindingValue,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BindingValue {
    String(String),
    Time(i64),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item {
    Structural {
        id: String,
        id_span: Span,
        type_id: Option<String>,
        from: String,
        to: String,
        predicate: String,
        metadata: Vec<Metadata>,
        properties: BTreeMap<String, LiteralExpr>,
    },
    Claim {
        id: String,
        id_span: Span,
        edge_id: String,
        source: String,
        context: Option<weave_contract::GraphRef>,
        negative: bool,
        valid_from: i64,
        valid_to: Option<i64>,
        metadata: Vec<Metadata>,
        properties: BTreeMap<String, LiteralExpr>,
    },
    Attachment {
        context: Option<weave_contract::GraphRef>,
        id: String,
        id_span: Span,
        host: MetadataHost,
        key: String,
        value: MetadataValue,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        literal: Option<LiteralExpr>,
        valid_from: i64,
        valid_to: Option<i64>,
        required: bool,
    },
    Node {
        id: String,
        id_span: Span,
        type_id: Option<String>,
        entity: String,
        space: String,
        metadata: Vec<Metadata>,
        properties: BTreeMap<String, LiteralExpr>,
    },
    Edge {
        id: String,
        id_span: Span,
        type_id: Option<String>,
        from: String,
        to: String,
        predicate: String,
        negative: bool,
        valid_from: i64,
        valid_to: Option<i64>,
        metadata: Vec<Metadata>,
        properties: BTreeMap<String, LiteralExpr>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    pub graph: String,
    pub revision: String,
}
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Word(String),
    String(String),
    Number(i64),
    Float(f64),
    Symbol(char),
    End,
}
#[derive(Clone, Debug)]
struct Token {
    kind: Kind,
    start: usize,
    end: usize,
}
fn lex(source: &str) -> Result<Vec<Token>, Diagnostic> {
    if source.len() > 1_048_576 {
        return Err(Diagnostic::new(
            "E_BUDGET",
            "Source exceeds 1 MiB limit",
            0,
            source.len(),
        ));
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < source.len() {
        let c = source[i..].chars().next().unwrap();
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        if source[i..].starts_with("//") {
            i = source[i..]
                .find('\n')
                .map(|n| i + n)
                .unwrap_or(source.len());
            continue;
        }
        let start = i;
        let kind = if c == '"' {
            i += 1;
            let mut escaped = false;
            let mut closed = false;
            while i < source.len() {
                let ch = source[i..].chars().next().unwrap();
                i += ch.len_utf8();
                if !escaped && ch == '"' {
                    closed = true;
                    break;
                }
                escaped = ch == '\\' && !escaped;
            }
            if !closed {
                return Err(Diagnostic::new("E_STRING", "Unterminated string", start, i));
            }
            Kind::String(
                serde_json::from_str(&source[start..i])
                    .map_err(|e| Diagnostic::new("E_STRING", e.to_string(), start, i))?,
            )
        } else if c.is_ascii_digit() || c == '-' {
            i += 1;
            while i < source.len() && source.as_bytes()[i].is_ascii_digit() {
                i += 1;
            }
            let mut floating = false;
            if i < source.len() && source.as_bytes()[i] == b'.' {
                floating = true;
                i += 1;
                let digits = i;
                while i < source.len() && source.as_bytes()[i].is_ascii_digit() {
                    i += 1;
                }
                if i == digits {
                    return Err(Diagnostic::new(
                        "E_NUMBER",
                        "Fraction requires digits",
                        start,
                        i,
                    ));
                }
            }
            if i < source.len() && matches!(source.as_bytes()[i], b'e' | b'E') {
                floating = true;
                i += 1;
                if i < source.len() && matches!(source.as_bytes()[i], b'+' | b'-') {
                    i += 1;
                }
                let digits = i;
                while i < source.len() && source.as_bytes()[i].is_ascii_digit() {
                    i += 1;
                }
                if i == digits {
                    return Err(Diagnostic::new(
                        "E_NUMBER",
                        "Exponent requires digits",
                        start,
                        i,
                    ));
                }
            }
            if floating {
                let number: f64 = source[start..i].parse().map_err(|_| {
                    Diagnostic::new("E_NUMBER", "Expected finite binary64 number", start, i)
                })?;
                if !number.is_finite() {
                    return Err(Diagnostic::new(
                        "E_NUMBER",
                        "Number exceeds finite binary64 range",
                        start,
                        i,
                    ));
                }
                Kind::Float(number)
            } else {
                Kind::Number(source[start..i].parse().map_err(|_| {
                    Diagnostic::new("E_NUMBER", "Expected a signed 64-bit integer", start, i)
                })?)
            }
        } else if c.is_ascii_alphabetic() || c == '_' {
            i += 1;
            while i < source.len() {
                if source.as_bytes()[i].is_ascii_alphanumeric() || source.as_bytes()[i] == b'_' {
                    i += 1;
                } else if source[i..].starts_with("::")
                    && source
                        .as_bytes()
                        .get(i + 2)
                        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
                {
                    i += 2;
                } else {
                    break;
                }
            }
            Kind::Word(source[start..i].into())
        } else if "{}[];(),:".contains(c) {
            i += 1;
            Kind::Symbol(c)
        } else {
            return Err(Diagnostic::new(
                "E_TOKEN",
                format!("Unexpected character {c:?}"),
                start,
                start + c.len_utf8(),
            ));
        };
        out.push(Token {
            kind,
            start,
            end: i,
        });
        if out.len() > 100_000 {
            return Err(Diagnostic::new(
                "E_BUDGET",
                "Token limit exceeded",
                start,
                i,
            ));
        }
    }
    out.push(Token {
        kind: Kind::End,
        start: i,
        end: i,
    });
    Ok(out)
}
struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}
impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }
    fn take(&mut self) -> Token {
        let t = self.peek().clone();
        if t.kind != Kind::End {
            self.cursor += 1;
        }
        t
    }
    fn error(&self, message: &str) -> Diagnostic {
        let t = self.peek();
        Diagnostic::new("E_PARSE", message, t.start, t.end)
    }
    fn word(&mut self, value: &str) -> Result<(), Diagnostic> {
        if self.peek().kind == Kind::Word(value.into()) {
            self.take();
            Ok(())
        } else {
            Err(self.error(&format!("Expected '{value}'")))
        }
    }
    fn symbol(&mut self, value: char) -> Result<(), Diagnostic> {
        if self.peek().kind == Kind::Symbol(value) {
            self.take();
            Ok(())
        } else {
            Err(self.error(&format!("Expected '{value}'")))
        }
    }
    fn name(&mut self) -> Result<String, Diagnostic> {
        let t = self.take();
        if let Kind::Word(s) = t.kind {
            Ok(s)
        } else {
            Err(Diagnostic::new(
                "E_PARSE",
                "Expected identifier",
                t.start,
                t.end,
            ))
        }
    }
    fn string(&mut self) -> Result<String, Diagnostic> {
        let t = self.take();
        if let Kind::String(s) = t.kind {
            if s.is_empty() {
                return Err(Diagnostic::new(
                    "E_ID",
                    "Identifiers cannot be empty",
                    t.start,
                    t.end,
                ));
            }
            Ok(s)
        } else {
            Err(Diagnostic::new(
                "E_PARSE",
                "Expected quoted string",
                t.start,
                t.end,
            ))
        }
    }
    fn selector_string(&mut self) -> Result<String, Diagnostic> {
        let token = self.peek().clone();
        let value = self.string()?;
        if value.len() > 512 || value.chars().any(char::is_control) {
            return Err(Diagnostic::new(
                "E_ID",
                "Selector requires 1 to 512 bytes without controls",
                token.start,
                token.end,
            ));
        }
        Ok(value)
    }
    fn explicit_context(&mut self) -> Result<weave_contract::ContextSelection, Diagnostic> {
        self.word("context")?;
        if self.peek().kind == Kind::Word("default".into()) {
            self.take();
            Ok(weave_contract::ContextSelection::Default)
        } else {
            self.word("graph")?;
            let graph_id = self.selector_string()?;
            self.word("revision")?;
            let revision = self.selector_string()?;
            Ok(weave_contract::ContextSelection::Pinned {
                reference: weave_contract::GraphRef { graph_id, revision },
            })
        }
    }
    fn number(&mut self) -> Result<i64, Diagnostic> {
        let t = self.take();
        if let Kind::Number(n) = t.kind {
            Ok(n)
        } else {
            Err(Diagnostic::new(
                "E_PARSE",
                "Expected integer",
                t.start,
                t.end,
            ))
        }
    }
    fn unit_descriptor(&mut self) -> Result<weave_contract::quantity::UnitDescriptor, Diagnostic> {
        let token = self.peek().clone();
        self.word("dimension")?;
        let dimension = self.string()?;
        self.word("unit")?;
        let unit = self.string()?;
        self.word("revision")?;
        let revision = self.string()?;
        weave_contract::quantity::UnitDescriptor::new(dimension, unit, revision).map_err(|e| {
            Diagnostic::new(
                "E_QUANTITY_DESCRIPTOR",
                e.to_string(),
                token.start,
                token.end,
            )
        })
    }
    fn decimal_literal(&mut self) -> Result<weave_contract::decimal::Decimal, Diagnostic> {
        let token = self.peek().clone();
        self.string()?
            .parse()
            .map_err(|e: weave_contract::decimal::DecimalError| {
                Diagnostic::new("E_DECIMAL", e.to_string(), token.start, token.end)
            })
    }
    fn numeric_literal(
        &mut self,
        operator: &str,
        depth: usize,
        span: Span,
    ) -> Result<serde_json::Value, Diagnostic> {
        self.symbol('(')?;
        let left = self.literal(depth + 1)?;
        self.symbol(',')?;
        let right = self.literal(depth + 1)?;
        self.symbol(')')?;
        let error = |message: String| Diagnostic::new("E_NUMERIC", message, span.0, span.1);
        if operator.starts_with("decimal_") {
            let a = weave_contract::decimal::Decimal::deserialize(&left)
                .map_err(|e| error(e.to_string()))?;
            let b = weave_contract::decimal::Decimal::deserialize(&right)
                .map_err(|e| error(e.to_string()))?;
            let result = match operator {
                "decimal_add" => a.checked_add(b),
                "decimal_sub" => a.checked_sub(b),
                "decimal_mul" => a.checked_mul(b),
                "decimal_div" => a.checked_div(b),
                _ => unreachable!("parser selects a known numeric operator"),
            }
            .map_err(|e| error(e.to_string()))?;
            return Ok(serde_json::json!(result));
        }
        let a = weave_contract::quantity::Quantity::deserialize(&left)
            .map_err(|e| error(e.to_string()))?;
        let result = match operator {
            "quantity_add" | "quantity_sub" => {
                let b = weave_contract::quantity::Quantity::deserialize(&right)
                    .map_err(|e| error(e.to_string()))?;
                if operator == "quantity_add" {
                    a.checked_add(&b)
                } else {
                    a.checked_sub(&b)
                }
            }
            "quantity_scale" | "quantity_div" => {
                let b = weave_contract::decimal::Decimal::deserialize(&right)
                    .map_err(|e| error(e.to_string()))?;
                if operator == "quantity_scale" {
                    a.checked_mul(b)
                } else {
                    a.checked_div(b)
                }
            }
            "quantity_convert" => {
                let conversion = weave_contract::quantity::RationalConversion::deserialize(&right)
                    .map_err(|e| error(e.to_string()))?;
                a.convert(&conversion)
            }
            _ => unreachable!("parser selects a known numeric operator"),
        }
        .map_err(|e| error(e.to_string()))?;
        Ok(serde_json::json!(result))
    }
    fn literal(&mut self, depth: usize) -> Result<serde_json::Value, Diagnostic> {
        let token = self.take();
        if depth > 32 {
            return Err(Diagnostic::new(
                "E_BUDGET",
                "Structured literal nesting exceeds 32",
                token.start,
                token.end,
            ));
        }
        Ok(match token.kind {
            Kind::String(v) => serde_json::Value::String(v),
            Kind::Number(v) => serde_json::json!(v),
            Kind::Float(v) => serde_json::json!(v),
            Kind::Word(v) if v == "decimal" => serde_json::json!(self.decimal_literal()?),
            Kind::Word(v) if v == "quantity" => {
                let amount = self.decimal_literal()?;
                let unit = self.unit_descriptor()?;
                serde_json::json!(weave_contract::quantity::Quantity::new(amount, unit))
            }
            Kind::Word(v)
                if matches!(
                    v.as_str(),
                    "decimal_add"
                        | "decimal_sub"
                        | "decimal_mul"
                        | "decimal_div"
                        | "quantity_add"
                        | "quantity_sub"
                        | "quantity_scale"
                        | "quantity_div"
                        | "quantity_convert"
                ) =>
            {
                self.numeric_literal(&v, depth, (token.start, token.end))?
            }
            Kind::Word(v) if v == "true" => serde_json::Value::Bool(true),
            Kind::Word(v) if v == "false" => serde_json::Value::Bool(false),
            Kind::Word(v) if v == "null" => serde_json::Value::Null,
            Kind::Symbol('[') => {
                let mut values = Vec::new();
                if self.peek().kind != Kind::Symbol(']') {
                    loop {
                        values.push(self.literal(depth + 1)?);
                        if self.peek().kind != Kind::Symbol(',') {
                            break;
                        }
                        self.take();
                    }
                }
                self.symbol(']')?;
                serde_json::Value::Array(values)
            }
            Kind::Symbol('{') => {
                let mut values = serde_json::Map::new();
                if self.peek().kind != Kind::Symbol('}') {
                    loop {
                        let key = self.take();
                        let Kind::String(name) = key.kind else {
                            return Err(Diagnostic::new(
                                "E_PROPERTY_TYPE",
                                "Object keys must be quoted strings",
                                key.start,
                                key.end,
                            ));
                        };
                        self.symbol(':')?;
                        let value = self.literal(depth + 1)?;
                        if values.insert(name, value).is_some() {
                            return Err(Diagnostic::new(
                                "E_DUPLICATE",
                                "Duplicate object key",
                                key.start,
                                key.end,
                            ));
                        }
                        if self.peek().kind != Kind::Symbol(',') {
                            break;
                        }
                        self.take();
                    }
                }
                self.symbol('}')?;
                serde_json::Value::Object(values)
            }
            _ => {
                return Err(Diagnostic::new(
                    "E_PROPERTY_TYPE",
                    "Expected a structured literal or finite scalar",
                    token.start,
                    token.end,
                ));
            }
        })
    }
    fn scalar_type(&mut self) -> Result<ValueType, Diagnostic> {
        Ok(match self.name()?.as_str() {
            "boolean" => ValueType::Boolean,
            "integer" => ValueType::Integer,
            "string" => ValueType::String,
            "time" => ValueType::Time,
            "decimal" => ValueType::Decimal,
            "quantity" => ValueType::Quantity(self.unit_descriptor()?),
            _ => return Err(self.error("Expected ordinary scalar type")),
        })
    }
    fn scalar_expr(&mut self, depth: usize) -> Result<ScalarExpr, Diagnostic> {
        let token = self.take();
        if depth > 32 {
            return Err(Diagnostic::new(
                "E_SCALAR_BUDGET",
                "Scalar expression depth exceeds 32",
                token.start,
                token.end,
            ));
        }
        let expression = match token.kind {
            Kind::String(v) => ScalarExpression::Literal(ScalarValue::String(v)),
            Kind::Number(v) => ScalarExpression::Literal(ScalarValue::Integer(v)),
            Kind::Word(v) if v == "true" || v == "false" => {
                ScalarExpression::Literal(ScalarValue::Boolean(v == "true"))
            }
            Kind::Word(v) if v == "time" => {
                ScalarExpression::Literal(ScalarValue::Time(self.number()?))
            }
            Kind::Word(v) if v == "decimal" => {
                ScalarExpression::Literal(ScalarValue::Decimal(self.decimal_literal()?))
            }
            Kind::Word(v) if v == "quantity" => {
                let amount = self.decimal_literal()?;
                ScalarExpression::Literal(ScalarValue::Quantity(
                    weave_contract::quantity::Quantity::new(amount, self.unit_descriptor()?),
                ))
            }
            Kind::Word(v) if v == "param" => ScalarExpression::Parameter(self.name()?),
            Kind::Word(v) if v == "value" => ScalarExpression::Value(self.name()?),
            Kind::Word(operator) if operator == "quantity_convert" => {
                self.symbol('(')?;
                let input = Box::new(self.scalar_expr(depth + 1)?);
                self.symbol(',')?;
                let raw = self.literal(depth + 1)?;
                let conversion = serde_json::from_value(raw).map_err(|e| {
                    Diagnostic::new("E_NUMERIC", e.to_string(), token.start, token.end)
                })?;
                self.symbol(')')?;
                ScalarExpression::Convert {
                    input,
                    conversion: Box::new(conversion),
                }
            }
            Kind::Word(operator) => {
                self.symbol('(')?;
                let mut arguments = Vec::new();
                while self.peek().kind != Kind::Symbol(')') {
                    if arguments.len() >= 2 {
                        return Err(self.error("Scalar builtins accept at most two arguments"));
                    }
                    arguments.push(self.scalar_expr(depth + 1)?);
                    if self.peek().kind != Kind::Symbol(')') {
                        self.symbol(',')?;
                    }
                }
                self.symbol(')')?;
                ScalarExpression::Call {
                    operator,
                    arguments,
                }
            }
            _ => {
                return Err(Diagnostic::new(
                    "E_SCALAR_TYPE",
                    "Expected typed scalar expression (no Float coercion)",
                    token.start,
                    token.end,
                ));
            }
        };
        if let ScalarExpression::Parameter(n) | ScalarExpression::Value(n) = &expression
            && (n.starts_with("__weave_function_") || n.starts_with("__weave_module_"))
        {
            return Err(Diagnostic::new(
                "E_FUNCTION_SCOPE",
                "Compiler-generated scalar names are not source references",
                token.start,
                self.tokens[self.cursor.saturating_sub(1)].end,
            ));
        }
        Ok(ScalarExpr {
            span: (token.start, self.tokens[self.cursor.saturating_sub(1)].end),
            expression,
        })
    }
    fn template_literal(&mut self, depth: usize) -> Result<LiteralExpr, Diagnostic> {
        if depth > 32 {
            let t = self.peek();
            return Err(Diagnostic::new(
                "E_BUDGET",
                "Structured literal depth exceeds 32",
                t.start,
                t.end,
            ));
        }
        if self.peek().kind == Kind::Word("value".into()) {
            return Ok(LiteralExpr::Scalar(self.scalar_expr(0)?));
        }
        if self.peek().kind == Kind::Symbol('[') {
            self.take();
            let mut values = Vec::new();
            while self.peek().kind != Kind::Symbol(']') {
                values.push(self.template_literal(depth + 1)?);
                if self.peek().kind != Kind::Symbol(']') {
                    self.symbol(',')?;
                }
            }
            self.symbol(']')?;
            return Ok(LiteralExpr::Array(values));
        }
        if self.peek().kind == Kind::Symbol('{') {
            self.take();
            let mut values = BTreeMap::new();
            while self.peek().kind != Kind::Symbol('}') {
                let key_span = (self.peek().start, self.peek().end);
                let key = self.string()?;
                self.symbol(':')?;
                let value = self.template_literal(depth + 1)?;
                if values.insert(key, value).is_some() {
                    return Err(Diagnostic::new(
                        "E_DUPLICATE",
                        "Duplicate object key",
                        key_span.0,
                        key_span.1,
                    ));
                }
                if self.peek().kind != Kind::Symbol('}') {
                    self.symbol(',')?;
                }
            }
            self.symbol('}')?;
            return Ok(LiteralExpr::Object(values));
        }
        Ok(LiteralExpr::Json(self.literal(depth)?))
    }
    fn function_parameter(&mut self, callback: bool) -> Result<FunctionParameter, Diagnostic> {
        let token = self.take();
        let Kind::Word(label) = token.kind else {
            return Err(self.error("Expected parameter type"));
        };
        let mut kind = match label.as_str() {
            "graph" => ParameterKind::Graph,
            "string" => ParameterKind::String,
            "time" => ParameterKind::Time,
            "function" if !callback => ParameterKind::Function,
            "boolean" => ParameterKind::Scalar(ValueType::Boolean),
            "integer" => ParameterKind::Scalar(ValueType::Integer),
            "decimal" => ParameterKind::Scalar(ValueType::Decimal),
            "quantity" => ParameterKind::Scalar(ValueType::Boolean),
            _ => return Err(self.error("Expected graph, scalar or function parameter")),
        };
        let span = (self.peek().start, self.peek().end);
        let name = self.name()?;
        if label == "quantity" {
            kind = ParameterKind::Scalar(ValueType::Quantity(self.unit_descriptor()?));
        }
        let schema = if self.peek().kind == Kind::Word("schema".into()) {
            self.take();
            if kind != ParameterKind::Graph {
                return Err(self.error("Only graph parameters accept schema constraints"));
            }
            let span = (self.peek().start, self.peek().end);
            Some(SchemaConstraint {
                name: self.name()?,
                span,
            })
        } else {
            None
        };
        let signature = if kind == ParameterKind::Function && self.peek().kind == Kind::Symbol('(')
        {
            self.take();
            let input = self.function_parameter(true)?;
            if input.name != "input" {
                return Err(Diagnostic::new(
                    "E_FUNCTION_SIGNATURE",
                    "Callback parameter must be named input",
                    input.span.0,
                    input.span.1,
                ));
            }
            self.symbol(')')?;
            self.word("returns")?;
            let (output_schema, output_type) = self.function_result()?;
            Some(CallbackSignature {
                input: Box::new(input),
                output_schema,
                output_type,
            })
        } else {
            None
        };
        Ok(FunctionParameter {
            name,
            span,
            kind,
            schema,
            callback: signature,
        })
    }
    fn function_result(
        &mut self,
    ) -> Result<(Option<SchemaConstraint>, Option<ValueType>), Diagnostic> {
        if self.peek().kind == Kind::Word("graph".into()) {
            self.take();
            let schema = if self.peek().kind == Kind::Word("schema".into()) {
                self.take();
                let span = (self.peek().start, self.peek().end);
                Some(SchemaConstraint {
                    name: self.name()?,
                    span,
                })
            } else {
                None
            };
            Ok((schema, None))
        } else {
            Ok((None, Some(self.scalar_type()?)))
        }
    }
    fn metadata(&mut self) -> Result<(Vec<Metadata>, BTreeMap<String, LiteralExpr>), Diagnostic> {
        let mut result = Vec::new();
        let mut properties = BTreeMap::new();
        loop {
            if self.peek().kind == Kind::Word("metadata".into()) {
                self.word("metadata")?;
                self.word("graph")?;
                let graph = self.string()?;
                self.word("revision")?;
                let revision = self.string()?;
                result.push(Metadata { graph, revision });
            } else if self.peek().kind == Kind::Word("property".into()) {
                self.word("property")?;
                let key_token = self.peek().clone();
                let key = self.string()?;
                let value = self.template_literal(0)?;
                if properties.insert(key, value).is_some() {
                    return Err(Diagnostic::new(
                        "E_DUPLICATE",
                        "Duplicate property",
                        key_token.start,
                        key_token.end,
                    ));
                }
            } else {
                break;
            }
        }
        Ok((result, properties))
    }
    fn schema_properties(
        &mut self,
    ) -> Result<(BTreeMap<String, PropertySchema>, bool), Diagnostic> {
        self.symbol('{')?;
        let mut properties = BTreeMap::new();
        let mut open = false;
        while self.peek().kind != Kind::Symbol('}') {
            if self.peek().kind == Kind::Word("open".into()) {
                self.word("open")?;
                if open {
                    return Err(self.error("Duplicate open declaration"));
                }
                open = true;
                self.symbol(';')?;
                continue;
            }
            self.word("property")?;
            let token = self.peek().clone();
            let key = self.string()?;
            let value_type = match self.name()?.as_str() {
                "string" => ScalarType::String,
                "integer" => ScalarType::Integer,
                "boolean" => ScalarType::Boolean,
                "float" => ScalarType::Float,
                "decimal" => ScalarType::Decimal,
                "quantity" => ScalarType::Quantity(self.unit_descriptor()?),
                _ => {
                    return Err(self.error(
                        "Schema type must be string, integer, float, decimal, quantity or boolean",
                    ));
                }
            };
            let required = match self.name()?.as_str() {
                "required" => true,
                "optional" => false,
                _ => return Err(self.error("Schema property must be required or optional")),
            };
            let nullable = if self.peek().kind == Kind::Word("nullable".into()) {
                self.word("nullable")?;
                true
            } else {
                false
            };
            self.symbol(';')?;
            if properties
                .insert(
                    key,
                    PropertySchema {
                        value_type,
                        required,
                        nullable,
                    },
                )
                .is_some()
            {
                return Err(Diagnostic::new(
                    "E_DUPLICATE",
                    "Duplicate schema property",
                    token.start,
                    token.end,
                ));
            }
        }
        self.symbol('}')?;
        Ok((properties, open))
    }
    fn schema(&mut self, name: String, name_span: Span) -> Result<Statement, Diagnostic> {
        self.word("revision")?;
        let revision = self.string()?;
        self.symbol('{')?;
        let mut nodes = BTreeMap::new();
        let mut edges = BTreeMap::new();
        while self.peek().kind != Kind::Symbol('}') {
            let kind = self.name()?;
            let token = self.peek().clone();
            let type_name = self.name()?;
            match kind.as_str() {
                "node" => {
                    let space_id = if self.peek().kind == Kind::Word("space".into()) {
                        self.word("space")?;
                        Some(self.string()?)
                    } else {
                        None
                    };
                    let (properties, allow_extra_properties) = self.schema_properties()?;
                    if nodes
                        .insert(
                            type_name,
                            NodeSchema {
                                properties,
                                space_id,
                                allow_extra_properties,
                            },
                        )
                        .is_some()
                    {
                        return Err(Diagnostic::new(
                            "E_DUPLICATE",
                            "Duplicate node schema",
                            token.start,
                            token.end,
                        ));
                    }
                }
                "edge" => {
                    self.word("from")?;
                    let from_type = self.name()?;
                    self.word("to")?;
                    let to_type = self.name()?;
                    let allow_cross_space = if self.peek().kind == Kind::Word("cross_space".into())
                    {
                        self.word("cross_space")?;
                        true
                    } else {
                        false
                    };
                    let (properties, allow_extra_properties) = self.schema_properties()?;
                    if edges
                        .insert(
                            type_name,
                            EdgeSchema {
                                from_type,
                                to_type,
                                properties,
                                allow_cross_space,
                                allow_extra_properties,
                            },
                        )
                        .is_some()
                    {
                        return Err(Diagnostic::new(
                            "E_DUPLICATE",
                            "Duplicate edge schema",
                            token.start,
                            token.end,
                        ));
                    }
                }
                _ => return Err(self.error("Schema must declare node or edge types")),
            }
        }
        self.symbol('}')?;
        Ok(Statement::Schema {
            name: name.clone(),
            name_span,
            definition: GraphSchema {
                id: name,
                revision,
                nodes,
                edges,
            },
        })
    }
    fn metadata_host(&mut self) -> Result<MetadataHost, Diagnostic> {
        match self.name()?.as_str() {
            "node" => Ok(MetadataHost::Node { id: self.string()? }),
            "edge" => Ok(MetadataHost::Edge { id: self.string()? }),
            "assertion" => Ok(MetadataHost::Assertion { id: self.string()? }),
            "entity" => Ok(MetadataHost::Entity { id: self.string()? }),
            "graph" => Ok(MetadataHost::Graph),
            _ => Err(self.error("Expected node, edge, entity or graph metadata host")),
        }
    }
    fn rule_atom(&mut self) -> Result<(weave_contract::RuleAtom, [(usize, usize); 2]), Diagnostic> {
        let polarity = if self.peek().kind == Kind::Word("negative".into()) {
            self.take();
            weave_contract::Polarity::Negative
        } else {
            weave_contract::Polarity::Positive
        };
        let predicate = self.string()?;
        self.symbol('(')?;
        let term = |parser: &mut Self| -> Result<(weave_contract::RuleTerm, Span), Diagnostic> {
            let span = (parser.peek().start, parser.peek().end);
            let value = if matches!(parser.peek().kind, Kind::String(_)) {
                weave_contract::RuleTerm::Node {
                    id: parser.string()?,
                }
            } else {
                weave_contract::RuleTerm::Variable {
                    name: parser.name()?,
                }
            };
            Ok((value, span))
        };
        let (from, a) = term(self)?;
        self.symbol(',')?;
        let (to, b) = term(self)?;
        self.symbol(')')?;
        self.symbol(';')?;
        Ok((
            weave_contract::RuleAtom {
                predicate,
                from,
                to,
                polarity,
            },
            [a, b],
        ))
    }
    fn rules(&mut self, name: String, name_span: Span) -> Result<Statement, Diagnostic> {
        self.word("revision")?;
        let revision = self.string()?;
        self.symbol('{')?;
        let mut rules = Vec::new();
        let mut ids = std::collections::BTreeSet::new();
        while self.peek().kind != Kind::Symbol('}') {
            self.word("rule")?;
            let span = (self.peek().start, self.peek().end);
            let id = self.name()?;
            if !ids.insert(id.clone()) {
                return Err(Diagnostic::new(
                    "E_DUPLICATE",
                    "Duplicate rule ID",
                    span.0,
                    span.1,
                ));
            }
            self.symbol('{')?;
            let mut body = Vec::new();
            let mut head = None;
            let mut allow_cross_space = false;
            while self.peek().kind != Kind::Symbol('}') {
                match self.name()?.as_str() {
                    "when" => body.push(self.rule_atom()?.0),
                    "yield" => {
                        if head.is_some() {
                            return Err(self.error("Rule has more than one conclusion"));
                        }
                        head = Some(self.rule_atom()?);
                    }
                    "cross_space" => {
                        if allow_cross_space {
                            return Err(self.error("Duplicate cross_space declaration"));
                        }
                        allow_cross_space = true;
                        self.symbol(';')?;
                    }
                    _ => return Err(self.error("Rule expects when, yield or cross_space")),
                }
            }
            self.symbol('}')?;
            let (head, spans) = head.ok_or_else(|| self.error("Rule requires a conclusion"))?;
            let bound: std::collections::BTreeSet<_> = body
                .iter()
                .flat_map(|a| [&a.from, &a.to])
                .filter_map(|t| match t {
                    weave_contract::RuleTerm::Variable { name } => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            for (term, span) in [(&head.from, spans[0]), (&head.to, spans[1])] {
                if let weave_contract::RuleTerm::Variable { name } = term
                    && !bound.contains(name.as_str())
                {
                    return Err(Diagnostic::new(
                        "E_RULE_RANGE",
                        format!("Head variable '{name}' is not bound by evidence"),
                        span.0,
                        span.1,
                    ));
                }
            }
            rules.push(weave_contract::Rule {
                id,
                head,
                body,
                allow_cross_space,
            });
        }
        self.symbol('}')?;
        let definition = weave_contract::RuleSet {
            id: name.clone(),
            revision,
            rules,
        };
        if let Some(d) = weave_contract::validate_rule_set(&definition)
            .into_iter()
            .next()
        {
            return Err(Diagnostic::new(
                &d.code,
                d.message,
                name_span.0,
                name_span.1,
            ));
        }
        Ok(Statement::Rules {
            name,
            name_span,
            definition,
        })
    }
    fn program(&mut self, context: u8) -> Result<Program, Diagnostic> {
        let mut statements = Vec::new();
        while self.peek().kind != Kind::End
            && !(context != 0 && self.peek().kind == Kind::Symbol('}'))
        {
            if context == 2 && self.peek().kind == Kind::Word("return".into()) {
                break;
            }
            let kind = self.name()?;
            let name_span = (self.peek().start, self.peek().end);
            if kind == "module" {
                if context != 0 {
                    return Err(
                        self.error("Module headers are only allowed at source-unit top level")
                    );
                }
                let name = self.selector_string()?;
                self.word("revision")?;
                let revision = self.selector_string()?;
                self.symbol(';')?;
                statements.push(Statement::ModuleHeader {
                    name,
                    name_span,
                    revision,
                });
                continue;
            }
            let name = self.name()?;
            if name.contains("::") {
                return Err(Diagnostic::new(
                    "E_NAME",
                    "Declaration names cannot be qualified",
                    name_span.0,
                    name_span.1,
                ));
            }
            if context == 1 && !matches!(kind.as_str(), "graph" | "context_value") {
                return Err(
                    self.error("A local transaction contains only graph snapshot declarations")
                );
            }
            if context == 2
                && matches!(
                    kind.as_str(),
                    "function" | "transaction" | "graph" | "use" | "schema" | "rules"
                )
            {
                return Err(
                    self.error("Pure functions may only compose declared input graph values")
                );
            }
            if context == 2
                && matches!(
                    kind.as_str(),
                    "resolve_identity"
                        | "cluster_navigation"
                        | "typed_context"
                        | "context_schema"
                        | "context_value"
                        | "live_handle"
                        | "pin"
                        | "import"
                )
            {
                return Err(Diagnostic::new(
                    "E_FUNCTION_EFFECT",
                    "Pinned service reads must be bound outside pure functions and passed as graph arguments",
                    name_span.0,
                    name_span.1,
                ));
            }
            match kind.as_str() {
                "value" => {
                    let value = self.scalar_expr(0)?;
                    self.symbol(';')?;
                    statements.push(Statement::Value {
                        name,
                        name_span,
                        value,
                    });
                }
                "import" => {
                    self.word("module")?;
                    let module_id = self.selector_string()?;
                    self.word("revision")?;
                    let revision = self.selector_string()?;
                    self.word("sha256")?;
                    let digest_span = (self.peek().start, self.peek().end);
                    let digest = self.selector_string()?;
                    self.symbol(';')?;
                    statements.push(Statement::Import {
                        name,
                        name_span,
                        module_id,
                        revision,
                        digest,
                        digest_span,
                    });
                }
                "live_handle" => {
                    self.word("graph")?;
                    let graph = self.selector_string()?;
                    self.word("branch")?;
                    let branch = self.selector_string()?;
                    self.symbol(';')?;
                    statements.push(Statement::LiveHandle {
                        name,
                        name_span,
                        graph,
                        branch,
                    });
                }
                "pin" => {
                    self.word("from")?;
                    let source_span = (self.peek().start, self.peek().end);
                    let source = self.name()?;
                    let valid_at = if self.peek().kind == Kind::Word("at".into()) {
                        self.take();
                        Some(self.number()?)
                    } else {
                        None
                    };
                    let metadata_depth = if self.peek().kind == Kind::Word("metadata".into()) {
                        self.take();
                        self.word("depth")?;
                        let depth = self.number()?;
                        if !(0..=32).contains(&depth) {
                            return Err(self.error("Metadata depth must be between 0 and 32"));
                        }
                        Some(depth as u32)
                    } else {
                        None
                    };
                    self.symbol(';')?;
                    statements.push(Statement::Pin {
                        name,
                        name_span,
                        source,
                        source_span,
                        valid_at,
                        metadata_depth,
                    });
                }
                "context_schema" => {
                    self.word("revision")?;
                    let revision = self.selector_string()?;
                    self.symbol('{')?;
                    let mut axes = BTreeMap::new();
                    while self.peek().kind != Kind::Symbol('}') {
                        if axes.len() >= 32 {
                            return Err(self.error("Context schema permits at most 32 axes"));
                        }
                        self.word("axis")?;
                        let span = (self.peek().start, self.peek().end);
                        let axis = self.selector_string()?;
                        if axes.contains_key(&axis) {
                            return Err(Diagnostic::new(
                                "E_DUPLICATE",
                                "Duplicate context axis",
                                span.0,
                                span.1,
                            ));
                        }
                        let kind=match self.name()?.as_str() {
                            "boolean"=>weave_contract::context_axes::ContextAxisType::Boolean,
                            "integer"=>weave_contract::context_axes::ContextAxisType::Integer,
                            "string"=>weave_contract::context_axes::ContextAxisType::String,
                            "decimal"=>weave_contract::context_axes::ContextAxisType::Decimal,
                            "quantity"=>weave_contract::context_axes::ContextAxisType::Quantity{unit:self.unit_descriptor()?},
                            "enum"=>{
                                self.symbol('[')?;let mut members=Vec::new();
                                while self.peek().kind!=Kind::Symbol(']') {
                                    if members.len()>=128{return Err(self.error("Context enum permits at most 128 members"));}
                                    let token=self.peek().clone();let member=self.selector_string()?;
                                    if members.contains(&member){return Err(Diagnostic::new("E_DUPLICATE","Duplicate context enum member",token.start,token.end));}
                                    members.push(member);
                                    if self.peek().kind!=Kind::Symbol(']'){self.symbol(',')?;}
                                }
                                self.symbol(']')?;weave_contract::context_axes::ContextAxisType::Enum{members}
                            }
                            _=>return Err(self.error("Expected boolean, integer, string, decimal, quantity or enum axis type")),
                        };
                        self.symbol(';')?;
                        axes.insert(axis, kind);
                    }
                    self.symbol('}')?;
                    let definition = weave_contract::context_axes::ContextSchema {
                        reference: weave_contract::context_axes::ContextSchemaRef {
                            id: name.clone(),
                            revision,
                        },
                        axes,
                    };
                    definition.validate().map_err(|_| {
                        Diagnostic::new(
                            "E_CONTEXT_SCHEMA",
                            "Invalid bounded context schema",
                            name_span.0,
                            name_span.1,
                        )
                    })?;
                    statements.push(Statement::ContextSchema {
                        name,
                        name_span,
                        definition,
                    });
                }
                "context_value" => {
                    self.word("schema")?;
                    let schema_span = (self.peek().start, self.peek().end);
                    let schema = self.name()?;
                    self.word("source")?;
                    let attribution = self.selector_string()?;
                    self.symbol('{')?;
                    let mut axes = Vec::new();
                    let mut seen = std::collections::BTreeSet::new();
                    while self.peek().kind != Kind::Symbol('}') {
                        if axes.len() >= 32 {
                            return Err(self.error("Context value permits at most 32 axes"));
                        }
                        self.word("axis")?;
                        let name_span = (self.peek().start, self.peek().end);
                        let name = self.selector_string()?;
                        if !seen.insert(name.clone()) {
                            return Err(Diagnostic::new(
                                "E_DUPLICATE",
                                "Duplicate context assignment",
                                name_span.0,
                                name_span.1,
                            ));
                        }
                        let start = self.peek().start;
                        let value = self.literal(0)?;
                        let value_span = (start, self.tokens[self.cursor.saturating_sub(1)].end);
                        self.symbol(';')?;
                        axes.push(ContextAxisBinding {
                            name,
                            name_span,
                            value,
                            value_span,
                        });
                    }
                    self.symbol('}')?;
                    statements.push(Statement::ContextValue {
                        name,
                        name_span,
                        schema,
                        schema_span,
                        attribution,
                        axes,
                    });
                }
                "typed_context" => {
                    self.word("from")?;
                    let source_span = (self.peek().start, self.peek().end);
                    let source = self.name()?;
                    self.word("graph")?;
                    let graph_id = self.selector_string()?;
                    self.word("revision")?;
                    let revision = self.selector_string()?;
                    self.word("schema")?;
                    let schema_span = (self.peek().start, self.peek().end);
                    let schema = self.name()?;
                    self.symbol(';')?;
                    statements.push(Statement::TypedContext {
                        name,
                        name_span,
                        source,
                        source_span,
                        reference: weave_contract::GraphRef { graph_id, revision },
                        schema,
                        schema_span,
                    });
                }
                "resolve_identity" | "cluster_navigation" => {
                    self.word("source")?;
                    self.word("graph")?;
                    let graph_id = self.selector_string()?;
                    self.word("revision")?;
                    let revision = self.selector_string()?;
                    let service = if kind == "resolve_identity" {
                        self.word("node")?;
                        let node_id = self.selector_string()?;
                        self.word("mapping")?;
                        let mapping_id = self.selector_string()?;
                        self.word("revision")?;
                        let mapping_revision = self.selector_string()?;
                        self.word("policy")?;
                        let policy_id = self.selector_string()?;
                        self.word("revision")?;
                        let policy_revision = self.selector_string()?;
                        self.word("to")?;
                        self.word("space")?;
                        let target_space = self.selector_string()?;
                        self.word("at")?;
                        let valid_at = self.number()?;
                        let context = self.explicit_context()?;
                        NativeService::Identity {
                            selection: weave_contract::IdentityResolve {
                                mapping_id,
                                revision: mapping_revision,
                                policy: weave_contract::IdentityPolicyRef {
                                    id: policy_id,
                                    revision: policy_revision,
                                },
                                source: weave_contract::NodeRef {
                                    graph_id,
                                    revision,
                                    node_id,
                                },
                                target_space,
                                valid_at,
                                context,
                            },
                        }
                    } else {
                        self.word("relation")?;
                        let predicate = self.selector_string()?;
                        self.word("at")?;
                        let time = self.peek().clone();
                        let valid_at = self.number()?;
                        if valid_at == i64::MAX {
                            return Err(Diagnostic::new(
                                "E_CLUSTER_TIME",
                                "Sample time requires a representable exclusive end",
                                time.start,
                                time.end,
                            ));
                        }
                        self.word("levels")?;
                        let level = self.peek().clone();
                        let levels = self.number()?;
                        if !(0..=10000).contains(&levels) {
                            return Err(Diagnostic::new(
                                "E_CLUSTER_INPUT",
                                "Levels must be between 0 and 10000",
                                level.start,
                                level.end,
                            ));
                        }
                        let context = self.explicit_context()?;
                        NativeService::Cluster {
                            selection: weave_contract::ClusterRequest {
                                source: weave_contract::GraphRef { graph_id, revision },
                                context,
                                valid_at,
                                predicate,
                                levels: levels as usize,
                            },
                        }
                    };
                    self.symbol(';')?;
                    statements.push(Statement::NativeService {
                        name,
                        name_span,
                        service,
                    });
                }
                "rules" => {
                    statements.push(self.rules(name, name_span)?);
                }
                "reason" => {
                    self.word("from")?;
                    let source_span = (self.peek().start, self.peek().end);
                    let source = self.name()?;
                    self.word("using")?;
                    let rule_span = (self.peek().start, self.peek().end);
                    let rule_set = self.name()?;
                    self.symbol(';')?;
                    statements.push(Statement::Reason {
                        name,
                        name_span,
                        source,
                        source_span,
                        rule_set,
                        rule_span,
                    });
                }
                "function" => {
                    self.word("revision")?;
                    let revision = self.string()?;
                    self.symbol('(')?;
                    let mut parameters = Vec::new();
                    while self.peek().kind != Kind::Symbol(')') {
                        parameters.push(self.function_parameter(false)?);
                        if self.peek().kind != Kind::Symbol(')') {
                            self.symbol(',')?;
                        }
                    }
                    self.symbol(')')?;
                    let (output_schema, output_type) =
                        if self.peek().kind == Kind::Word("returns".into()) {
                            self.take();
                            self.function_result()?
                        } else {
                            (None, None)
                        };
                    self.symbol('{')?;
                    let body = self.program(2)?.statements;
                    self.word("return")?;
                    let output_span = (self.peek().start, self.peek().end);
                    let (output, scalar_return) = if let Some(value_type) = output_type {
                        (
                            String::new(),
                            Some(ScalarReturn {
                                value_type,
                                value: self.scalar_expr(0)?,
                            }),
                        )
                    } else {
                        (self.name()?, None)
                    };
                    self.symbol(';')?;
                    self.symbol('}')?;
                    statements.push(Statement::Function {
                        name,
                        name_span,
                        revision,
                        parameters,
                        output_schema,
                        scalar_return,
                        body,
                        output,
                        output_span,
                    });
                }
                "apply" => {
                    self.word("from")?;
                    let function_span = (self.peek().start, self.peek().end);
                    let function = self.name()?;
                    self.symbol('{')?;
                    let mut arguments = Vec::new();
                    while self.peek().kind != Kind::Symbol('}') {
                        let kind = self.name()?;
                        let span = (self.peek().start, self.peek().end);
                        let name = self.name()?;
                        let value_start = self.peek().start;
                        let value = match kind.as_str() {
                            "graph" => ArgumentValue::Graph(self.name()?),
                            "function" => ArgumentValue::Function(self.name()?),
                            "boolean" | "integer" | "decimal" | "quantity" => {
                                ArgumentValue::Scalar {
                                    kind: kind.clone(),
                                    value: self.scalar_expr(0)?,
                                }
                            }
                            "string" if matches!(&self.peek().kind,Kind::Word(v) if v!="param") => {
                                ArgumentValue::String(StringExpr::Value(self.scalar_expr(0)?))
                            }
                            "time" if matches!(&self.peek().kind,Kind::Word(v) if v!="param") => {
                                ArgumentValue::Time(TimeExpr::Value(self.scalar_expr(0)?))
                            }
                            "string" => ArgumentValue::String(
                                if self.peek().kind == Kind::Word("param".into()) {
                                    self.take();
                                    StringExpr::Parameter(self.name()?)
                                } else {
                                    StringExpr::Literal(self.string()?)
                                },
                            ),
                            "time" => ArgumentValue::Time(
                                if self.peek().kind == Kind::Word("param".into()) {
                                    self.take();
                                    TimeExpr::Parameter(self.name()?)
                                } else {
                                    TimeExpr::Literal(self.number()?)
                                },
                            ),
                            _ => return Err(self.error("Expected a typed function argument")),
                        };
                        let value_span =
                            (value_start, self.tokens[self.cursor.saturating_sub(1)].end);
                        self.symbol(';')?;
                        arguments.push(Argument {
                            name,
                            span,
                            value_span,
                            value,
                        });
                    }
                    self.symbol('}')?;
                    statements.push(Statement::Apply {
                        name,
                        name_span,
                        function,
                        function_span,
                        arguments,
                    });
                }
                "transaction" => {
                    if name.len() > 64 {
                        return Err(self.error("Transaction ID exceeds 64 bytes"));
                    }
                    self.symbol('{')?;
                    let body = self.program(1)?.statements;
                    self.symbol('}')?;
                    if body.is_empty() {
                        return Err(self.error("Transaction must contain at least one graph"));
                    }
                    statements.push(Statement::Transaction {
                        name,
                        name_span,
                        body,
                    });
                }
                "union" | "diff" | "project" | "support" | "context" | "explain" | "distance"
                | "transform" | "project_axes" | "counterparts" => {
                    self.word("from")?;
                    let source_span = (self.peek().start, self.peek().end);
                    let source = self.name()?;
                    let operation = match kind.as_str() {
                        "distance" | "transform" => {
                            self.word("assertion")?;
                            let left_assertion = self.string()?;
                            self.word(if kind == "distance" { "to" } else { "using" })?;
                            let right_span = (self.peek().start, self.peek().end);
                            let right = self.name()?;
                            self.word("assertion")?;
                            let right_assertion = self.string()?;
                            self.word("at")?;
                            let valid_at = self.number()?;
                            self.symbol(';')?;
                            if kind == "distance" {
                                AlgebraOperation::Distance {
                                    left_assertion,
                                    right,
                                    right_span,
                                    right_assertion,
                                    valid_at,
                                }
                            } else {
                                AlgebraOperation::Transform {
                                    left_assertion,
                                    right,
                                    right_span,
                                    right_assertion,
                                    valid_at,
                                }
                            }
                        }
                        "project_axes" => {
                            self.word("assertion")?;
                            let assertion_id = self.string()?;
                            self.word("axes")?;
                            self.symbol('(')?;
                            let mut axes = Vec::new();
                            for index in 0..3 {
                                if index > 0 {
                                    self.symbol(',')?;
                                }
                                let token = self.peek().clone();
                                let axis = self.number()?;
                                if !(0..4096).contains(&axis) {
                                    return Err(Diagnostic::new(
                                        "E_GEOMETRY_AXIS",
                                        "Axis index must be between 0 and 4095",
                                        token.start,
                                        token.end,
                                    ));
                                }
                                if axes.contains(&(axis as usize)) {
                                    return Err(Diagnostic::new(
                                        "E_GEOMETRY_AXIS",
                                        "Projection axes must be distinct",
                                        token.start,
                                        token.end,
                                    ));
                                }
                                axes.push(axis as usize);
                            }
                            self.symbol(')')?;
                            self.word("revision")?;
                            let revision_span = (self.peek().start, self.peek().end);
                            let projection_revision = self.string()?;
                            if projection_revision.len() > 1024 {
                                return Err(Diagnostic::new(
                                    "E_GEOMETRY_REVISION",
                                    "Projection revision exceeds 1024 bytes",
                                    revision_span.0,
                                    revision_span.1,
                                ));
                            }
                            self.word("at")?;
                            let valid_at = self.number()?;
                            self.symbol(';')?;
                            AlgebraOperation::ProjectAxes {
                                assertion_id,
                                axes,
                                projection_revision,
                                valid_at,
                            }
                        }
                        "counterparts" => {
                            self.word("relation")?;
                            let predicate = self.string()?;
                            self.word("entity")?;
                            let entity_id = self.string()?;
                            self.word("from")?;
                            self.word("space")?;
                            let from_space_id = self.string()?;
                            self.word("to")?;
                            self.word("space")?;
                            let to_space_id = self.string()?;
                            self.word("at")?;
                            let valid_at = self.number()?;
                            self.symbol(';')?;
                            let selection = weave_contract::CounterpartSelection {
                                predicate,
                                entity_id,
                                from_space_id,
                                to_space_id,
                                valid_at,
                            };
                            if let Err(d) =
                                weave_contract::counterpart::validate_selection(&selection)
                            {
                                return Err(Diagnostic::new(
                                    &d.code,
                                    d.message,
                                    name_span.0,
                                    name_span.1,
                                ));
                            }
                            AlgebraOperation::Counterparts { selection }
                        }
                        "explain" => {
                            self.symbol(';')?;
                            AlgebraOperation::Explain
                        }
                        "context" => {
                            let selection = if self.peek().kind == Kind::Word("default".into()) {
                                self.take();
                                weave_contract::ContextSelection::Default
                            } else {
                                self.word("graph")?;
                                let graph_id = self.string()?;
                                self.word("revision")?;
                                let revision = self.string()?;
                                weave_contract::ContextSelection::Pinned {
                                    reference: weave_contract::GraphRef { graph_id, revision },
                                }
                            };
                            if let Err(d) = weave_contract::context::validate_selection(&selection)
                            {
                                return Err(Diagnostic::new(
                                    &d.code,
                                    d.message,
                                    name_span.0,
                                    name_span.1,
                                ));
                            }
                            self.symbol(';')?;
                            AlgebraOperation::Context { selection }
                        }
                        "union" | "diff" => {
                            self.word(if kind == "union" { "with" } else { "to" })?;
                            let right_span = (self.peek().start, self.peek().end);
                            let right = self.name()?;
                            self.symbol(';')?;
                            if kind == "union" {
                                AlgebraOperation::Union { right, right_span }
                            } else {
                                AlgebraOperation::Diff { right, right_span }
                            }
                        }
                        "project" => {
                            self.symbol('{')?;
                            let mut node_ids = Vec::new();
                            let mut edge_ids = Vec::new();
                            while self.peek().kind != Kind::Symbol('}') {
                                let member = self.name()?;
                                let id = self.string()?;
                                match member.as_str() {
                                    "node" => node_ids.push(id),
                                    "edge" => edge_ids.push(id),
                                    _ => return Err(self.error("Projection expects node or edge")),
                                };
                                self.symbol(';')?;
                            }
                            self.symbol('}')?;
                            AlgebraOperation::Project { node_ids, edge_ids }
                        }
                        "support" => {
                            self.word("relation")?;
                            let predicate = self.string()?;
                            self.word("from")?;
                            self.word("entity")?;
                            let entity_id = self.string()?;
                            self.word("space")?;
                            let space_id = self.string()?;
                            let from = EntitySpace {
                                entity_id,
                                space_id,
                            };
                            self.word("to")?;
                            self.word("entity")?;
                            let entity_id = self.string()?;
                            self.word("space")?;
                            let space_id = self.string()?;
                            let to = EntitySpace {
                                entity_id,
                                space_id,
                            };
                            self.word("at")?;
                            let valid_at = self.number()?;
                            self.symbol(';')?;
                            AlgebraOperation::Support {
                                predicate,
                                from,
                                to,
                                valid_at,
                            }
                        }
                        _ => unreachable!(),
                    };
                    statements.push(Statement::Algebra {
                        name,
                        name_span,
                        source,
                        source_span,
                        operation,
                    });
                }
                "metadata" => {
                    self.word("from")?;
                    let source_span = (self.peek().start, self.peek().end);
                    let source = self.name()?;
                    self.word("on")?;
                    let host = self.metadata_host()?;
                    self.word("key")?;
                    let key = self.string()?;
                    self.symbol(';')?;
                    statements.push(Statement::Metadata {
                        name,
                        name_span,
                        source,
                        source_span,
                        host,
                        key,
                    });
                }
                "schema" => {
                    statements.push(self.schema(name, name_span)?);
                }
                "graph" => {
                    let profile = if self.peek().kind == Kind::Word("explicit".into()) {
                        self.take();
                        GraphProfile::Explicit
                    } else {
                        GraphProfile::Legacy
                    };
                    let mut schema_span = None;
                    let schema = if self.peek().kind == Kind::Word("schema".into()) {
                        self.word("schema")?;
                        schema_span = Some((self.peek().start, self.peek().end));
                        Some(self.name()?)
                    } else {
                        None
                    };
                    let branch = if self.peek().kind == Kind::Word("branch".into()) {
                        self.take();
                        self.selector_string()?
                    } else {
                        "main".into()
                    };
                    let expected_head = if self.peek().kind == Kind::Word("replace".into()) {
                        self.take();
                        self.word("revision")?;
                        Some(self.selector_string()?)
                    } else {
                        None
                    };
                    self.symbol('{')?;
                    let mut items = Vec::new();
                    while self.peek().kind != Kind::Symbol('}') {
                        let item = self.name()?;
                        let id_span = (self.peek().start, self.peek().end);
                        let id = self.string()?;
                        let type_id = if self.peek().kind == Kind::Word("type".into()) {
                            self.word("type")?;
                            Some(self.name()?)
                        } else {
                            None
                        };
                        let value = match item.as_str() {
                            "relation" => {
                                self.word("from")?;
                                let from = self.string()?;
                                self.word("to")?;
                                let to = self.string()?;
                                self.word("predicate")?;
                                let predicate = self.string()?;
                                let (metadata, properties) = self.metadata()?;
                                Item::Structural {
                                    id,
                                    id_span,
                                    type_id,
                                    from,
                                    to,
                                    predicate,
                                    metadata,
                                    properties,
                                }
                            }
                            "claim" => {
                                if type_id.is_some() {
                                    return Err(
                                        self.error("Claim type belongs to its structural relation")
                                    );
                                }
                                self.word("on")?;
                                let edge_id = self.string()?;
                                self.word("source")?;
                                let source = self.string()?;
                                let context = if self.peek().kind == Kind::Word("context".into()) {
                                    self.take();
                                    self.word("graph")?;
                                    let graph_id = self.string()?;
                                    self.word("revision")?;
                                    let revision = self.string()?;
                                    Some(weave_contract::GraphRef { graph_id, revision })
                                } else {
                                    None
                                };
                                self.word("polarity")?;
                                let negative = match self.name()?.as_str() {
                                    "positive" => false,
                                    "negative" => true,
                                    _ => {
                                        return Err(
                                            self.error("Expected positive or negative polarity")
                                        );
                                    }
                                };
                                self.word("valid")?;
                                let valid_from = self.number()?;
                                self.word("until")?;
                                let valid_to = if self.peek().kind == Kind::Word("infinity".into())
                                {
                                    self.take();
                                    None
                                } else {
                                    Some(self.number()?)
                                };
                                let (metadata, properties) = self.metadata()?;
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
                                }
                            }
                            "attachment" => {
                                if type_id.is_some() {
                                    return Err(
                                        self.error("Attachments cannot carry an object type")
                                    );
                                }
                                self.word("on")?;
                                let host = self.metadata_host()?;
                                self.word("key")?;
                                let key = self.string()?;
                                let mut literal = None;
                                let value = if self.peek().kind == Kind::Word("literal".into()) {
                                    self.take();
                                    literal = Some(self.template_literal(0)?);
                                    MetadataValue::Literal {
                                        value: serde_json::Value::Null,
                                    }
                                } else {
                                    let live = self.peek().kind == Kind::Word("live".into());
                                    if live {
                                        self.take();
                                    }
                                    self.word("graph")?;
                                    let graph_id = self.selector_string()?;
                                    if live {
                                        self.word("branch")?;
                                        let branch_id = self.selector_string()?;
                                        MetadataValue::LiveGraph {
                                            graph_id,
                                            branch_id,
                                        }
                                    } else {
                                        self.word("revision")?;
                                        let revision = self.selector_string()?;
                                        MetadataValue::Graph {
                                            reference: weave_contract::GraphRef {
                                                graph_id,
                                                revision,
                                            },
                                        }
                                    }
                                };
                                self.word("valid")?;
                                let valid_from = self.number()?;
                                self.word("until")?;
                                let valid_to = if self.peek().kind == Kind::Word("infinity".into())
                                {
                                    self.take();
                                    None
                                } else {
                                    Some(self.number()?)
                                };
                                let required = if self.peek().kind == Kind::Word("required".into())
                                {
                                    self.take();
                                    true
                                } else {
                                    false
                                };
                                let context = if self.peek().kind == Kind::Word("context".into()) {
                                    self.take();
                                    self.word("graph")?;
                                    let graph_id = self.string()?;
                                    self.word("revision")?;
                                    let revision = self.string()?;
                                    Some(weave_contract::GraphRef { graph_id, revision })
                                } else {
                                    None
                                };
                                Item::Attachment {
                                    context,
                                    id,
                                    id_span,
                                    host,
                                    key,
                                    value,
                                    literal,
                                    valid_from,
                                    valid_to,
                                    required,
                                }
                            }
                            "node" => {
                                self.word("entity")?;
                                let entity = self.string()?;
                                self.word("space")?;
                                let space = self.string()?;
                                let (metadata, properties) = self.metadata()?;
                                Item::Node {
                                    id,
                                    id_span,
                                    type_id,
                                    entity,
                                    space,
                                    metadata,
                                    properties,
                                }
                            }
                            "edge" => {
                                self.word("from")?;
                                let from = self.string()?;
                                self.word("to")?;
                                let to = self.string()?;
                                self.word("relation")?;
                                let predicate = self.string()?;
                                let negative = if self.peek().kind == Kind::Word("polarity".into())
                                {
                                    self.word("polarity")?;
                                    match self.name()?.as_str() {
                                        "negative" => true,
                                        "positive" => false,
                                        _ => {
                                            return Err(
                                                self.error("Polarity must be positive or negative")
                                            );
                                        }
                                    }
                                } else {
                                    false
                                };
                                self.word("valid")?;
                                let valid_from = self.number()?;
                                self.word("until")?;
                                let valid_to = if self.peek().kind == Kind::Word("infinity".into())
                                {
                                    self.take();
                                    None
                                } else {
                                    Some(self.number()?)
                                };
                                let (metadata, properties) = self.metadata()?;
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
                                }
                            }
                            _ => return Err(self.error("Expected node or edge declaration")),
                        };
                        self.symbol(';')?;
                        items.push(value);
                    }
                    self.symbol('}')?;
                    statements.push(Statement::Graph {
                        branch,
                        expected_head,
                        profile,
                        schema,
                        schema_span,
                        name,
                        name_span,
                        items,
                    });
                }
                "use" => {
                    self.word("graph")?;
                    let graph = self.string()?;
                    let revision = if self.peek().kind == Kind::Word("revision".into()) {
                        self.word("revision")?;
                        Some(self.string()?)
                    } else {
                        None
                    };
                    self.symbol(';')?;
                    statements.push(Statement::Use {
                        name,
                        name_span,
                        graph,
                        revision,
                    });
                }
                "join" => {
                    self.word("from")?;
                    let left_span = (self.peek().start, self.peek().end);
                    let left = self.name()?;
                    self.word("to")?;
                    let right_span = (self.peek().start, self.peek().end);
                    let right = self.name()?;
                    self.word("relation")?;
                    let predicate = self.string()?;
                    self.symbol(';')?;
                    statements.push(Statement::Join {
                        name,
                        name_span,
                        left,
                        left_span,
                        right,
                        right_span,
                        predicate,
                    });
                }
                "bind" => {
                    self.word("from")?;
                    let source_span = (self.peek().start, self.peek().end);
                    let source = self.name()?;
                    self.symbol('{')?;
                    let mut bindings = Vec::new();
                    while self.peek().kind != Kind::Symbol('}') {
                        let kind = self.name()?;
                        let span = (self.peek().start, self.peek().end);
                        let name = self.name()?;
                        let value = match kind.as_str() {
                            "string" => BindingValue::String(self.string()?),
                            "time" => BindingValue::Time(self.number()?),
                            _ => return Err(self.error("Binding must declare string or time")),
                        };
                        self.symbol(';')?;
                        bindings.push(Binding { name, span, value });
                    }
                    self.symbol('}')?;
                    statements.push(Statement::Bind {
                        name,
                        name_span,
                        source,
                        source_span,
                        bindings,
                    });
                }
                "lens" => {
                    self.word("from")?;
                    let source_span = (self.peek().start, self.peek().end);
                    let source = self.name()?;
                    self.symbol('{')?;
                    let mut predicate = None;
                    let mut valid_at = None;
                    let mut include_metadata = false;
                    let mut max_depth = 8;
                    while self.peek().kind != Kind::Symbol('}') {
                        match self.name()?.as_str() {
                            "match" => {
                                if predicate.is_some() {
                                    return Err(self.error("Duplicate match filter"));
                                }
                                self.word("relation")?;
                                predicate =
                                    Some(if self.peek().kind == Kind::Word("value".into()) {
                                        StringExpr::Value(self.scalar_expr(0)?)
                                    } else if self.peek().kind == Kind::Word("param".into()) {
                                        self.word("param")?;
                                        StringExpr::Parameter(self.name()?)
                                    } else {
                                        StringExpr::Literal(self.string()?)
                                    });
                            }
                            "metadata" => {
                                if include_metadata {
                                    return Err(self.error("Duplicate metadata clause"));
                                }
                                self.word("depth")?;
                                let depth = self.number()?;
                                if !(0..=32).contains(&depth) {
                                    return Err(
                                        self.error("Metadata depth must be between 0 and 32")
                                    );
                                }
                                include_metadata = true;
                                max_depth = depth as u32;
                            }
                            "at" => {
                                if valid_at.is_some() {
                                    return Err(self.error("Duplicate valid-time filter"));
                                }
                                valid_at =
                                    Some(if self.peek().kind == Kind::Word("value".into()) {
                                        TimeExpr::Value(self.scalar_expr(0)?)
                                    } else if self.peek().kind == Kind::Word("param".into()) {
                                        self.word("param")?;
                                        TimeExpr::Parameter(self.name()?)
                                    } else {
                                        TimeExpr::Literal(self.number()?)
                                    });
                            }
                            _ => {
                                return Err(self.error(
                                    "Lens allows only pure 'match relation' and 'at' filters",
                                ));
                            }
                        };
                        self.symbol(';')?;
                    }
                    self.symbol('}')?;
                    statements.push(Statement::Lens {
                        name,
                        name_span,
                        source,
                        source_span,
                        predicate,
                        valid_at,
                        include_metadata,
                        max_depth,
                    });
                }
                _ => return Err(self.error("Expected graph or lens declaration")),
            }
        }
        Ok(Program { statements })
    }
}
pub fn parse(source: &str) -> Result<Program, Diagnostic> {
    Parser {
        tokens: lex(source)?,
        cursor: 0,
    }
    .program(0)
}

/// Visit only typed source expressions; never interpret user JSON object keys.
pub(crate) fn scalar_expressions(statement: &mut Statement, f: &mut impl FnMut(&mut ScalarExpr)) {
    fn literal(v: &mut LiteralExpr, f: &mut impl FnMut(&mut ScalarExpr)) {
        match v {
            LiteralExpr::Scalar(e) => f(e),
            LiteralExpr::Array(v) => {
                for x in v {
                    literal(x, f)
                }
            }
            LiteralExpr::Object(v) => {
                for x in v.values_mut() {
                    literal(x, f)
                }
            }
            _ => (),
        }
    }
    match statement {
        Statement::Value { value, .. } => f(value),
        Statement::Function {
            scalar_return,
            body,
            ..
        } => {
            if let Some(r) = scalar_return {
                f(&mut r.value);
            }
            for s in body {
                scalar_expressions(s, f)
            }
        }
        Statement::Transaction { body, .. } => {
            for s in body {
                scalar_expressions(s, f)
            }
        }
        Statement::Apply { arguments, .. } => {
            for a in arguments {
                match &mut a.value {
                    ArgumentValue::Scalar { value, .. }
                    | ArgumentValue::String(StringExpr::Value(value))
                    | ArgumentValue::Time(TimeExpr::Value(value)) => f(value),
                    _ => (),
                }
            }
        }
        Statement::Lens {
            predicate,
            valid_at,
            ..
        } => {
            if let Some(StringExpr::Value(v)) = predicate {
                f(v)
            }
            if let Some(TimeExpr::Value(v)) = valid_at {
                f(v)
            }
        }
        Statement::Graph { items, .. } => {
            for i in items {
                match i {
                    Item::Node { properties, .. }
                    | Item::Edge { properties, .. }
                    | Item::Structural { properties, .. }
                    | Item::Claim { properties, .. } => {
                        for v in properties.values_mut() {
                            literal(v, f)
                        }
                    }
                    Item::Attachment {
                        literal: Some(v), ..
                    } => literal(v, f),
                    _ => (),
                }
            }
        }
        _ => (),
    }
}
pub(crate) fn callback_constraints(
    statement: &mut Statement,
    f: &mut impl FnMut(&mut SchemaConstraint),
) {
    if let Statement::Function { parameters, .. } = statement {
        for p in parameters {
            if let Some(c) = &mut p.callback {
                if let Some(s) = &mut c.input.schema {
                    f(s)
                }
                if let Some(s) = &mut c.output_schema {
                    f(s)
                }
            }
        }
    }
}
