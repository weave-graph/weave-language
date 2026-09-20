# Weave Language

An experimental graph-native language for composing and explaining multidimensional temporal knowledge graphs, built with [Weave Engine](https://github.com/weave-graph/weave-engine).

**Implemented foundation:** a bounded lexer/parser, basic graph validation, structured diagnostics, pinned graph references and graph-valued metadata, typed relation/time parameters and partial application, plus filter composition and reusable temporal path-join graph values compiled to engine protocol v0.17.0. Versioned scalar schemas, typed endpoints/spaces, source schema discovery, named metadata graph extraction and atomic cyclic local snapshots are also supported. Pure graph functions support graph/scalar/function parameters, immutable partial application and statically proven exact schema constraints on graph parameters and returns. Pure union, membership diff, projection and four-valued support at an explicit instant preserve pinned origins and alternative derivations. Explicit graph profiles separate structural relations from source claims; source-aware fingerprints and portable explanation helpers preserve attribution. Finite range-restricted rule modules provide bounded graph-valued recursive closure with temporal and alternative provenance. Exact default/pinned contexts scope claims, metadata paths and derived values without implicit broadcasts. Typed total context assignments now retain descriptor witnesses and restrictions through empty, derived and saved graph values. Graph-valued distance, explicit frame transforms, embedding distance, display projections and source-level explanations use authorized assertion operands. Declared directed counterpart bridges remain graph-valued evidence with exact source identity and distinct manifestation state. Pinned node influences preserve restrictions on isolated-node copies and derived scalar values. Exact Decimal and nominal Quantity schemas, canonical literals and pure literal arithmetic preserve precision without Float conversion. Ordinary scalar functions additionally specialize Boolean/Integer/String/Time/Decimal/Quantity parameters and results, with typed unary callbacks, bounded work and no runtime scalar effects. Explicit pinned accepted-identity resolution and bounded cluster navigation now return reusable graph values through native host services. Explicit live handles can be pinned into immutable per-execution values; live metadata and full-snapshot CAS replacement preserve fixed historical bindings. Explicit content-pinned pure source modules retain canonical namespaces, source manifests and per-file diagnostics without hidden reads. Exact accepted-decision reads and definition-matched current-view reads preserve native proof restrictions. Source templates produce separate, explicit host registration artifacts with source identities. Rust library and CLI; MIT licensed.

**Not yet complete:** general higher-order graph functions and joins, stratified absence and aggregation, general reactive/live-view syntax, permission syntax, general vector/quantity types and learned mappings, general overlapping clustering, distributed governance and mobile integration. The full roadmap is retained. Original white papers are recovered and [reconciled](docs/SOURCE_RECONCILIATION.md); the implementation remains a partial profile and the syntax is provisional.

```sh
cargo test --locked
cargo run -- check examples/fleet.weave
cargo run -- plan examples/fleet.weave > fleet.plan.json
```

The emitted JSON executes through the engine's trusted host interface. Graph declarations create new graph snapshots, so use a fresh database for the example. Input source cannot grant runtime authority. See [syntax and execution](docs/SYNTAX.md).

- [Implementation plan and requirement matrix](docs/IMPLEMENTATION_PLAN.md)
- [Workflow and dependency graph](docs/WORKFLOW.md)
- [Machine-readable workflow](docs/workflow.json)
- [Verified status and remaining gates](docs/STATUS.md)
- [Acceptance gaps](docs/ACCEPTANCE_GAPS.md)
- [Contributing](CONTRIBUTING.md) and [security policy](SECURITY.md)
- [Protocol 0.15 pairing and selectors](docs/PROTOCOL_015.md)
- [Pinned source modules](docs/MODULES.md)
- [Ordinary scalar functions and `weave values`](docs/SCALAR_FUNCTIONS.md)
- [Accepted graphs and explicit view artifacts](docs/VIEW_ARTIFACTS.md)
- [Shared protocol](docs/contract/v0.17/README.md)

The contract is vendored to keep clean checkouts independently buildable; [its manifest](vendor/manifest.json) records exact file hashes. Public source-paper recovery and conformance are separate gates from this executable foundation.

Build or install from a Git checkout/source archive with `cargo install --locked --path .`. Registry publication is disabled until an optional coordinated crate release; this does not block publishing the open-source repositories. See [release readiness](docs/RELEASE_READINESS.md).

The [source formatter](docs/FORMATTER.md) preserves tokens and comments: `weave fmt FILE.weave` prints formatted source, `--check` checks it and explicit `--write` updates one regular file. Formatting a content-pinned module changes its byte digest and requires explicit relinking.
