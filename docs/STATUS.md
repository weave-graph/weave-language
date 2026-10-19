# Current status

As of **19 October 2026**: an experimental compiler at `main` targeting engine
contract **0.22.0** and its **core v1** profile (LA0, see
[CORE_V1.md](CORE_V1.md)). It vendors the complete 82-file contract crate from
engine `61eccfb90e5a` (A2 part 2), including the core v1 golden fixtures, which
`scripts/verify_contract.py` checks against `vendor/manifest.json`.
`cargo test --workspace --locked` passes 203 tests locally. CI runs Linux, macOS
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

- k-hop traversal, change-set ("what changed between K1 and K2") queries and
  vector search or hybrid retrieval. These wait for the engine Phase A operators
  (A3, A4) and are planned as LA1, which starts after engine A1.
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
