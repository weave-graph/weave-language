//! Explicit content-pinned linking. This core accepts caller-supplied bytes and performs no I/O.
mod rewrite;
use crate::syntax::{self, Diagnostic, Span, Statement};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use weave_contract::{GraphSchema, Program, SourceRevision};

pub const MAX_UNITS: usize = 64;
pub const MAX_UNIT_BYTES: usize = 1_048_576;
pub const MAX_TOTAL_BYTES: usize = 4 * MAX_UNIT_BYTES;
const MAX_AST_BYTES: usize = 16 * MAX_UNIT_BYTES;
const MAX_DECLARATIONS: usize = 10_000;
const MAX_EDGES: usize = 256;
const MAX_DEPTH: usize = 16;
const PREFIX: &str = "__weave_module_";
#[derive(Clone, Copy)]
pub struct SourceModule<'a> {
    pub id: &'a str,
    pub revision: &'a str,
    pub source: &'a str,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ImportTrace {
    pub source_id: String,
    pub start: usize,
    pub end: usize,
    pub module_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModuleDiagnostic {
    pub source_id: String,
    pub code: String,
    pub message: String,
    pub start: usize,
    pub end: usize,
    pub import_trace: Vec<ImportTrace>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[allow(clippy::box_collection)]
    // Keep the public error result small; traces are exceptional.
    pub application_trace: Box<Vec<ApplicationLocation>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ApplicationLocation {
    pub source_id: String,
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct Import {
    pub alias: String,
    pub module_id: String,
    pub revision: String,
    pub digest: String,
    pub span: Span,
    pub digest_span: Span,
}
#[derive(Clone)]
struct Location {
    id: String,
    base: usize,
    length: usize,
    trace: Vec<ImportTrace>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Schema,
    ContextSchema,
    Function,
    Rules,
}
impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::ContextSchema => "context",
            Self::Function => "function",
            Self::Rules => "rules",
        }
    }
}
#[derive(Clone)]
struct Export {
    kind: Kind,
    symbol: String,
    identity: String,
}
struct Unit {
    id: String,
    revision: String,
    digest: String,
    ast: syntax::Program,
    imports: Vec<Import>,
    exports: BTreeMap<String, Export>,
    location: Location,
    height: usize,
}
/// Original source ranges are retained privately for diagnostics; AST spans use virtual offsets.
pub struct LinkedProgram {
    ast: syntax::Program,
    locations: Vec<Location>,
    manifests: Vec<SourceRevision>,
    identities: BTreeMap<String, String>,
}
fn issue(code: &str, message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(code, message, span.0, span.1)
}
fn diagnostic(location: &Location, error: Diagnostic) -> ModuleDiagnostic {
    ModuleDiagnostic {
        source_id: location.id.clone(),
        code: error.code,
        message: error.message,
        start: error
            .start
            .saturating_sub(location.base)
            .min(location.length),
        end: error.end.saturating_sub(location.base).min(location.length),
        import_trace: location.trace.clone(),
        application_trace: Box::default(),
    }
}
fn valid_label(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
}
fn valid_module_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}
fn valid_symbol(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && !s.starts_with(PREFIX)
        && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
/// Raw UTF-8 digest; line endings and comments are deliberately part of the content pin.
pub fn content_digest(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}
fn declaration(s: &Statement) -> (&str, Span) {
    crate::functions::declaration(s)
}
fn kind(s: &Statement) -> Option<Kind> {
    match s {
        Statement::Schema { .. } => Some(Kind::Schema),
        Statement::ContextSchema { .. } => Some(Kind::ContextSchema),
        Statement::Function { .. } => Some(Kind::Function),
        Statement::Rules { .. } => Some(Kind::Rules),
        _ => None,
    }
}
fn inspect(ast: &syntax::Program, module: Option<(&str, &str)>) -> Result<Vec<Import>, Diagnostic> {
    let mut imports = Vec::new();
    let mut aliases = BTreeSet::new();
    let mut members = BTreeSet::new();
    let mut declarations = false;
    for (index, s) in ast.statements.iter().enumerate() {
        let (name, span) = declaration(s);
        match s {
            Statement::ModuleHeader { name, revision, .. } => {
                if index != 0 || module != Some((name.as_str(), revision.as_str())) {
                    return Err(issue(
                        "E_MODULE_HEADER",
                        "Imported unit requires one matching first module header",
                        span,
                    ));
                }
            }
            Statement::Import {
                name,
                module_id,
                revision,
                digest,
                digest_span,
                ..
            } => {
                if declarations {
                    return Err(issue(
                        "E_MODULE_ORDER",
                        "Imports must precede declarations",
                        span,
                    ));
                }
                if !valid_symbol(name) || !valid_module_id(module_id) || !valid_label(revision) {
                    return Err(issue(
                        "E_MODULE_ID",
                        "Bounded module, revision and alias identifiers required",
                        span,
                    ));
                }
                if digest.len() != 64
                    || !digest
                        .bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                {
                    return Err(issue(
                        "E_MODULE_DIGEST",
                        "Expected 64 lowercase SHA-256 hex digits",
                        *digest_span,
                    ));
                }
                if !aliases.insert(name.clone()) {
                    return Err(issue("E_DUPLICATE", "Duplicate import alias", span));
                }
                imports.push(Import {
                    alias: name.clone(),
                    module_id: module_id.clone(),
                    revision: revision.clone(),
                    digest: digest.clone(),
                    span,
                    digest_span: *digest_span,
                });
            }
            _ => {
                declarations = true;
                if name.starts_with(PREFIX) || name.contains("::") || aliases.contains(name) {
                    return Err(issue(
                        "E_MODULE_NAME",
                        "Declaration conflicts with a reserved namespace or import alias",
                        span,
                    ));
                }
                if module.is_some() && (kind(s).is_none() || !valid_symbol(name)) {
                    return Err(issue(
                        "E_MODULE_EFFECT",
                        "Imported units permit only bounded pure function, schema, context-schema and finite-rule declarations",
                        span,
                    ));
                }
                if module.is_some() && !members.insert(name.to_owned()) {
                    return Err(issue("E_DUPLICATE", "Duplicate exported declaration", span));
                }
            }
        }
    }
    if module.is_some() && !matches!(ast.statements.first(), Some(Statement::ModuleHeader { .. })) {
        return Err(issue(
            "E_MODULE_HEADER",
            "Imported unit requires a matching module header",
            (0, 0),
        ));
    }
    Ok(imports)
}
/// Inspect direct imports only; useful to an explicit host loader. This does not resolve files.
pub fn imports(source: &str, module: Option<(&str, &str)>) -> Result<Vec<Import>, Diagnostic> {
    inspect(&crate::parse(source)?, module)
}
struct Counter(usize);
impl std::io::Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        if self.0 > MAX_AST_BYTES {
            return Err(std::io::Error::other("linked AST budget"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn charge<T: Serialize>(value: &T, bytes: &mut usize) -> Result<(), Diagnostic> {
    let mut c = Counter(*bytes);
    serde_json::to_writer(&mut c, value)
        .map_err(|_| issue("E_MODULE_BUDGET", "Linked AST exceeds 16 MiB", (0, 0)))?;
    *bytes = c.0;
    Ok(())
}
fn count_declarations(statement: &Statement, count: &mut usize) -> Result<(), Diagnostic> {
    *count += 1;
    if *count > MAX_DECLARATIONS {
        return Err(issue(
            "E_MODULE_BUDGET",
            "At most 10000 linked declarations including function bodies",
            declaration(statement).1,
        ));
    }
    if let Statement::Function { body, .. } | Statement::Transaction { body, .. } = statement {
        for child in body {
            count_declarations(child, count)?;
        }
    }
    Ok(())
}
impl LinkedProgram {
    pub fn ast(&self) -> &syntax::Program {
        &self.ast
    }
    pub fn schemas(&self) -> Vec<GraphSchema> {
        self.ast
            .statements
            .iter()
            .filter_map(|s| {
                if let Statement::Schema { definition, .. } = s {
                    Some(definition.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    fn locate(&self, error: Diagnostic) -> ModuleDiagnostic {
        let location = self
            .locations
            .iter()
            .find(|l| error.start >= l.base && error.start <= l.base + l.length)
            .unwrap_or(&self.locations[0]);
        let trace = error
            .trace
            .iter()
            .map(|&(start, end)| {
                let l = self
                    .locations
                    .iter()
                    .find(|l| start >= l.base && start <= l.base + l.length)
                    .unwrap_or(&self.locations[0]);
                ApplicationLocation {
                    source_id: l.id.clone(),
                    start: start.saturating_sub(l.base).min(l.length),
                    end: end.saturating_sub(l.base).min(l.length),
                }
            })
            .collect();
        let mut result = diagnostic(location, error);
        result.application_trace = Box::new(trace);
        result
    }
    pub fn compile(&self) -> Result<Program, ModuleDiagnostic> {
        Ok(self.specialize()?.program)
    }
    pub fn specialize(&self) -> Result<crate::SpecializedProgram, ModuleDiagnostic> {
        let mut output = crate::specialize_parsed(self.ast.clone()).map_err(|e| self.locate(e))?;
        let program = &mut output.program;
        for revision in &mut program.source_revisions {
            if let Some(name) = self.identities.get(&revision.name) {
                revision.name = name.clone();
            }
        }
        program.source_revisions.extend(self.manifests.clone());
        let mut seen = BTreeMap::new();
        for r in &program.source_revisions {
            if seen
                .insert((&r.name, &r.revision), &r.digest)
                .is_some_and(|old| old != &r.digest)
            {
                return Err(self.locate(issue(
                    "E_SOURCE_REVISION",
                    "Conflicting linked source identities",
                    (0, 0),
                )));
            }
        }
        Ok(output)
    }
    pub fn fingerprint(&self) -> Result<String, ModuleDiagnostic> {
        weave_contract::identity::program_fingerprint(&self.compile()?, &self.schemas())
            .map_err(|e| self.locate(issue(&e.code, e.message, (0, 0))))
    }
}
/// Deterministically link explicit supplied units. No implicit resolution or execution occurs.
pub fn link(
    entry_id: &str,
    source: &str,
    supplied: &[SourceModule<'_>],
) -> Result<LinkedProgram, ModuleDiagnostic> {
    let entry = Location {
        id: entry_id.into(),
        base: 0,
        length: source.len(),
        trace: vec![],
    };
    let fail = |e| diagnostic(&entry, e);
    if entry_id.is_empty()
        || entry_id.len() > 512
        || source.len() > MAX_UNIT_BYTES
        || supplied.len() > MAX_UNITS
    {
        return Err(fail(issue(
            "E_MODULE_BUDGET",
            "Bounded entry and at most 64 supplied units required",
            (0, 0),
        )));
    }
    let mut bytes = source.len();
    let mut registry = BTreeMap::new();
    for module in supplied {
        bytes = bytes.saturating_add(module.source.len());
        if !valid_module_id(module.id)
            || !valid_label(module.revision)
            || module.source.len() > MAX_UNIT_BYTES
            || bytes > MAX_TOTAL_BYTES
        {
            return Err(fail(issue(
                "E_MODULE_BUDGET",
                "Module identifiers or aggregate source exceed bounds",
                (0, 0),
            )));
        }
        if registry.insert(module.id, *module).is_some() {
            return Err(fail(issue(
                "E_MODULE_REVISION",
                "Only one supplied revision and byte sequence per module ID is permitted",
                (0, 0),
            )));
        }
    }
    let ast = crate::parse(source).map_err(fail)?;
    let direct = inspect(&ast, None).map_err(fail)?;
    let mut loader = Loader {
        registry,
        units: BTreeMap::new(),
        visiting: BTreeSet::new(),
        order: vec![],
        edges: 0,
        ast_bytes: 0,
    };
    charge(&ast, &mut loader.ast_bytes).map_err(fail)?;
    let mut sorted = direct.clone();
    sorted.sort_by(|a, b| (&a.module_id, &a.alias).cmp(&(&b.module_id, &b.alias)));
    for import in &sorted {
        loader.visit(import, &entry, 0)?;
    }
    let mut locations = vec![entry.clone()];
    let mut base = source.len() + 1;
    for unit in loader.units.values_mut() {
        unit.location.base = base;
        base += unit.location.length + 1;
        locations.push(unit.location.clone());
    }
    let exports: BTreeMap<_, _> = loader
        .units
        .iter()
        .map(|(id, u)| (id.clone(), u.exports.clone()))
        .collect();
    let mut linked = Vec::new();
    let mut manifests = Vec::new();
    let mut identities = BTreeMap::new();
    let mut retained = 0;
    let mut declarations = 0;
    for id in &loader.order {
        let unit = loader.units.get_mut(id).unwrap();
        let resolver =
            rewrite::Resolver::new(&unit.id, &unit.imports, &exports, Some(&unit.exports));
        for mut statement in std::mem::take(&mut unit.ast.statements) {
            if matches!(
                statement,
                Statement::Import { .. } | Statement::ModuleHeader { .. }
            ) {
                continue;
            }
            resolver
                .statement(&mut statement, true, &BTreeSet::new())
                .map_err(|e| {
                    diagnostic(
                        &Location {
                            base: 0,
                            ..unit.location.clone()
                        },
                        e,
                    )
                })?;
            rewrite::shift(&mut statement, unit.location.base);
            charge(&statement, &mut retained).map_err(|e| diagnostic(&unit.location, e))?;
            count_declarations(&statement, &mut declarations)
                .map_err(|e| diagnostic(&unit.location, e))?;
            linked.push(statement);
        }
        for export in unit.exports.values() {
            if matches!(export.kind, Kind::Function | Kind::Rules) {
                identities.insert(export.symbol.clone(), export.identity.clone());
            }
        }
        manifests.push(SourceRevision {
            name: format!("module:{}", unit.id),
            revision: unit.revision.clone(),
            digest: format!("sha256:{}", unit.digest),
        });
    }
    let resolver = rewrite::Resolver::new("", &direct, &exports, None);
    for mut statement in ast.statements {
        if matches!(statement, Statement::Import { .. }) {
            continue;
        }
        resolver
            .statement(&mut statement, true, &BTreeSet::new())
            .map_err(fail)?;
        charge(&statement, &mut retained).map_err(fail)?;
        count_declarations(&statement, &mut declarations).map_err(fail)?;
        linked.push(statement);
    }
    if linked.len() > MAX_DECLARATIONS {
        return Err(fail(issue(
            "E_MODULE_BUDGET",
            "At most 10000 linked declarations",
            (0, 0),
        )));
    }
    Ok(LinkedProgram {
        ast: syntax::Program { statements: linked },
        locations,
        manifests,
        identities,
    })
}
struct Loader<'a> {
    registry: BTreeMap<&'a str, SourceModule<'a>>,
    units: BTreeMap<String, Unit>,
    visiting: BTreeSet<String>,
    order: Vec<String>,
    edges: usize,
    ast_bytes: usize,
}
impl Loader<'_> {
    fn visit(
        &mut self,
        import: &Import,
        parent: &Location,
        depth: usize,
    ) -> Result<(), ModuleDiagnostic> {
        let fail = |e| {
            diagnostic(
                &Location {
                    base: 0,
                    ..parent.clone()
                },
                e,
            )
        };
        self.edges += 1;
        if depth >= MAX_DEPTH || self.edges > MAX_EDGES {
            return Err(fail(issue(
                "E_MODULE_BUDGET",
                "Import depth or edge count exceeded",
                import.span,
            )));
        }
        let module = *self
            .registry
            .get(import.module_id.as_str())
            .ok_or_else(|| {
                fail(issue(
                    "E_MODULE_MISSING",
                    "Pinned module bytes were not supplied",
                    import.span,
                ))
            })?;
        if module.revision != import.revision {
            return Err(fail(issue(
                "E_MODULE_REVISION",
                "Imported module revision differs from supplied unit",
                import.span,
            )));
        }
        if self.visiting.contains(module.id) {
            return Err(fail(issue(
                "E_MODULE_CYCLE",
                "Import dependency cycle rejected",
                import.span,
            )));
        }
        let digest = content_digest(module.source);
        if digest != import.digest {
            return Err(fail(issue(
                "E_MODULE_DIGEST",
                "Imported source bytes do not match the explicit SHA-256 pin",
                import.digest_span,
            )));
        }
        if let Some(unit) = self.units.get(module.id) {
            if depth + unit.height > MAX_DEPTH {
                return Err(fail(issue(
                    "E_MODULE_BUDGET",
                    "Longest import path exceeds depth 16",
                    import.span,
                )));
            }
            return Ok(());
        }
        let mut trace = parent.trace.clone();
        trace.push(ImportTrace {
            source_id: parent.id.clone(),
            start: import.span.0,
            end: import.span.1,
            module_id: module.id.into(),
        });
        let location = Location {
            id: format!("module:{}@{}", module.id, module.revision),
            base: 0,
            length: module.source.len(),
            trace,
        };
        let ast = crate::parse(module.source).map_err(|e| diagnostic(&location, e))?;
        charge(&ast, &mut self.ast_bytes).map_err(|e| diagnostic(&location, e))?;
        let imports = inspect(&ast, Some((module.id, module.revision)))
            .map_err(|e| diagnostic(&location, e))?;
        let mut exports = BTreeMap::new();
        for statement in &ast.statements {
            if let Some(kind) = kind(statement) {
                let (name, _) = declaration(statement);
                exports.insert(
                    name.into(),
                    Export {
                        kind,
                        symbol: format!("{PREFIX}{}_{}", content_digest(module.id), name),
                        identity: format!("module:{}:{}:{name}", module.id, kind.label()),
                    },
                );
            }
        }
        self.visiting.insert(module.id.into());
        let mut sorted = imports.clone();
        sorted.sort_by(|a, b| (&a.module_id, &a.alias).cmp(&(&b.module_id, &b.alias)));
        for dependency in &sorted {
            self.visit(dependency, &location, depth + 1)?;
        }
        let height = 1 + imports
            .iter()
            .map(|i| self.units[&i.module_id].height)
            .max()
            .unwrap_or(0);
        self.visiting.remove(module.id);
        self.order.push(module.id.into());
        self.units.insert(
            module.id.into(),
            Unit {
                id: module.id.into(),
                revision: module.revision.into(),
                digest,
                ast,
                imports,
                exports,
                location,
                height,
            },
        );
        Ok(())
    }
}
pub fn compile_with_modules(
    entry_id: &str,
    source: &str,
    modules: &[SourceModule<'_>],
) -> Result<Program, ModuleDiagnostic> {
    link(entry_id, source, modules)?.compile()
}
