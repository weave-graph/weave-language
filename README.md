# Weave Language

An experimental graph-native language for composing and explaining multidimensional temporal knowledge graphs, built with [Weave Engine](https://github.com/weave-graph/weave-engine).

**Implemented foundation:** a bounded lexer/parser, basic graph validation, structured diagnostics, pinned graph references and graph-valued metadata, typed relation/time parameters and partial application, plus filter composition and reusable temporal path-join graph values compiled to engine protocol v0.7.0. Versioned scalar schemas, typed endpoints/spaces, source schema discovery, named metadata graph extraction and atomic cyclic local snapshots are also supported. Pure graph functions support graph/scalar/function parameters and immutable partial application. Pure union, membership diff, projection and four-valued support at an explicit instant preserve pinned origins and alternative derivations. Explicit graph profiles separate structural relations from source claims; source-aware fingerprints and portable explanation helpers preserve attribution. Finite range-restricted rule modules provide bounded graph-valued recursive closure with temporal and alternative provenance. Rust library and CLI; MIT licensed.

**Not yet complete:** general higher-order graph functions and joins, stratified absence and aggregation, live views, permission syntax, spatial/embedding operators, clustering, distributed governance and mobile integration. The full roadmap is retained. Original white papers are recovered and [reconciled](docs/SOURCE_RECONCILIATION.md); the implementation remains a partial profile and the syntax is provisional.

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
- [Shared protocol](docs/contract/v0.7/README.md)

The contract is vendored to keep clean checkouts independently buildable; [its manifest](vendor/manifest.json) records exact file hashes. Public source-paper recovery and conformance are separate gates from this executable foundation.

Build or install from a Git checkout/source archive with `cargo install --locked --path .`. Registry publication is disabled until an optional coordinated crate release; this does not block publishing the open-source repositories. See [release readiness](docs/RELEASE_READINESS.md).
