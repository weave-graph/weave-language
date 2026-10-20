//! Agent-safe compilation and permission-aware diagnostics (LB1).
//!
//! Three things live here, all pure:
//!
//! - [`plan_effects`] lists every graph a compiled plan reads or writes, so a
//!   host can show, compare or gate a plan before running it;
//! - [`CompilePolicy`] adds compile-time rules for source the host does not
//!   trust (an agent's or a user's text): `untrusted` forbids implicitly public
//!   records and host templates, and `read_only` forbids every write;
//! - [`VisibleManifest`] is what one principal can see (graphs with their
//!   permission, schemas and embedding spaces, and the groups it may name in
//!   `reader_groups`). Checked against it, a source gets the engine's own error
//!   codes at compile time instead of at run time.
//!
//! None of this is a security boundary. The engine validates and authorizes
//! every plan; a policy only rejects more, never less, and a manifest only
//! adds diagnostics. A manifest is supplied by the host (engine B3/B4 serve
//! it; until then a host assembles it from `weave-engine graphs` and query
//! results), and the compiler never reads a store.
use crate::syntax::{self, Diagnostic, Item, Statement, Visibility, WriteFact, WriteKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use weave_contract::embedding::EmbeddingSpace;
use weave_contract::{GraphSchema, Program};

/// The `format` of a [`VisibleManifest`].
pub const MANIFEST_FORMAT: &str = "weave-visible/1";
/// Most graphs one manifest lists.
pub const MAX_MANIFEST_GRAPHS: usize = 10_000;
/// Most groups one manifest lists (the engine's transitive group bound).
pub const MAX_MANIFEST_GROUPS: usize = 1_024;
/// Most embedding spaces one manifest graph lists.
pub const MAX_MANIFEST_SPACES: usize = 1_024;
/// Largest encoded manifest.
pub const MAX_MANIFEST_BYTES: usize = 8 * 1024 * 1024;

/// A graph permission, ordered as in the engine registry:
/// Discover ≤ Read ≤ Write ≤ Administer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Discover,
    Read,
    Write,
    Administer,
}

/// One graph the principal can discover.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleGraph {
    pub graph_id: String,
    pub permission: Permission,
    #[serde(default)]
    pub branches: Vec<String>,
    /// The graph's schema as the principal reads it, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<GraphSchema>,
    /// Every embedding space the principal can see in the graph, when known.
    /// Absent means unknown, so spaces are not checked; `[]` means none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_spaces: Option<Vec<EmbeddingSpace>>,
}

/// What one principal can see and name (format `weave-visible/1`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleManifest {
    pub format: String,
    pub principal: String,
    pub graphs: Vec<VisibleGraph>,
    /// The groups this principal may name in `reader_groups`.
    #[serde(default)]
    pub reader_groups: Vec<String>,
}

fn manifest_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new("E_MANIFEST", message, 0, 0)
}

