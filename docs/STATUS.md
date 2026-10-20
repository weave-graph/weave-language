# Current status

As of **20 October 2026**: an experimental compiler at `main` targeting engine
contract **0.24.0** and its **core v1** profile (LA0, see
[CORE_V1.md](CORE_V1.md)). It vendors the complete 98-file contract crate from
engine `5e2a1d9bbf71` (A4 part 2), including the core v1 golden fixtures, which
`scripts/verify_contract.py` checks against `vendor/manifest.json`.
`cargo test --workspace --locked` passes 214 tests locally. CI runs Linux, macOS
and Windows plus WASM parity, and runs the engine's core v1 fixture harness on
the vendored crate (`.github/workflows/ci.yml`).

The per-version checkpoint ledger is in [STATUS_HISTORY.md](STATUS_HISTORY.md).

## Supported

- CLI commands: `check`, `plan`, `ast`, `describe`, `fingerprint`, `values`,
  `artifacts`, `view-plan`, `handler-plan` and `fmt`. Pinned modules are
  linked with `--modules`.
- **Core contract v1:** every plan and template emitted for `examples/`
  decodes with the contract's strict decoder (`decode_program` for plans,
  `decode` for templates) and re-encodes unchanged.
  `weave describe FILE --contract` classifies what a source compiles to as
  core v1, extension or deferred (engine core-v1.md section 12), with exact
  plan locations. See [CORE_V1.md](CORE_V1.md) for the classes and limits.
- **Protocol 0.22.0 (engine A2):** plans, view templates and handler artifacts
  are labelled 0.22.0, so engine results of compiled plans carry
  `fact_provenance` (engine-recorded writer and recorded time for each cited
  source fact). The compiler emits no 0.22-only field: it has no syntax for
  the `write` command, node `valid_time` or claimed `provenance`, and it never
  supplies a writer or recorded time.
- **Protocol 0.24.0 (engine A4):** plans, view templates and handler
  artifacts are labelled 0.24.0, with the same commands and data as 0.23.0.
  `describe --contract` classifies the new `nearest` and
  `similar_candidates` expressions as core v1 for programs built through the
  protocol.
- **Protocol 0.23.0 (engine A3) and LA1 part 1 query syntax**
  ([TRAVERSAL.md](TRAVERSAL.md)): plans, view templates and handler
  artifacts are labelled 0.23.0 (0.24.0 since the A4 vendor update).
  - Time travel: `use G graph "g" as_of T;` reads the branch as known at
    recorded time T and lowers a lens over it to `as_of`; `at` keeps selecting
    valid time.
  - k-hop traversal: `expand E from G { seeds ...; hops N; direction ...;
    match relation ...; at T; limit N; }` lowers to `expand`, at the head, a
    pinned revision or an `as_of` time.
  - Change sets: `changes C from G { between B1 and B2; entities ...; }` (or
    `since B`), with `as_of T`, `cursor "R"` and `head` bounds, lowers to
    `changes`. A change set is terminal (`E_CHANGES_OPERAND`).
  - `explain` and every graph operator compose over `expand` and `as_of`
    values. `describe --contract` classifies all three operators as core v1.
  - Acceptance: golden plan tests and fixture equality (`tests/traversal.rs`)
    and `scripts/check_traversal.py`, which runs the engine's core v1 oracle
    fixtures and Phase A question 2 on a real engine.
- **Graphs:** nodes with entity and space, first-class edges, claims and
  assertions, graph-valued metadata attachments, pinned and live references,
  and atomic transactions.
- **Time:**
  - `valid` intervals and valid-at filters;
  - `Window`/`Sequence` (before/meets/overlaps/within);
  - `diff`;
  - recorded handles (`known_at`), and recorded and accepted ranges and history.
- **Computation:**
  - lenses, binds and joins (entity-space), union, projection and four-valued
    `support`;
  - pure graph and scalar functions with partial application;
  - bounded rules;
  - `explain`.
- **Types:** typed contexts, exact Decimal and nominal Quantity, Interval, typed
  physical and embedding vectors (with `distance`), Bytes, and typed references.
- **Artifacts:** compiled view templates and sealed handler artifacts, installed
  only by the engine's trusted host.
- **SDK:** a bounded Rust, C ABI (`sdk/weave-compiler.h`) and zero-import WASM
  (`sdk/weave-compiler.mjs`) compiler API.

## Not supported, mapped to the [engine roadmap](https://github.com/weave-graph/weave-engine/blob/main/docs/ROADMAP.md)

- **Hybrid search and embeddings: not yet supported in source.** Engine A4
  ([weave-engine#13](https://github.com/weave-graph/weave-engine/issues/13))
  activated them in protocol 0.24.0, and `describe --contract` classifies
  `nearest` and `similar_candidates` as core v1. The source syntax for
  embeddings, `nearest`, candidate relations and the space-descriptor check
  is LA1 part 2 ([#3](https://github.com/weave-graph/weave-language/issues/3)).
  The keywords `nearest` and `similar_candidates` are reserved and fail with
  `E_NOT_YET_SUPPORTED`.
- Parameters for the LA1 clauses: seeds, relations, entities, times and
  limits are literals, not function, `bind` or scalar parameters. `expand`
  and `changes` are top-level declarations only.
- Source syntax for fact-level writes (`write`: assert, correct, retract),
  node valid time and claimed per-fact provenance (engine A2, protocol 0.22).
  Source `graph` and `transaction` still lower to snapshot `commit` and
  `commit_batch`.
- Permission syntax and agent-facing workflows (Phase B, planned as LB1). All
  source-created records are public; restricted fixtures are built through the
  protocol directly.
- Mixed graph-to-scalar functions, general named joins with explicit
  resolution, stratified negation and aggregation.
- Distributed and offline governance, clustering as a language feature, and
  general geometry. These are deferred.

Constructs classified *extension* (rules, contexts, typed contexts, decimals
and quantities, geometry and typed vectors, counterparts, identity resolution,
accepted reads, views) stay supported, but engine Phase A does not build on
them. Constructs classified *deferred* (`cluster_navigation`, handler
templates) are maintained without new development.

The syntax is provisional. Emitted plans are validated and authorized by the
engine; the compiler is never a security boundary.
