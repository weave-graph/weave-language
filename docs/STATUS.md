# Verified status

As of 2026-09-19, this is an experimental compiler foundation, not full Weave implementation and not full conformance to the recovered architecture white papers.

| Gate | Status | Evidence and remaining work |
|---|---|---|
| L0 Source and contract | in_progress | Original papers recovered and reconciled; exact language source and hashes are included. Protocol v0.4 schema/metadata implementation addresses documented model gaps. Full semantic conformance remains open. |
| L1 Front end | in_progress, provisional | Lexer/parser, versioned scalar schemas, typed endpoint/space validation, JSON diagnostics and check/ast/plan/describe CLI implemented; 32 tests. Rich graph/vector/quantity types, effects and formatter remain. |
| L2 Deterministic semantic kernel | in_progress, provisional | Typed relation/time parameters, partial application, temporal filtering and reusable cross-graph path-join values lower to engine IR. Named intermediate results feed later parameterized lenses and joins without commits. General higher-order lenses/joins, context and four-valued support remain. |
| L3 Full knowledge semantics | in_progress, provisional | Named attributable graph attachments, native metadata-value extraction and real cyclic local snapshots now execute through a logical manifest; typed schema meanings are retained through query/join results. General identity alignment, rule evaluation, provenance derivation/scenarios remain. |
| L4 Reactive secure integration | proposed | Public bootstrap declarations only; engine must enforce host authorization. Reactive syntax and differential live views remain. |
| L5 Spatial and multiscale language | proposed | No geometry, embedding or clustering language yet. |
| L6 Distributed offline governance | proposed | Portable source structure only; no mobile, networking or governance conformance claim. |
| L7 Tooling and release | in_progress, provisional | MIT license, independent crate, README and three-platform/WASM CI definition exist. No CI run or public release verified here. LSP is optional proposed editor tooling, not a user-scope completion gate. |

## Local verification

- `cargo test --locked`: 32 behavior/conformance tests passed, including half-open interval laws, negative scope/type cases, revision pins, data/code isolation and 5,000 deterministic malformed input cases.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- `cargo run --quiet -- check examples/fleet.weave`: valid, three commands.
- `cargo run --quiet -- plan examples/fleet.weave`: emitted version 0.4.0 protocol plan.

The checks above were run locally on macOS with Rust 1.94.0. A CI definition is not evidence of remote CI success. `cargo check --locked --lib --target wasm32-unknown-unknown` passed; browser-host execution and mobile integration remain unverified. Exact cross-project integration is recorded separately by the orchestrator.

## Publication boundary

The private, partial chat export is excluded by `.gitignore`; it is not an original white paper and must not enter public history. Public docs summarize requirements without publishing unrelated private conversation material. The original language paper is preserved under docs/source with exact provenance; source reconciliation is complete. Full implementation remains incomplete. The public MIT repositories are https://github.com/weave-graph/weave-language and https://github.com/weave-graph/weave-engine. Publication and CI verification are orchestrator-owned; a local commit is not necessarily the current remote head.

## Independent orchestrator review

The orchestrator verified a clean independent clone of foundation commit `573befd` against engine `0cee0e1`: locked tests and actual CLI integration passed for metadata partial coverage, pinned replay after restart, half-open time boundaries, host authorization and optimistic concurrency. The orchestrator also verified the v0.2 join slice: time intersection, both premise revisions, principal-scoped output, disjoint/private/negative/identity-mismatched exclusion and collision-safe local IDs.

## Composed-value integration

The v0.3 example was executed against the native engine CLI in a fresh temporary database. A first join fed a parameterized lens and a second join, followed by a final time filter. Ten commands produced three explicit source commits and one final `needs_fix` edge valid over `[170, 200)`, with three input snapshots and exactly the three leaf premises `uses-model`, `affected-model`, and `repair`. No intermediate derived value was committed. The source's negative repair claim did not generate a positive conclusion.

Reproduce with `python3 scripts/check_composed.py --engine /path/to/weave-engine`. This is an actual cross-project local execution result; separate independent orchestrator review is still required for the new stage.

## Source distribution audit

The independent source archive at `ea484f7f1c24932b177e4a87daf7bb875cecb671` passed contract verification, all 22 tests with locked cached dependencies, CLI installation, and the composed-example compile check without a sibling engine checkout. The archive contained the complete vendored contract and excluded the private source export. [Release readiness](RELEASE_READINESS.md) records exact evidence and publication limits; [acceptance gaps](ACCEPTANCE_GAPS.md) maps all 30 remaining full-design requirement areas.

## Paper-directed v0.4 verification

32 frontend tests cover schema field/type/space errors, discovery, named metadata, local transaction boundaries and graph-value composition. The shared portable validator has four additional engine-contract tests. Strict Clippy and the WASM library target check pass.

Actual CLI runs of `schema.weave` retain the graph schema. `metadata_cycle.weave` commits the two mutually referring snapshots atomically, traverses the real cycle with complete coverage, selects an edge's evidence graph and joins it to an independent catalog. Its final provenance contains the evidence edge, the named host attachment and the catalog review edge. The orchestrator independently ran both examples through the compiler/runtime.

Full schema types/migrations, live-handle source syntax, arbitrary attachment policies, source-level rebinding and the remaining full-design gates are still incomplete. Current docs and source artifacts do not imply those capabilities.

The typed cross-schema join example deliberately reuses type names with different definitions. Its actual runtime result retains two distinct resolvable node types, a declared derived edge type and valid interval `[150, 200)`. The compiler rejects statically known mixed typed/untyped joins; dynamic inputs remain subject to the same runtime check.