impl VisibleManifest {
    /// Decode and validate a manifest from strict JSON bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(Diagnostic::new(
                "E_BUDGET",
                format!("A visible manifest is at most {MAX_MANIFEST_BYTES} bytes"),
                0,
                0,
            ));
        }
        let manifest: Self = serde_json::from_slice(bytes)
            .map_err(|e| manifest_error(format!("Invalid {MANIFEST_FORMAT} manifest: {e}")))?;
        manifest.validate()?;
        Ok(manifest)
    }
    /// Check the format, bounds and uniqueness of a manifest.
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.format != MANIFEST_FORMAT {
            return Err(manifest_error(format!(
                "Manifest format must be '{MANIFEST_FORMAT}'"
            )));
        }
        if self.principal.is_empty() {
            return Err(manifest_error("Manifest principal must be nonempty"));
        }
        if self.graphs.len() > MAX_MANIFEST_GRAPHS || self.reader_groups.len() > MAX_MANIFEST_GROUPS
        {
            return Err(Diagnostic::new(
                "E_BUDGET",
                format!(
                    "A manifest lists at most {MAX_MANIFEST_GRAPHS} graphs and {MAX_MANIFEST_GROUPS} groups"
                ),
                0,
                0,
            ));
        }
        let mut graphs = BTreeSet::new();
        for graph in &self.graphs {
            if graph.graph_id.is_empty() || !graphs.insert(graph.graph_id.as_str()) {
                return Err(manifest_error(format!(
                    "Manifest graph IDs must be nonempty and unique: '{}'",
                    graph.graph_id
                )));
            }
            if let Some(spaces) = &graph.embedding_spaces {
                if spaces.len() > MAX_MANIFEST_SPACES {
                    return Err(Diagnostic::new(
                        "E_BUDGET",
                        format!("A manifest graph lists at most {MAX_MANIFEST_SPACES} spaces"),
                        0,
                        0,
                    ));
                }
                for space in spaces {
                    space
                        .validate()
                        .map_err(|e| manifest_error(format!("Manifest space: {}", e.message)))?;
                }
            }
        }
        let mut groups = BTreeSet::new();
        if self
            .reader_groups
            .iter()
            .any(|g| g.is_empty() || !groups.insert(g))
        {
            return Err(manifest_error(
                "Manifest reader groups must be nonempty and unique",
            ));
        }
        Ok(())
    }
    fn graph(&self, graph_id: &str) -> Option<&VisibleGraph> {
        self.graphs.iter().find(|g| g.graph_id == graph_id)
    }
}

/// Compile-time rules for source the host does not trust (LB1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompilePolicy {
    /// Every written record must state its readers (`readers`,
    /// `reader_groups` or `public`), and host view and handler templates are
    /// rejected.
    pub untrusted: bool,
    /// The plan may not write: no `graph`, `transaction` or `write`
    /// declarations, and no host templates.
    pub read_only: bool,
    /// Check graph, group and space references against what this principal
    /// can see.
    pub visible: Option<VisibleManifest>,
}

impl CompilePolicy {
    /// Whether the policy adds any rule.
    pub fn is_default(&self) -> bool {
        !self.untrusted && !self.read_only && self.visible.is_none()
    }
}

/// How a plan touches a graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    /// A query, traversal or other evaluation of the graph.
    Read,
    /// A pinned `{graph_id, revision}` reference held in data (metadata,
    /// attachments, derivation pins). The engine treats an unreadable one as an
    /// unavailable revision, not as an error.
    Reference,
    /// A snapshot commit or fact write.
    Write,
}

/// One graph a plan command reads, references or writes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlanEffect {
    /// Index of the command in the plan.
    pub command: usize,
    /// The command's `op`.
    pub op: String,
    pub access: Access,
    pub graph_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
}

/// The ops that write a graph.
pub const WRITE_OPS: [&str; 3] = ["commit", "commit_batch", "write"];

/// Every graph each command of a plan reads, references or writes, in command
/// order. Only commands present in the plan appear; the list is derived from
/// the plan bytes, not from source.
pub fn plan_effects(program: &Program) -> Vec<PlanEffect> {
    let mut effects = BTreeSet::new();
    for (index, command) in program.commands.iter().enumerate() {
        let value = serde_json::to_value(command).unwrap_or_default();
        let op = value["op"].as_str().unwrap_or_default().to_string();
        let mut add = |access, graph_id: &str, branch_id: Option<&str>| {
            // An empty ID is a compiler placeholder (a handler's seeded event
            // input), never a stored graph. Every other ID is listed.
            if !graph_id.is_empty() {
                effects.insert(PlanEffect {
                    command: index,
                    op: op.clone(),
                    access,
                    graph_id: graph_id.into(),
                    branch_id: branch_id.map(Into::into),
                });
            }
        };
        let mut targets: Vec<(&serde_json::Value, &serde_json::Value)> = Vec::new();
        match op.as_str() {
            "commit" | "write" => targets.push((&value["graph_id"], &value["branch_id"])),
            "commit_batch" => {
                for commit in value["commits"].as_array().into_iter().flatten() {
                    targets.push((&commit["graph_id"], &commit["branch_id"]));
                }
            }
            _ => {}
        }
        for (graph, branch) in &targets {
            if let Some(graph) = graph.as_str() {
                add(Access::Write, graph, branch.as_str());
            }
        }
        // Every other graph the command names. Inside written data (metadata,
        // attachments, derivation pins) a graph is referenced, not read.
        let nested = if targets.is_empty() {
            Access::Read
        } else {
            Access::Reference
        };
        let mut stack: Vec<(&serde_json::Value, bool)> = vec![(&value, true)];
        while let Some((node, top)) = stack.pop() {
            match node {
                serde_json::Value::Object(map) => {
                    let target = !targets.is_empty()
                        && (top || (op == "commit_batch" && map.contains_key("data")));
                    if let (Some(graph), false) =
                        (map.get("graph_id").and_then(|g| g.as_str()), target)
                    {
                        add(nested, graph, map.get("branch_id").and_then(|b| b.as_str()));
                    }
                    stack.extend(map.values().map(|v| (v, false)));
                }
                serde_json::Value::Array(items) => stack.extend(items.iter().map(|v| (v, false))),
                _ => {}
            }
        }
    }
    effects.into_iter().collect()
}

