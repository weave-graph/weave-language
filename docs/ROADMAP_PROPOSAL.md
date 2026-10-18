# Proposed GitHub issue changes (language)

Status: **approved by the owner on 2026-10-18.**
It follows the engine's proposed
[phased roadmap](https://github.com/weave-graph/weave-engine/blob/main/docs/ROADMAP.md)
and [vision](https://github.com/weave-graph/weave-engine/blob/main/VISION.md). The
engine-side proposal is
[weave-engine/docs/ROADMAP_PROPOSAL.md](https://github.com/weave-graph/weave-engine/blob/main/docs/ROADMAP_PROPOSAL.md).

## Position of the language

The language is optional: the engine's JSON, Python and Rust interfaces cover
every goal without it. It earns its place in two ways:

- as a compact, checkable way for engineers to express temporal, graph and
  hybrid queries;
- as an agent-friendly query surface, through structured diagnostics,
  `describe` and a bounded SDK.

**Sequencing (approved):** until engine epic A1 (core contract v1) lands,
language work is limited to contract synchronization and maintenance. **LA1
starts only after engine A1 lands**, and builds against the Phase A operators
as they stabilize. The engine's JSON and Python
interfaces remain the Phase A critical path.

## How to apply, once approved

Use the same procedure as the engine:

1. Create the labels `phase-a`, `phase-b`, `phase-c`, `deferred` and `epic`.
2. Post each old issue body as a "Historical checkpoint evidence" comment before
   replacing it.
3. Close merged issues with a link to the issue that absorbs them.

## Summary of operations

| Op | Issue | New title | Label |
|---|---|---|---|
| retitle + rebody | #1 L0 | LA0: Track core contract v1 | phase-a |
| close as merged | #2 L1 | → #8 (LT) | — |
| retitle + rebody | #3 L2 | LA1: Query language for the Phase A core | phase-a |
| close as merged | #4 L3 | → #3 (LA1); identity-map, scenario and manifestation rows deferred | — |
| retitle + rebody | #5 L4 | LB1: Agent-safe compilation and permission-aware queries | phase-b |
| label + prepend note | #6 L5 | (unchanged title) | deferred; embedding search moves to #3 |
| label + prepend note | #7 L6 | (unchanged title) | deferred |
| retitle + rebody | #8 L7 | LT: Tooling, maintenance and per-phase releases | ongoing |

**Deferral note** for #6:

> **Deferred (roadmap of <date>).** Embedding search and hybrid retrieval syntax
> moved to #3 (LA1). Geometry, clustering and semantic-zoom language work is not
> scheduled in Phases A–C. Existing typed-vector, geometry and cluster-navigation
> support is kept and must keep passing.

**Deferral note** for #7:

> **Deferred (roadmap of <date>).** Decentralized peer sync and offline
> governance are not among Weave's three goals for Phases A–C. Phase B shares
> knowledge through an authenticated engine service. Existing accepted-history
> support is kept.

**Closing comment** for #2:

> Merged into #8 (LT). The front end (parser, diagnostics, formatter, hostile-input
> limits) is implemented; the remaining work is maintenance and tooling. The
> evidence above stays valid.

**Closing comment** for #4:

> Merged into #3 (LA1) under the phased roadmap. Explain and provenance (L20)
> and safe recursion (L22) carry into LA1. Identity-map acceptance (L19),
> scenarios (L23) and broader manifestation operations (L06) are deferred.

## Common footer for every epic body

> **Definition of done.** Every criterion has a linked test, full CI passes
> (fmt, Clippy, tests, contract verification, WASM parity), docs and
> `docs/STATUS.md` are updated in the same PR, and contract changes are
> coordinated with the engine and its golden fixtures. Progress notes go in
> comments, not this body.

---

## LA0: Track core contract v1 (rebody #1)

**Goal.** The compiler emits and validates exactly the engine's core contract
v1 (engine epic A1).

**Acceptance criteria.**
- [ ] Vendor the engine's core contract v1 crate, with an updated
  `vendor/manifest.json`, and pass `verify_contract.py`.
- [ ] The engine's golden fixtures run in language CI. Every plan the compiler
  emits for `examples/` validates against v1.
- [ ] Constructs that compile only to v1 *extension* or *deferred* features are
  reported by `describe` as such.
- [ ] Docs name 0.21 → v1 differences in a migration note.

**Depends on:** engine A1.

---

## LA1: Query language for the Phase A core (rebody #3, absorbs #4 and the embedding part of #6)

**Goal.** Express the Phase A questions directly: time travel, traversal,
change sets, hybrid retrieval and explanation.

**Scope.**
- Syntax for `expand` (k-hop, with predicate and direction filters, valid-at and
  as-of) and `changes between K1 and K2` (or `since cursor`), lowering to the
  engine A3 operators.
- Syntax for `nearest` (space, k, query vector or fact, candidate set from a
  graph expression) and candidate relations, lowering to A4. The type checker
  enforces the space descriptor.
- `explain` across the new operators.
- Fact-level writes (assert, retract, correct) lowering to the A2 write API, in
  place of whole-snapshot declarations.

**Acceptance criteria.**
- [ ] Each Phase A exit-scenario question
  ([engine roadmap](https://github.com/weave-graph/weave-engine/blob/main/docs/ROADMAP.md#phase-a-rock-solid-shared-core))
  is expressible in at most 15 lines of `.weave`, and its compiled plan produces
  the same result as the engine's JSON reference.
- [ ] A space or descriptor mismatch in `nearest` is a compile-time diagnostic
  with a stable code and exact span.
- [ ] Compiled `changes` and `expand` results match engine oracle fixtures.
- [ ] Native/WASM parity covers the new constructs.
- [ ] SYNTAX.md and examples cover each construct.

**Starts after:** engine A1 has landed. **Depends on:** engine A2–A4 contracts
and LA0.

---

## LB1: Agent-safe compilation and permission-aware queries (rebody #5)

**Goal.** Agents can safely generate and repair Weave queries, and the language
can express sharing intent that the engine enforces.

**Scope.**
- A `describe` manifest of the schemas, graphs and operators visible to the
  calling principal, served by the engine B3/B4 service.
- Syntax for declaring readers or groups on asserted facts. It lowers to the B1
  principal model and never grants authority, because the engine checks
  everything.
- A compile tool exposed through the engine's MCP server (B4, which extends the
  A7 read-only preview): source in, plan and diagnostics out, with no host
  effects.
- Data/code boundary: compiling untrusted text cannot produce effects beyond
  the declared plan.

**Acceptance criteria.**
- [ ] A scripted agent test discovers a schema, writes a query with a type error,
  repairs it from the diagnostics and runs it, with no hard-coded field names.
- [ ] Declared readers lower to B1 groups. A test shows the engine rejecting a
  declaration that exceeds the writer's authority.
- [ ] Fuzzed hostile source cannot produce a plan containing commands not present
  in the source.

**Depends on:** engine B1, B3 and B4. The reactive parts of the old L4 (watch
plans, rolling windows) wait for engine C3.

---

## LT: Tooling, maintenance and per-phase releases (rebody #8, absorbs #2)

**Goal.** Keep the compiler healthy and release it in step with engine phases.

**Acceptance criteria (rolling, per phase).**
- [ ] A tagged release accompanies each engine phase release (A6, B6, C6), with
  notes stating the supported contract version.
- [ ] CI covers Linux, macOS and Windows plus WASM, and verifies the contract.
- [ ] Docs carry no stale "current version" claims; `docs/README.md` marks the
  version each feature was introduced in.
- [ ] Optional: an LSP or editor integration, only if it does not delay LA1 or LB1.
