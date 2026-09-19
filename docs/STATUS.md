# Verified status

As of 2026-09-19, this is an experimental compiler foundation, not full Weave implementation and not verified conformance to the unrecovered white papers.

| Gate | Status | Evidence and remaining work |
|---|---|---|
| L0 Source and contract | blocked | Original paper attachments require source recovery. Protocol v0.1.0 is implemented for the subset and vendored with SHA256 manifest. Full paper reconciliation remains mandatory. |
| L1 Front end | in_progress, provisional | Lexer/parser, basic graph validation, JSON diagnostics and check/ast/plan CLI implemented; 14 tests. Full schemas/types/effects/formatter remain. |
| L2 Deterministic semantic kernel | in_progress, provisional | Temporal interval validation and pure relation/time query composition lower to engine IR. General lenses, parameters, joins, context, four-valued support remain. |
| L3 Full knowledge semantics | in_progress, provisional | Node manifestations and pinned graph-valued edge/node metadata represented in IR. General identity alignment, rule evaluation, provenance derivation/scenarios remain. |
| L4 Reactive secure integration | proposed | Public bootstrap declarations only; engine must enforce host authorization. Reactive syntax and differential live views remain. |
| L5 Spatial and multiscale language | proposed | No geometry, embedding or clustering language yet. |
| L6 Distributed offline governance | proposed | Portable source structure only; no mobile, networking or governance conformance claim. |
| L7 Tooling and release | in_progress, provisional | MIT license, independent crate, README and three-platform/WASM CI definition exist. No CI run or public release verified here. LSP is optional proposed editor tooling, not a user-scope completion gate. |

## Local verification

- `cargo test --locked`: 14 behavior/conformance tests passed, including half-open interval laws, negative scope/type cases, revision pins, data/code isolation and 5,000 deterministic malformed input cases.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- `cargo run --quiet -- check examples/fleet.weave`: valid, three commands.
- `cargo run --quiet -- plan examples/fleet.weave`: emitted version 0.1.0 protocol plan.

The checks above were run locally on macOS with Rust 1.94.0. A CI definition is not evidence of remote CI success. WASM/mobile support requires separate target/build/host validation. Exact cross-project integration is recorded separately by the orchestrator.

## Publication boundary

The private, partial chat export is excluded by `.gitignore`; it is not an original white paper and must not enter public history. Public docs summarize requirements without publishing unrelated private conversation material. Source-paper publication/reconciliation remains incomplete. Public GitHub organization and repositories are orchestrator-owned; this status file does not imply they exist.
