# Weave Language

**An optional, AI-friendly language for writing queries and graph computations
against [Weave Engine](https://github.com/weave-graph/weave-engine).**

Weave is an open-source temporal knowledge graph for hardware-heavy industries
and the AI agents that work in them. It relates telemetry, hardware metadata and
user metadata, tracks when every fact was true and when it was recorded, shares
knowledge across people, teams and agents with permissions, and links embeddings
to the graph for time-aware hybrid retrieval. The engine's [vision](https://github.com/weave-graph/weave-engine/blob/main/VISION.md)
and [phased roadmap](https://github.com/weave-graph/weave-engine/blob/main/docs/ROADMAP.md)
describe the goals.

This repository holds a compiler. It turns `.weave` source into the engine's
versioned JSON contract (Programs, view artifacts and handler artifacts). It never
executes anything or grants authority. The engine validates and authorizes
everything. **You do not need this language to use Weave.** The engine's
JSON, Python and Rust interfaces work without it. The language is for people and
agents who want a compact, checkable way to express temporal joins, time travel,
graph functions, rules and explanations.

## Who it is for

- Engineers who want to describe relationships across hardware, telemetry
  references and organizational data concisely, with compile-time checks.
- AI agents that generate queries. The compiler returns machine-readable
  diagnostics, schema discovery (`describe`) and a bounded native/WASM SDK.

## Status

The compiler is experimental, with 214 tests passing, and it targets engine
contract **0.24.0** and its core v1 profile
([migration note](docs/CORE_V1.md)). The syntax is provisional.

| Works today | Not yet |
|---|---|
| Graphs, nodes, first-class edges, graph-valued metadata, pinned and live references | Permission syntax (sharing is engine-side) |
| Valid-time filters, `Window`/`Sequence`, `diff`, recorded and accepted history, as-of reads, k-hop `expand` and `changes` change sets | Source syntax for fact-level writes (assert, correct, retract) |
| Pure graph functions, joins, union, projection, four-valued support, bounded rules, `explain` | Vector search and hybrid retrieval operators (waiting on engine A4; `nearest` is reserved) |
| Typed contexts, exact Decimal/Quantity, typed vectors, intervals, bytes, typed refs | Agent identity and agent-facing workflows (engine Phase B) |
| `check`, `plan`, `ast`, `describe`, `fmt`, view and handler artifacts, native/C/WASM compiler SDK | Distributed and offline governance (deferred) |

See [docs/STATUS.md](docs/STATUS.md) for details and
[docs/ROADMAP_PROPOSAL.md](docs/ROADMAP_PROPOSAL.md) for the approved plan.

## Quickstart

```sh
cargo test --locked
cargo run -- check examples/fleet.weave
cargo run -- plan examples/fleet.weave > fleet.plan.json
```

A small example (from [examples/join.weave](examples/join.weave)) derives which
devices were exposed to an advisory, and when:

```weave
graph Operations {
  node "device" entity "device-17" space "operational";
  node "model-local" entity "model-4" space "catalog";
  edge "uses-model" from "device" to "model-local" relation "model_of" valid 100 until 200;
}
graph Advisories {
  node "model-vendor" entity "model-4" space "catalog";
  node "issue" entity "issue-31" space "advisory";
  edge "affected-model" from "model-vendor" to "issue" relation "affected_by" valid 150 until 250;
}
lens ModelOf from Operations {match relation "model_of";}
lens AffectedBy from Advisories {match relation "affected_by";}
// Joins on stable entity+space identity; the result is valid for [150,200).
join Exposure from ModelOf to AffectedBy relation "exposed_to";
```

The emitted JSON runs through the engine's trusted host, for example
`weave-engine run --db fleet.sqlite --actor demo --write Fleet fleet.plan.json`.
Graph declarations create new snapshots, so use a fresh database. Source code
cannot grant runtime authority. See [syntax and execution](docs/SYNTAX.md).

To install from a checkout, run `cargo install --locked --path .`. Registry
publication is disabled; see [release readiness](docs/RELEASE_READINESS.md).

## Documentation and development

- [Documentation index](docs/README.md): guides by feature, references and
  history.
- [AGENTS.md](AGENTS.md): architecture, commands, conventions and the
  definition of done, for AI agents and contributors.
- [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

The engine's `weave-contract` crate is vendored in `vendor/` with a hash
[manifest](vendor/manifest.json), so clean checkouts build on their own.

MIT licensed.
