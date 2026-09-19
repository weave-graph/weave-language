# Deterministic pinned source modules: approved bounded design

This source-only stage implements part of L01–L03 using protocol 0.14. The paper specifies reusable pure functions, schemas, finite rules, explicit effect boundaries and source identities (§§4–5, 10–11); it does not prescribe import grammar. The syntax and limits here are implementation choices, not quotations from the paper.

```weave
// tools.weave: an imported pure unit
module "equipment.tools" revision "1";
schema Fleet revision "1" { node Device space "operations" {} }
function Keep revision "1" (graph input schema Fleet)
  returns graph schema Fleet { return input; }
```

```weave
// main.weave: caller with an explicit content pin
import tools module "equipment.tools" revision "1" sha256 "<64 lowercase hex digits>";
graph G schema tools::Fleet {}
apply Result from tools::Keep { graph input G; }
```

Imports form an acyclic dependency graph. Every imported unit has a matching module header and raw UTF-8 SHA-256 content digest; new unit bytes are checked before parsing. A back-edge to an active module ID is rejected before checking that edge's digest; mutually raw-hash-pinned authored cycles would otherwise require a hash fixed point. One module ID has one revision and byte sequence in a linked bundle. Different revisions of one ID, duplicate supplied module IDs, mismatched pins, cycles and missing units fail explicitly. Diamonds load/link a shared exact module once. Import aliases must be unique in their unit. Imports precede declarations and may name only direct exported members via `alias::Name`; nested namespace traversal and reexports are deferred.

All module-level schema, context-schema, finite rule and pure function declarations are exported in this first profile. Export names are unique across kinds. Imported modules cannot declare graph snapshots, runtime reads, live handles, pins, top-level applications, transactions or other effects. Function bodies retain existing purity checks. An import is neither module installation into a runtime nor permission to execute external effects. Source text found in arbitrary graph properties remains data.

## Library and CLI boundary

Implemented pure API:

```rust
struct SourceModule<'a> { id: &'a str, revision: &'a str, source: &'a str }
fn compile_with_modules(entry_id: &str, source: &str, modules: &[SourceModule<'_>])
    -> Result<weave_contract::Program, ModuleDiagnostic>;
```

The caller supplies all source units. The library does not inspect files, resolve URLs, fetch a registry, read environment variables or request host authority. Existing `compile(source)` remains unchanged for programs without imports; an unresolved import reports that supplied modules are required. A separate linked AST/schema-discovery result may support CLI `ast`/`describe` without execution.

The CLI optionally accepts an explicit local module map with entries `{id, revision, path}`. Paths are relative to the map directory, with no absolute paths, parent traversal or URL interpretation; canonicalized symlink targets must remain inside that directory and be regular files. These checks define a local loader policy, not a filesystem sandbox. The map is bounded and checked for unknown/duplicate fields and duplicate module identities. Source pins remain in imports, so changing a mapped file cannot silently change compiled meaning.

## Names, schema identities and source manifests

Linking renames AST identifiers, never raw text or JSON literals. Internal symbols are deterministic from module ID and declaration name/kind, independent of local alias, import order, file path and source offsets. Module ID/revision, aliases and member names are bounded before qualification. Root declaration/effect order is preserved; dependency definitions are linked in deterministic dependency order. Existing within-unit declaration ordering and nonrecursive function rules remain.

Semantic schema/rule/function identities are qualified by module ID and declaration kind/name, while authored schema/rule/function revisions remain explicit. Renaming an import alias does not rename its nominal schema. Property keys/values, relation strings, type-looking string literals, entity/space IDs and scalar metadata are not rewritten. Same nominal schema ID/revision with different descriptors fails; no schema erasure or structural cast is introduced.

Each dependency module contributes its exact raw digest and header revision to `Program.source_revisions`, including explicitly imported but unused definitions. Individual function/rule revisions remain present with canonical qualified names and normalized semantic fingerprints. Code/module labels and hashes are reproducibility metadata, not authenticated authorship or runtime authority. Equivalent aliases/import order yield the same linked plan and source manifests; root ordered effects are never reordered for normalization.

## Diagnostics and resource limits

Diagnostics retain a source-unit ID, original UTF-8 byte span and bounded import trace. AST rewriting preserves original per-unit spans through a bounded virtual source map; errors in imported bodies do not point into generated names or the entry file. Location normalization remains type-directed and does not remove user fields named `span`.

Initial limits: at most 64 imported units, 1 MiB per unit, 4 MiB aggregate entry/module bytes, longest dependency path 16 (including cached shared subtrees), 256 import edges, 10,000 linked declarations including function bodies and 16 MiB retained linked AST. Module IDs use only ASCII letters, digits, dot, underscore and hyphen (no colon or Unicode), keeping raw-module and declaration manifest namespaces disjoint. Revisions are nonempty printable labels. Header IDs/revisions and export/alias identifiers have explicit pre-qualification bounds (module IDs/export names at most 128 UTF-8 bytes; canonical runtime names at most 512). The existing lexer, compiler expansion, rule and runtime budgets remain independent limits. Map bytes and file reads are bounded before retaining untrusted content. Missing/hash/cycle failures happen before emitting a plan.

## Acceptance

- Direct and transitive imports, diamond deduplication, deterministic alias/import-order independence and stable namespace identities.
- Exact schema-constrained functions captured through dependencies; same spelling from distinct module namespaces remains distinct.
- Duplicate identities/aliases, changed bytes under a pin, cycles, unknown exports and wrong declaration kinds rejected with original spans/traces.
- Imported effects rejected even if unused; no imported graph declarations or hidden source reads appear in commands.
- Actual local CLI map resolution and compiler→runtime equivalence with a direct pure implementation; empty typed output retains schemas.
- Literal fields that resemble identifiers or contain `span` remain byte-for-byte semantic data.
- Source pin changes affect module/plan identity; comments affect raw module digest but do not accidentally make source offsets semantic.
- Bounded units/edges/depth/bytes and loader path-policy tests; independent source distribution and WASM compiler checks.

Remote registries, package publishing, dynamic code loading, private exports/reexports, mutually recursive imports, multiple module versions per bundle, signed installation policy and source modules embedded in graph data remain outside this slice.
