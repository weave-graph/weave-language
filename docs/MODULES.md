# Pinned pure source modules

The compiler can link caller-supplied pure source units under the current protocol 0.15 (introduced as a source-only addition on 0.14). No runtime schema, authority, registry, network access or hidden effect is introduced. This is a bounded implementation of reusable source code, not a complete package ecosystem.

Run the transitive example:

```sh
cargo run -- check examples/modules/main.weave --modules examples/modules/map.json
cargo run -- plan examples/modules/main.weave --modules examples/modules/map.json > modules.plan.json
cargo run -- fingerprint examples/modules/main.weave --modules examples/modules/map.json
```

A module starts with `module "example.core" revision "1";`. An import states the exact raw UTF-8 SHA-256 digest:

```weave
import tools module "example.core" revision "1" sha256 "<64 lowercase hex digits>";
graph G schema tools::Network {}
apply Result from tools::Keep { graph input G; }
```

Use `weave_language::modules::{SourceModule, link, compile_with_modules, content_digest}` from library hosts. `link(entry_id, source, supplied)` returns a `LinkedProgram` with `compile`, `schemas`, `fingerprint` and debugging `ast` methods. `ModuleDiagnostic` retains `source_id`, original UTF-8 byte `start/end`, `code/message` and the bounded import trace. The debugging linked AST uses virtual byte offsets; it is not a rewritten source file.

The CLI map is a JSON array of `{ "id": "example.core", "revision": "1", "path": "core.weave" }`. It is an explicit loader policy: bounded regular files, normal relative paths, and canonical targets within the map directory. It is not an operating-system filesystem sandbox. Library linking performs no filesystem operations. Unknown or duplicate map fields and duplicate identities fail. `check`, `plan`, `ast`, `describe` and `fingerprint` accept the same optional map.

All top-level schemas, context schemas, vector type declarations, finite rules and pure functions are exported. Only direct `alias::Name` references work. Imported graph snapshots, reads, handles, effects and top-level applications are rejected even if unused. Module IDs use ASCII letters, digits, dot, underscore and hyphen; alias/export names are ordinary bounded identifiers. Compiler-generated names cannot be authored as references. One module ID has one revision/byte sequence in a bundle; multi-version imports, reexports and dynamic source loading remain unsupported.

Aliases, path names and import declaration order do not alter linked semantic identities. Dependency order is deterministic; diamonds link one exact unit once. Each dependency contributes its raw content digest to `Program.source_revisions`; individual function/rule revisions remain. Comments and line endings change raw pins. Module labels and hashes describe reproducibility, not authenticated authorship. Literal properties, relations, structural type names and entity/space identifiers are never textually rewritten. Vector declaration references are qualified as syntax and resolve to complete canonical descriptors before function hashing. Exact nominal schemas remain distinct across module namespaces.

Limits are 64 supplied units, 1 MiB/unit, 4 MiB total source, longest dependency path 16, 256 visited import edges, 10,000 linked declarations including bodies and 16 MiB retained AST. CLI maps are at most 64 KiB. Missing units, inconsistent pins, cycles, namespace/type errors or budget failures produce no plan. Existing compiler expansion and runtime budgets still apply.

Tests cover schema-constrained transitive captures, source diagnostics, imported rules/context schemas, alias identity, diamond deduplication, cycle/hash/depth failures, forged private symbols and loader policy. `scripts/check_modules.py --engine PATH` compares the real compiler/runtime result with an explicit direct-definition oracle, aligning the intentional nominal schema namespace difference. It checks no extra commands and empty typed result preservation. The full design and remaining limits are in [the stage proposal](proposals/pinned-source-modules.md).
