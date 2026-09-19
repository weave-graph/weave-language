use serde::{Deserialize, Serialize};

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
    Node {
        id: String,
        id_span: Span,
        entity: String,
        space: String,
        metadata: Vec<Metadata>,
    },
    Edge {
        id: String,
        id_span: Span,
        from: String,
        to: String,
        predicate: String,
        valid_from: i64,
        valid_to: Option<i64>,
        metadata: Vec<Metadata>,
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
        } else if "{};".contains(c) {
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
    fn metadata(&mut self) -> Result<Vec<Metadata>, Diagnostic> {
        let mut result = Vec::new();
        while self.peek().kind == Kind::Word("metadata".into()) {
            self.word("metadata")?;
            self.word("graph")?;
            let graph = self.string()?;
            self.word("revision")?;
            let revision = self.string()?;
            result.push(Metadata { graph, revision });
        }
        Ok(result)
    }
    fn program(&mut self) -> Result<Program, Diagnostic> {
        let mut statements = Vec::new();
        while self.peek().kind != Kind::End {
            let kind = self.name()?;
            let name_span = (self.peek().start, self.peek().end);
            let name = self.name()?;
            match kind.as_str() {
                "graph" => {
                    self.symbol('{')?;
                    let mut items = Vec::new();
                    while self.peek().kind != Kind::Symbol('}') {
                        let item = self.name()?;
                        let id_span = (self.peek().start, self.peek().end);
                        let id = self.string()?;
                        let value = match item.as_str() {
                            "node" => {
                                self.word("entity")?;
                                let entity = self.string()?;
                                self.word("space")?;
                                let space = self.string()?;
                                let metadata = self.metadata()?;
                                Item::Node {
                                    id,
                                    id_span,
                                    entity,
                                    space,
                                    metadata,
                                }
                            }
                            "edge" => {
                                self.word("from")?;
                                let from = self.string()?;
                                self.word("to")?;
                                let to = self.string()?;
                                self.word("relation")?;
                                let predicate = self.string()?;
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
                                let metadata = self.metadata()?;
                                Item::Edge {
                                    id,
                                    id_span,
                                    from,
                                    to,
                                    predicate,
                                    valid_from,
                                    valid_to,
                                    metadata,
                                }
                            }
                            _ => return Err(self.error("Expected node or edge declaration")),
                        };
                        self.symbol(';')?;
                        items.push(value);
                    }
                    self.symbol('}')?;
                    statements.push(Statement::Graph {
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
    .program()
}
