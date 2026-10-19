# Core contract v1: migration note from 0.21

Introduced by LA0 ([#1](https://github.com/weave-graph/weave-language/issues/1)),
which tracks engine epic A1. The normative specification is the engine's
[core-v1.md](https://github.com/weave-graph/weave-engine/blob/main/docs/contract/core-v1.md).
This note covers what changes for weave-language users and tools.

## Summary

- **No wire change.** Core v1 is a *profile* (`core-v1`) of protocol
  **0.21.0**, not a new protocol version. The compiler emits the same bytes as
  before. `Program.version` stays `0.21.0`, and so do the vendored crate version
  and `weave_contract::VERSION`.
- **Every emitted plan is checked against v1.** A golden test compiles every
  entry source in `examples/` and decodes the plan and its templates with the
  contract's strict decoder (`tests/core_v1_contract.rs`).
- **The engine's golden fixtures run in language CI.** The vendored crate now
  ships `fixtures/core-v1/` (17 positive and 17 negative JSON fixtures plus a
  hash manifest) and
  `tests/core_v1_fixtures.rs`.
- **`weave describe FILE --contract`** reports which core v1, extension and
  deferred features a source compiles to.

## Differences from reading 0.21 alone

| Area | 0.21 version notes | Core v1 |
|---|---|---|
| Specification | 21 additive notes (`docs/contract/v0.1` to `v0.21`) | One consolidated document for everything classified core v1. The notes still govern extension and deferred features |
| Decoding | serde decoding, one generic input error | `weave_contract::decode_program` checks size, strict JSON (no duplicate keys), then `version`, then structure, with stable codes `E_BUDGET`, `E_DECODE`, `E_VERSION`, `E_UNKNOWN_FIELD` and `E_UNKNOWN_VARIANT`. A program with an unknown version is never partially decoded. `weave-engine run` now reports these codes instead of `E_INPUT` |
| Versions | Implicit | `SUPPORTED_VERSIONS` is the closed set 0.1.0 to 0.21.0. Each version keeps its published semantics, and a feature newer than the declared version fails with `E_VERSION` |
| Feature status | Every feature equal | Each feature is *core v1*, *extension* or *deferred* (table below). No semantics change because of the class |
| Reserved names | None | Rejected by 0.21.0 and reserved for later epics: `provenance` on nodes, edges, structural edges, assertions and attachments; `Node.valid_time`; `GraphData.embedding_spaces` and `embeddings`; result fields `fact_provenance`, `candidates` and `exactness`; commands `assert`, `retract` and `correct`; expressions `expand`, `changes`, `nearest` and `similar_candidates`; the `weave:` predicate and ID namespace |
| Provenance and embeddings | Not modelled | Records (`FactProvenance`, `ClaimedProvenance`, `EmbeddingSpace`, `EmbeddingFact`, `CandidateRelation`) are defined, but no program accepts them yet. Engine A2 and A4 activate them in later minor versions |
| Compatibility | Additive by convention | Written policy: before 1.0.0 every wire change is an additive minor bump; protocol 1.0.0 is cut at the engine Phase A release |

## Language constructs by class

`describe --contract` derives the class from the emitted plan, so this table
is a guide and the command is authoritative.

| Class | Constructs | Contract features |
|---|---|---|
| core v1 | `graph`, `transaction`, schemas with string, integer, float and boolean properties, claims and assertions (including a claim's `context graph`), attachments, `lens`, `pin`, live handles, `join`, `union`, `diff`, `project`, `support`, `explain`, `window`, `sequence`, functions over graphs, `recorded_handle`, `recorded_range` | `commit`, `commit_batch`, `query`, `join`, `bind`, `evaluate`, expressions `query`, `reference`, `filter`, `join`, `metadata`, `union`, `diff`, `project`, `support`, `explain`, `window`, `sequence`, `recorded_query`, command `recorded_range` |
| extension | `rules` and rule application, context selection and contextual attachments, typed contexts, `decimal` and `quantity` schema properties, geometry and typed-vector `distance` (including embedding-role vectors), `counterparts`, `resolve_identity`, `accepted_history`, `accepted_range`, `accepted`, `view_current`, view templates | `reason`, `context`, `attachment_context`, `context_scope`, `context_typing`, `typed_context`, `decimal`, `quantity`, `geometry`, `counterparts`, `resolve_identity`, `accepted_history`, `accepted_range`, `accepted_graph`, `current_view`, `compiled_view_template` |
| deferred | `cluster_navigation`, handler templates | `cluster`, `compiled_handler_template` |

Typed embedding vectors (see [typed vectors](TYPED_VECTORS.md)) still lower to
the 0.9 geometry extension and legacy binary64 vector properties. They are not
core v1 `EmbeddingFact` records, and the engine never reinterprets them as such.
Embedding retrieval in the language waits for engine A4 and language LA1.

## `describe --contract`

```sh
weave describe examples/rules.weave --contract
weave describe examples/modules/main.weave --modules examples/modules/map.json --contract
```

The output is a JSON object:

- `schemas`: the same source schema descriptors that plain `describe` prints.
- `contract.protocol`: the emitted `Program.version`.
- `contract.profile`: `core-v1`.
- `contract.core_only`: true when every emitted feature is core v1.
- `contract.features`: one entry per feature used, ordered by class (core v1,
  extension, deferred), then by name. Each entry has `feature`, `construct`
  (`command`, `expression`, `field` or `artifact`), `since` (the version
  that introduced it), `class` (`core-v1`, `extension` or `deferred`) and
  `locations`.

`locations` are JSON-pointer-style paths into the `artifacts` object that
`weave artifacts FILE` prints, such as `/program/commands/3/value/input` or
`/handler_templates/Warning`.

Unlike plain `describe`, `--contract` compiles view and handler templates
instead of rejecting them with `E_HOST_ARTIFACT_REQUIRED`, because templates
are the constructs it reports as extension and deferred. Plain `describe`
keeps its output (a JSON array of schemas) and behaviour. The library API is
`weave_language::describe_contract`, `LinkedProgram::describe_contract` and
`CompiledArtifacts::contract_profile`.

## Limits

- Locations point into the emitted plan, not to source byte spans.
- The classification of data fields checks the section 12 extension fields of
  `GraphData`: node `context_scope` (which this compiler does not currently
  emit), attachment `context`, `context_typing`, and decimal and quantity
  schema types. Other data fields are core v1.
- The compiler SDK (C ABI and WASM) does not expose `--contract` yet.
- The compiler does not reject author-written `weave:` predicates or IDs. The
  namespace is reserved for the engine.
- The golden example test skips the native scenario `.weave.in` files. They
  are templates with placeholder tokens that the engine harness fills in.
- The language-side fixture check runs the engine's harness on its vendored
  copy. It catches drift between the compiler and the vendored contract. Drift
  between the vendored copy and the engine's main branch is caught when the
  vendor is next updated.