fn effect_denied(what: &str, span: syntax::Span) -> Diagnostic {
    Diagnostic::new(
        "E_EFFECT_DENIED",
        format!("The read-only policy forbids {what}"),
        span.0,
        span.1,
    )
}

fn visibility_required(span: syntax::Span) -> Diagnostic {
    Diagnostic::new(
        "E_VISIBILITY_REQUIRED",
        "The untrusted policy requires every written record to state its readers: add 'readers', 'reader_groups' or 'public'",
        span.0,
        span.1,
    )
}

/// The first exact JSON string literal of `id` in the source, for a span.
fn literal_span(source: &str, id: &str) -> syntax::Span {
    let quoted = serde_json::to_string(id).unwrap_or_default();
    source
        .find(&quoted)
        .map(|start| (start, start + quoted.len()))
        .unwrap_or((0, 0))
}

struct Checker<'a> {
    policy: &'a CompilePolicy,
    spaces: BTreeMap<String, EmbeddingSpace>,
    /// Program-local graph names (`use` and `graph`) to stored graph IDs.
    graphs: BTreeMap<String, String>,
}

impl Checker<'_> {
    fn need(
        &self,
        graph_id: &str,
        needed: Permission,
        span: syntax::Span,
    ) -> Result<(), Diagnostic> {
        let Some(manifest) = &self.policy.visible else {
            return Ok(());
        };
        need(manifest, graph_id, needed, span)
    }
    fn visibility(&self, visibility: &Visibility) -> Result<(), Diagnostic> {
        let Some(manifest) = &self.policy.visible else {
            return Ok(());
        };
        for (group, span) in &visibility.reader_groups {
            if !manifest.reader_groups.iter().any(|g| g == group) {
                return Err(Diagnostic::new(
                    "E_UNAVAILABLE",
                    format!(
                        "Principal '{}' cannot name group '{group}' in reader_groups: it does not exist, is not active, or is not a group this principal may name",
                        manifest.principal
                    ),
                    span.0,
                    span.1,
                ));
            }
        }
        Ok(())
    }
    /// A space used against a stored graph must be one the principal sees there.
    fn space(&self, graph_id: &str, name: &str, span: syntax::Span) -> Result<(), Diagnostic> {
        let Some(manifest) = &self.policy.visible else {
            return Ok(());
        };
        let (Some(declared), Some(graph)) = (self.spaces.get(name), manifest.graph(graph_id))
        else {
            return Ok(());
        };
        let Some(visible) = &graph.embedding_spaces else {
            return Ok(());
        };
        match visible
            .iter()
            .find(|s| s.id == declared.id && s.revision == declared.revision)
        {
            Some(stored) if stored == declared => Ok(()),
            Some(stored) => Err(Diagnostic::new(
                "E_SPACE_MISMATCH",
                format!(
                    "Embedding space '{}@{}' differs from the one in graph '{graph_id}': stored encoder '{}' version '{}', preprocessing '{}', {} dimensions, {:?}",
                    declared.id,
                    declared.revision,
                    stored.encoder,
                    stored.encoder_version,
                    stored.preprocessing,
                    stored.dimensions,
                    stored.metric
                ),
                span.0,
                span.1,
            )),
            None => Err(Diagnostic::new(
                "E_UNKNOWN_SPACE",
                format!(
                    "Graph '{graph_id}' has no embedding space '{}@{}' visible to principal '{}'",
                    declared.id, declared.revision, manifest.principal
                ),
                span.0,
                span.1,
            )),
        }
    }
    fn record(&self, visibility: &Visibility, span: syntax::Span) -> Result<(), Diagnostic> {
        if self.policy.untrusted && visibility.is_unstated() {
            return Err(visibility_required(span));
        }
        if let (true, Some(widen)) = (self.policy.untrusted, visibility.widen) {
            // The engine's own code for a widening correction: untrusted
            // source may restate or narrow readers, never publish wider.
            return Err(Diagnostic::new(
                "E_READERS_WIDENED",
                "The untrusted policy forbids 'widen_readers': a correction from untrusted source may not make a fact readable by more principals",
                widen.0,
                widen.1,
            ));
        }
        self.visibility(visibility)
    }
    fn statements(&mut self, statements: &[Statement]) -> Result<(), Diagnostic> {
        for statement in statements {
            self.statement(statement)?;
        }
        Ok(())
    }
    fn statement(&mut self, statement: &Statement) -> Result<(), Diagnostic> {
        match statement {
            Statement::EmbeddingSpace {
                name, definition, ..
            } => {
                self.spaces.insert(name.clone(), definition.clone());
            }
            Statement::ViewTemplate { name_span, .. }
            | Statement::HandlerTemplate { name_span, .. } => {
                if self.policy.read_only {
                    return Err(effect_denied("host view and handler templates", *name_span));
                }
                if self.policy.untrusted {
                    return Err(Diagnostic::new(
                        "E_UNTRUSTED_ARTIFACT",
                        "The untrusted policy forbids host view and handler templates; only the trusted host installs them",
                        name_span.0,
                        name_span.1,
                    ));
                }
            }
            Statement::Transaction {
                name_span, body, ..
            } => {
                if self.policy.read_only {
                    return Err(effect_denied(
                        "a transaction (snapshot commits)",
                        *name_span,
                    ));
                }
                self.statements(body)?;
            }
            Statement::Use {
                name,
                name_span,
                graph,
                ..
            } => {
                self.need(graph, Permission::Read, *name_span)?;
                self.graphs.insert(name.clone(), graph.clone());
            }
            Statement::Graph {
                name,
                name_span,
                items,
                ..
            } => {
                if self.policy.read_only {
                    return Err(effect_denied(
                        &format!("a snapshot commit of graph '{name}'"),
                        *name_span,
                    ));
                }
                self.need(name, Permission::Write, *name_span)?;
                for item in items {
                    let (visibility, span) = match item {
                        Item::Structural {
                            visibility,
                            id_span,
                            ..
                        }
                        | Item::Claim {
                            visibility,
                            id_span,
                            ..
                        }
                        | Item::Attachment {
                            visibility,
                            id_span,
                            ..
                        }
                        | Item::Node {
                            visibility,
                            id_span,
                            ..
                        }
                        | Item::Edge {
                            visibility,
                            id_span,
                            ..
                        } => (visibility, *id_span),
                    };
                    self.record(visibility, span)?;
                }
                self.graphs.insert(name.clone(), name.clone());
            }
            Statement::ContextValue {
                name, name_span, ..
            } => {
                if self.policy.read_only {
                    return Err(effect_denied(
                        &format!("a context value commit of graph '{name}'"),
                        *name_span,
                    ));
                }
                if self.policy.untrusted {
                    return Err(Diagnostic::new(
                        "E_VISIBILITY_REQUIRED",
                        "The untrusted policy forbids context_value: it commits a public context graph whose readers source cannot state",
                        name_span.0,
                        name_span.1,
                    ));
                }
                self.need(name, Permission::Write, *name_span)?;
                self.graphs.insert(name.clone(), name.clone());
            }
            Statement::Write {
                name_span,
                graph,
                changes,
                ..
            } => {
                if self.policy.read_only {
                    return Err(effect_denied(
                        &format!("a fact write to graph '{graph}'"),
                        *name_span,
                    ));
                }
                self.need(graph, Permission::Write, *name_span)?;
                for change in changes {
                    if change.change == WriteKind::Retract {
                        continue;
                    }
                    match &change.fact {
                        WriteFact::Node { visibility, .. } | WriteFact::Edge { visibility, .. } => {
                            self.record(visibility, change.span)?;
                        }
                        WriteFact::Embedding {
                            visibility,
                            space,
                            space_span,
                            ..
                        } => {
                            self.record(visibility, change.span)?;
                            self.space(graph, space, *space_span)?;
                        }
                        WriteFact::Space { space, space_span } => {
                            self.space(graph, space, *space_span)?;
                        }
                        WriteFact::Target { .. } => {}
                    }
                }
            }
            Statement::Traversal {
                source, operation, ..
            } => {
                if let Some(graph) = self.graphs.get(source).cloned() {
                    match operation {
                        syntax::TraversalOperation::Nearest {
                            space, space_span, ..
                        }
                        | syntax::TraversalOperation::SimilarCandidates {
                            space, space_span, ..
                        } => self.space(&graph, space, *space_span)?,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn need(
    manifest: &VisibleManifest,
    graph_id: &str,
    needed: Permission,
    span: syntax::Span,
) -> Result<(), Diagnostic> {
    match manifest.graph(graph_id) {
        // The same message whether the graph is missing or hidden, as in the engine.
        None => Err(Diagnostic::new(
            "E_UNAVAILABLE",
            format!(
                "Graph '{graph_id}' is not visible to principal '{}': it does not exist or this principal cannot discover it",
                manifest.principal
            ),
            span.0,
            span.1,
        )),
        Some(graph) if graph.permission < needed => Err(Diagnostic::new(
            "E_FORBIDDEN",
            format!(
                "Principal '{}' holds {:?} on graph '{graph_id}', but this needs {:?}",
                manifest.principal, graph.permission, needed
            ),
            span.0,
            span.1,
        )),
        Some(_) => Ok(()),
    }
}

/// Check a compiled entry source against a policy. `parsed` is the entry
/// source's AST and `artifacts` what it compiled to. Source-level rules run
/// first, with exact spans; then every effect of the plan itself is checked,
/// so constructs without a dedicated rule are still covered.
pub fn check(
    source: &str,
    parsed: &syntax::Program,
    artifacts: &crate::CompiledArtifacts,
    policy: &CompilePolicy,
) -> Result<(), Diagnostic> {
    if policy.is_default() {
        return Ok(());
    }
    if let Some(manifest) = &policy.visible {
        manifest.validate()?;
    }
    let mut checker = Checker {
        policy,
        spaces: BTreeMap::new(),
        graphs: BTreeMap::new(),
    };
    checker.statements(&parsed.statements)?;
    if policy.read_only
        && (!artifacts.view_templates.is_empty() || !artifacts.handler_templates.is_empty())
    {
        return Err(effect_denied("host view and handler templates", (0, 0)));
    }
    if policy.untrusted
        && (!artifacts.view_templates.is_empty() || !artifacts.handler_templates.is_empty())
    {
        return Err(Diagnostic::new(
            "E_UNTRUSTED_ARTIFACT",
            "The untrusted policy forbids host view and handler templates",
            0,
            0,
        ));
    }
    for effect in plan_effects(&artifacts.program) {
        let span = literal_span(source, &effect.graph_id);
        match effect.access {
            Access::Write if policy.read_only => {
                return Err(effect_denied(
                    &format!("'{}' of graph '{}'", effect.op, effect.graph_id),
                    span,
                ));
            }
            Access::Write => {
                if let Some(manifest) = &policy.visible {
                    need(manifest, &effect.graph_id, Permission::Write, span)?;
                }
            }
            Access::Read => {
                if let Some(manifest) = &policy.visible {
                    need(manifest, &effect.graph_id, Permission::Read, span)?;
                }
            }
            Access::Reference => {}
        }
    }
    Ok(())
}
