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
}
impl Diagnostic {
    pub(crate) fn new(code: &str, message: impl Into<String>, start: usize, end: usize) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            start,
            end,
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
    Function {
        name: String,
        name_span: Span,
        revision: String,
        parameters: Vec<FunctionParameter>,
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
        profile: GraphProfile,
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
pub struct FunctionParameter {
    pub name: String,
    pub span: Span,
    pub kind: ParameterKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterKind {
    Graph,
    String,
    Time,
    Function,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Argument {
    pub name: String,
    pub span: Span,
    pub value: ArgumentValue,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ArgumentValue {
    Graph(String),
    Function(String),
    String(StringExpr),
    Time(TimeExpr),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operator", rename_all = "snake_case")]
pub enum AlgebraOperation {
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
    Literal(String),
    Parameter(String),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TimeExpr {
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
        properties: BTreeMap<String, serde_json::Value>,
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
        properties: BTreeMap<String, serde_json::Value>,
    },
    Attachment {
        id: String,
        id_span: Span,
        host: MetadataHost,
        key: String,
        value: MetadataValue,
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
        properties: BTreeMap<String, serde_json::Value>,
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
        properties: BTreeMap<String, serde_json::Value>,
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
            Kind::Number(source[start..i].parse().map_err(|_| {
                Diagnostic::new("E_NUMBER", "Expected a signed 64-bit integer", start, i)
            })?)
        } else if c.is_ascii_alphabetic() || c == '_' {
            i += 1;
            while i < source.len()
                && (source.as_bytes()[i].is_ascii_alphanumeric() || source.as_bytes()[i] == b'_')
            {
                i += 1;
            }
            Kind::Word(source[start..i].into())
        } else if "{};(),".contains(c) {
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
    fn metadata(
        &mut self,
    ) -> Result<(Vec<Metadata>, BTreeMap<String, serde_json::Value>), Diagnostic> {
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
                let token = self.take();
                let value = match token.kind {
                    Kind::String(v) => serde_json::Value::String(v),
                    Kind::Number(v) => serde_json::json!(v),
                    Kind::Word(v) if v == "true" => serde_json::Value::Bool(true),
                    Kind::Word(v) if v == "false" => serde_json::Value::Bool(false),
                    Kind::Word(v) if v == "null" => serde_json::Value::Null,
                    _ => {
                        return Err(Diagnostic::new(
                            "E_PROPERTY_TYPE",
                            "Property value must be string, integer, boolean or null",
                            token.start,
                            token.end,
                        ));
                    }
                };
                if properties.insert(key, value).is_some() {
                    return Err(Diagnostic::new(
                        "E_DUPLICATE",
                        "Duplicate scalar property",
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
                _ => {
                    return Err(self.error("Schema scalar type must be string, integer or boolean"));
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
            let name = self.name()?;
            if context == 1 && kind != "graph" {
                return Err(
                    self.error("A local transaction contains only graph snapshot declarations")
                );
            }
            if context == 2
                && matches!(
                    kind.as_str(),
                    "function" | "transaction" | "graph" | "use" | "schema"
                )
            {
                return Err(
                    self.error("Pure functions may only compose declared input graph values")
                );
            }
            match kind.as_str() {
                "function" => {
                    self.word("revision")?;
                    let revision = self.string()?;
                    self.symbol('(')?;
                    let mut parameters = Vec::new();
                    while self.peek().kind != Kind::Symbol(')') {
                        let kind = match self.name()?.as_str() {
                            "graph" => ParameterKind::Graph,
                            "string" => ParameterKind::String,
                            "time" => ParameterKind::Time,
                            "function" => ParameterKind::Function,
                            _ => {
                                return Err(self
                                    .error("Expected graph, string, time or function parameter"));
                            }
                        };
                        let span = (self.peek().start, self.peek().end);
                        let name = self.name()?;
                        parameters.push(FunctionParameter { name, span, kind });
                        if self.peek().kind != Kind::Symbol(')') {
                            self.symbol(',')?;
                        }
                    }
                    self.symbol(')')?;
                    self.symbol('{')?;
                    let body = self.program(2)?.statements;
                    self.word("return")?;
                    let output_span = (self.peek().start, self.peek().end);
                    let output = self.name()?;
                    self.symbol(';')?;
                    self.symbol('}')?;
                    statements.push(Statement::Function {
                        name,
                        name_span,
                        revision,
                        parameters,
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
                        let value = match kind.as_str() {
                            "graph" => ArgumentValue::Graph(self.name()?),
                            "function" => ArgumentValue::Function(self.name()?),
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
                        self.symbol(';')?;
                        arguments.push(Argument { name, span, value });
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
                "union" | "diff" | "project" | "support" => {
                    self.word("from")?;
                    let source_span = (self.peek().start, self.peek().end);
                    let source = self.name()?;
                    let operation = match kind.as_str() {
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
                    let schema = if self.peek().kind == Kind::Word("schema".into()) {
                        self.word("schema")?;
                        Some(self.name()?)
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
                                self.word("graph")?;
                                let graph_id = self.string()?;
                                self.word("revision")?;
                                let revision = self.string()?;
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
                                Item::Attachment {
                                    id,
                                    id_span,
                                    host,
                                    key,
                                    value: MetadataValue::Graph {
                                        reference: weave_contract::GraphRef { graph_id, revision },
                                    },
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
                        profile,
                        schema,
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
                                    Some(if self.peek().kind == Kind::Word("param".into()) {
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
                                    Some(if self.peek().kind == Kind::Word("param".into()) {
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
