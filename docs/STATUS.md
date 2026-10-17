# Current status

As of **17 October 2026**: an experimental compiler at `main` targeting engine
contract **0.21.0**. It vendors 37 contract files, which `scripts/verify_contract.py`
checks against `vendor/manifest.json`. `cargo test --workspace --locked` passes
198 tests locally. CI runs Linux, macOS and Windows plus WASM parity
(`.github/workflows/ci.yml`).

The per-version checkpoint ledger is in [STATUS_HISTORY.md](STATUS_HISTORY.md).

## Supported

- CLI commands: `check`, `plan`, `ast`, `describe`, `fingerprint`, `values`,
  `artifacts`, `view-plan`, `handler-plan` and `fmt`. Pinned modules are
  linked with `--modules`.
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
  (A3, A4) and are proposed as LA1.
- Permission syntax and agent-facing workflows (Phase B, proposed LB1). All
  source-created records are public; restricted fixtures are built through the
  protocol directly.
- Mixed graph-to-scalar functions, general named joins with explicit
  resolution, stratified negation and aggregation.
- Distributed and offline governance, clustering as a language feature, and
  general geometry. These are deferred.

The syntax is provisional. Emitted plans are validated and authorized by the
engine; the compiler is never a security boundary.
