# Verified status

As of 2026-09-19, this is an experimental compiler foundation, not full Weave implementation and not verified conformance to the unrecovered white papers.

| Gate | Status | Evidence and remaining work |
|---|---|---|
| L0 Source and contract | blocked | Original paper attachments require source recovery. Protocol v0.3.0 is implemented for the subset and vendored with SHA256 manifest. Full paper reconciliation remains mandatory. |
| L1 Front end | in_progress, provisional | Lexer/parser, basic graph validation, JSON diagnostics and check/ast/plan CLI implemented; 22 tests. Full schemas/types/effects/formatter remain. |
| L2 Deterministic semantic kernel | in_progress, provisional | Typed relation/time parameters, partial application, temporal filtering and reusable cross-graph path-join values lower to engine IR. Named intermediate results feed later parameterized lenses and joins without commits. General higher-order lenses/joins, context and four-valued support remain. |
| L3 Full knowledge semantics | in_progress, provisional | Node manifestations, pinned graph-valued edge/node metadata, scalar properties and explicit negative claims represented in IR. Same-commit cyclic metadata references remain unresolved by the content-hash revision design. General identity alignment, rule evaluation, provenance derivation/scenarios remain. |
| L4 Reactive secure integration | proposed | Public bootstrap declarations only; engine must enforce host authorization. Reactive syntax and differential live views remain. |
| L5 Spatial and multiscale language | proposed | No geometry, embedding or clustering language yet. |
| L6 Distributed offline governance | proposed | Portable source structure only; no mobile, networking or governance conformance claim. |
| L7 Tooling and release | in_progress, provisional | MIT license, independent crate, README and three-platform/WASM CI definition exist. No CI run or public release verified here. LSP is optional proposed editor tooling, not a user-scope completion gate. |

## Local verification

- `cargo test --locked`: 22 behavior/conformance tests passed, including half-open interval laws, negative scope/type cases, revision pins, data/code isolation and 5,000 deterministic malformed input cases.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- `cargo run --quiet -- check examples/fleet.weave`: valid, three commands.
- `cargo run --quiet -- plan examples/fleet.weave`: emitted version 0.3.0 protocol plan.

The checks above were run locally on macOS with Rust 1.94.0. A CI definition is not evidence of remote CI success. `cargo check --locked --lib --target wasm32-unknown-unknown` passed; browser-host execution and mobile integration remain unverified. Exact cross-project integration is recorded separately by the orchestrator.

## Publication boundary

The private, partial chat export is excluded by `.gitignore`; it is not an original white paper and must not enter public history. Public docs summarize requirements without publishing unrelated private conversation material. Source-paper publication/reconciliation remains incomplete. Public GitHub organization and repositories are orchestrator-owned; this status file does not imply they exist.

## Independent orchestrator review

The orchestrator verified a clean independent clone of foundation commit `573befd` against engine `0cee0e1`: locked tests and actual CLI integration passed for metadata partial coverage, pinned replay after restart, half-open time boundaries, host authorization and optimistic concurrency. The orchestrator also verified the v0.2 join slice: time intersection, both premise revisions, principal-scoped output, disjoint/private/negative/identity-mismatched exclusion and collision-safe local IDs.

## Composed-value integration

The v0.3 example was executed against the native engine CLI in a fresh temporary database. A first join fed a parameterized lens and a second join, followed by a final time filter. Ten commands produced three explicit source commits and one final `needs_fix` edge valid over `[170, 200)`, with three input snapshots and exactly the three leaf premises `uses-model`, `affected-model`, and `repair`. No intermediate derived value was committed. The source's negative repair claim did not generate a positive conclusion.

Reproduce with `python3 scripts/check_composed.py --engine /path/to/weave-engine`. This is an actual cross-project local execution result; separate independent orchestrator review is still required for the new stage.
