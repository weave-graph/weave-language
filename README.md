# Weave Language

An experimental graph-native language for composing and explaining multidimensional temporal knowledge graphs, built with [Weave Engine](https://github.com/weave-graph/weave-engine).

**Implemented foundation:** a bounded lexer/parser, basic graph validation, structured diagnostics, pinned graph references and graph-valued metadata, plus pure relation/time filter composition compiled to engine protocol v0.1.0. Rust library and CLI; MIT licensed.

**Not yet complete:** general parameterized graph functions, joins, rules, live views, permission syntax, spatial/embedding operators, clustering, distributed governance and mobile integration. The full roadmap is retained. Original white-paper attachments have not yet been recovered, so the syntax and implementation remain provisional.

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
- [Shared protocol](docs/contract/v0.1/README.md)

The contract is vendored to keep clean checkouts independently buildable; [its manifest](vendor/manifest.json) records exact file hashes. Public source-paper recovery and conformance are separate gates from this executable foundation.
