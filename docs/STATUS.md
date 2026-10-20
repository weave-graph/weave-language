# Current status

As of **20 October 2026**: **the language is at Phase A parity, on contract
1.1.0.** The compiler at `main` targets engine contract **1.1.0** (engine B1
part 1: group readers and delegation provenance) and its **core v1** profile (LA0, see
[CORE_V1.md](CORE_V1.md)). Every Phase A operator has source syntax (LA1,
[#3](https://github.com/weave-graph/weave-language/issues/3)), and the five
Phase A exit questions run from `.weave` source on a real engine
(`scripts/check_phase_a.py`). It vendors the complete 103-file contract crate
from engine `5a19549` (B1 part 1), including the core v1 golden fixtures, which
`scripts/verify_contract.py` checks against `vendor/manifest.json`.
`cargo test --workspace --locked` passes 224 tests locally. CI runs Linux, macOS
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
  source fact). Since LA1 part 2 the `write` declaration emits the `write`
  command with node valid time ([HYBRID.md](HYBRID.md)). There is no source
  syntax for claimed `provenance` or `readers`, and the compiler never
  supplies a writer or recorded time.
- **Protocol 1.1.0 (engine B1 part 1):** plans, view templates and handler
  artifacts are labelled 1.1.0, with the same commands and data as 1.0.0.
  The compiler never emits `reader_groups`, so its plans have no group
  readers; `describe --contract` lists `reader_groups` as a core v1 field
  (since 1.1) where a program built through the protocol uses it.
- **Protocol 1.0.0 (engine v1.0.0, A6):** plans, view templates and handler
  artifacts are labelled 1.0.0, with the same commands and data as 0.24.0.
  1.0.0 freezes core v1 as published in 0.24.0: there is no wire change.
- **Protocol 0.24.0 (engine A4) and LA1 part 2** ([HYBRID.md](HYBRID.md)):
  plans, view templates and handler artifacts were labelled 0.24.0 (1.0.0
  since the v1.0.0 vendor update).
  - Fact writes: `write W graph "g" { assert|correct node ...; assert|correct
    edge ...; retract edge "id"; assert space S; assert embedding ...; }`
    lowers to one `write` command (one fact revision).
  - Embedding spaces: `embedding_space S id ... dimensions N metric ...;`
    declares the full descriptor. Every vector is checked against it at
    compile time (`E_SPACE_MISMATCH`), including typed query vectors.
  - Hybrid retrieval: `nearest` (literal vector, typed vector value or `like
    embedding`, `k`, `at`, `candidates V`), `similar_candidates` (`k`,
    `max_score`, `at`, `sources`, `targets`, `exclude_linked`, `limit`) and
    `expand ... { seeds from V; }`.
  - `describe --contract` classifies all of it as core v1.
  - Acceptance: `tests/hybrid.rs` (golden plans, the A4 fixtures'
    expressions, diagnostics) and `scripts/check_phase_a.py`, which answers
    Phase A exit questions 1 to 5 from `examples/phase_a` on a real engine
    against JSON references and independent oracles.
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

- Parameters for the LA1 clauses: seeds, relations, entities, times, `k`,
  vectors and limits are literals (or typed vector values), not function or
  `bind` parameters. The store reads and `write` are top-level declarations
  only.
- Source writes are public: no `readers`, gates, claimed provenance or
  per-fact `expected` concurrency, and no attachment or structural-relation
  facts. Correcting a restricted fact from source would publish its new
  version, so write restricted data through the protocol (permission syntax
  is LB1).
- **Group readers (`reader_groups`, contract 1.1.0): not yet supported in
  source.** Group-reader and other permission syntax is LB1
  ([#5](https://github.com/weave-graph/weave-language/issues/5)). Results of
  compiled plans can carry engine-recorded `on_behalf_of` delegation chains
  in `fact_provenance`; the compiler never supplies them.
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
