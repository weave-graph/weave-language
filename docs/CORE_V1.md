# Core contract v1: migration note from 0.21, with protocols 0.22 to 1.0.0

Introduced by LA0 ([#1](https://github.com/weave-graph/weave-language/issues/1)),
which tracks engine epic A1. The normative specification is the engine's
[core-v1.md](https://github.com/weave-graph/weave-engine/blob/main/docs/contract/core-v1.md).
This note covers what changes for weave-language users and tools.

## Summary

- **No wire change for v1 itself.** Core v1 is a *profile* (`core-v1`)
  introduced at protocol **0.21.0**, not a new protocol version. Engine A2
  then activated per-fact provenance, node valid time and fact-level writes
  as protocol **0.22.0** (see [Protocol 0.22.0](#protocol-0220-engine-a2)),
  and engine A3 activated the `expand`, `as_of` and `changes` query operators
  as protocol **0.23.0** (see [Protocol 0.23.0](#protocol-0230-engine-a3)).
  Engine A4 activated embedding facts, `nearest` and `similar_candidates` as
  protocol **0.24.0** (see [Protocol 0.24.0](#protocol-0240-engine-a4)).
  Engine v1.0.0 froze core v1 as protocol **1.0.0** with no wire change (see
  [Protocol 1.0.0](#protocol-100-engine-v100-phase-a)). The compiler emits
  `weave_contract::VERSION`, now `1.0.0`, equal to the vendored crate
  version. **The language is at Phase A parity on contract 1.0.0.**
- **Every emitted plan is checked against v1.** A golden test compiles every
  entry source in `examples/` and decodes the plan and its templates with the
  contract's strict decoder (`tests/core_v1_contract.rs`).
- **The engine's golden fixtures run in language CI.** The vendored crate now
  ships `fixtures/core-v1/` (30 positive and 25 negative JSON fixtures plus a
  hash manifest at 1.0.0) and
  `tests/core_v1_fixtures.rs`.
- **`weave describe FILE --contract`** reports which core v1, extension and
  deferred features a source compiles to.

## Differences from reading 0.21 alone

| Area | 0.21 version notes | Core v1 |
|---|---|---|
| Specification | 21 additive notes (`docs/contract/v0.1` to `v0.21`) | One consolidated document for everything classified core v1. The notes still govern extension and deferred features |
| Decoding | serde decoding, one generic input error | `weave_contract::decode_program` checks size, strict JSON (no duplicate keys), then `version`, then structure, with stable codes `E_BUDGET`, `E_DECODE`, `E_VERSION`, `E_UNKNOWN_FIELD` and `E_UNKNOWN_VARIANT`. A program with an unknown version is never partially decoded. `weave-engine run` now reports these codes instead of `E_INPUT` |
| Versions | Implicit | `SUPPORTED_VERSIONS` (`weave_contract::versions`, the one version table since 1.0.0) is the closed set 0.1.0 to 0.24.0 and 1.0.0. Each version keeps its published semantics, and a feature newer than the declared version fails with `E_VERSION` |
| Feature status | Every feature equal | Each feature is *core v1*, *extension* or *deferred* (table below). No semantics change because of the class |
| Reserved names | None | Rejected and reserved for later epics: `reader_groups` on fact records (B1); A3 released `expand` and `changes` in 0.23.0; A4 released `GraphData.embedding_spaces` and `embeddings`, result fields `candidates` and `exactness` and expressions `nearest` and `similar_candidates` in 0.24.0; the `weave:` predicate and ID namespace |
| Provenance and embeddings | Not modelled | Records (`FactProvenance`, `ClaimedProvenance`, `EmbeddingSpace`, `EmbeddingFact`, `CandidateRelation`) are defined. Engine A2 activated provenance in 0.22.0. Engine A4 activated embedding spaces, embeddings and candidate relations in 0.24.0 |
| Compatibility | Additive by convention | Written policy: before 1.0.0 every wire change is an additive minor bump; protocol 1.0.0 is cut at the engine Phase A release |

## Language constructs by class

`describe --contract` derives the class from the emitted plan, so this table
is a guide and the command is authoritative.

| Class | Constructs | Contract features |
|---|---|---|
| core v1 | `graph`, `transaction`, schemas with string, integer, float and boolean properties, claims and assertions (including a claim's `context graph`), attachments, `lens`, `pin`, live handles, `join`, `union`, `diff`, `project`, `support`, `explain`, `window`, `sequence`, functions over graphs, `recorded_handle`, `recorded_range`, `use ... as_of`, `expand`, `changes`, `write`, `embedding_space`, `nearest`, `similar_candidates` (LA1) | `commit`, `commit_batch`, `query`, `join`, `bind`, `evaluate`, expressions `query`, `reference`, `filter`, `join`, `metadata`, `union`, `diff`, `project`, `support`, `explain`, `window`, `sequence`, `recorded_query`, command `recorded_range`, command `write` (0.22), expressions `expand`, `as_of` and `changes` (0.23, from `use ... as_of`, `expand` and `changes` source declarations). The compiler emits none of the 0.22 features yet |
| extension | `rules` and rule application, context selection and contextual attachments, typed contexts, `decimal` and `quantity` schema properties, geometry and typed-vector `distance` (including embedding-role vectors), `counterparts`, `resolve_identity`, `accepted_history`, `accepted_range`, `accepted`, `view_current`, view templates | `reason`, `context`, `attachment_context`, `context_scope`, `context_typing`, `typed_context`, `decimal`, `quantity`, `geometry`, `counterparts`, `resolve_identity`, `accepted_history`, `accepted_range`, `accepted_graph`, `current_view`, `compiled_view_template` |
| deferred | `cluster_navigation`, handler templates | `cluster`, `compiled_handler_template` |

Typed embedding vectors (see [typed vectors](TYPED_VECTORS.md)) still lower to
the 0.9 geometry extension and legacy binary64 vector properties. They are not
core v1 `EmbeddingFact` records, and the engine never reinterprets them as such.
Source syntax for 0.24 embedding retrieval is in [HYBRID.md](HYBRID.md).

## Protocol 0.22.0 (engine A2)

Engine A2 ([weave-engine#3](https://github.com/weave-graph/weave-engine/issues/3))
activated the reserved provenance, node valid time and fact-write names as
protocol 0.22.0 ([version note](contract/v0.22/README.md)). Engine
core-v1.md section 12 classifies all of it as **core v1**: the `write`
command (assert, correct, retract) and fact revisions, `Node.valid_time`,
claimed `provenance` on nodes, edges, structural edges, assertions and
attachments, and `QueryResult.fact_provenance`.

For weave-language:

- Plans, view templates and handler artifacts are labelled `0.22.0`. Their
  commands and data are byte-for-byte what 0.21.0 emitted, and the engine
  still accepts 0.21.0 programs and artifacts.
- Results of compiled plans now include `fact_provenance`: the
  engine-recorded writer and recorded time of each source fact the result
  cites. The compiler never supplies either.
- Since LA1 part 2, `write` declarations emit the `write` command, with node
  valid time ([HYBRID.md](HYBRID.md)). There is no source syntax for claimed
  provenance. `describe --contract` classifies them (`write` as a core v1
  command; node `valid_time` and `provenance` as core v1 data, so they are not
  listed), so plans built through the protocol directly classify correctly.

## Protocol 0.23.0 (engine A3)

Engine A3 ([weave-engine#4](https://github.com/weave-graph/weave-engine/issues/4))
activated three query operators as protocol 0.23.0
([version note](contract/v0.23/README.md)). Engine core-v1.md section 12
classifies all of them as **core v1** (section 5 `time.as_of`, section 8
`query.expand` and `query.changes`):

- the `expand` expression: the exact k-hop neighbourhood of seed nodes, by
  direction, predicate and valid time, at a revision, the head or an `as_of`
  recorded time;
- the `as_of` expression (and the `as_of` field of `expand`): a query of the
  branch as the replica knew it at a recorded time, authorized record by
  record;
- the `changes` expression: the facts asserted, retracted or corrected between
  two bounds (`as_of`, `cursor` or `head`), returned in the new optional
  `QueryResult.changes` change set. A change set is terminal.

For weave-language:

- Plans, view templates and handler artifacts are labelled `0.23.0`. Sources
  that use none of the LA1 declarations emit byte-for-byte what 0.22.0 and
  0.21.0 emitted. The engine still accepts 0.22.0 and older programs and
  artifacts.
- LA1 part 1 ([#3](https://github.com/weave-graph/weave-language/issues/3))
  adds source syntax for all three ([TRAVERSAL.md](TRAVERSAL.md)):
  `use ... as_of T` (a lens over it lowers to `as_of`), `expand` and
  `changes`. Plans with `changes` produce `QueryResult.changes`. Views and
  handler recipes cannot contain them (the declarations are top-level only,
  and the engine rejects all three operators in recipes).
- `describe --contract` classifies all three as core v1 expressions (`since`
  0.23). The `as_of` field of `expand` is core v1 data and is not listed
  separately.

## Protocol 0.24.0 (engine A4)

Engine A4 ([weave-engine#13](https://github.com/weave-graph/weave-engine/issues/13))
activated the embedding records as protocol 0.24.0
([version note](contract/v0.24/README.md)). Engine core-v1.md section 12
classifies all of it as **core v1** (section 7, section 8 `query.nearest` and
`query.similar_candidates`): the `embedding_space` and `embedding` fact
kinds of `write`, `GraphData.embedding_spaces` and `embeddings`, the
`nearest` and `similar_candidates` expressions, `ExpandPlan.seeds_from`,
embedding evidence and the result fields `neighbors`, `candidates` and
`exactness`.

For weave-language:

- Plans, view templates and handler artifacts are labelled `0.24.0`. Their
  commands and data are byte-for-byte what 0.23.0 emitted, and the engine
  still accepts 0.23.0 and older programs and artifacts.
- `describe --contract` classifies `nearest` and `similar_candidates` as
  core v1 expressions (`since` 0.24) and walks their `candidates`, `sources`
  and `targets` values, and the `seeds_from` value of `expand`, as inputs.
  Embedding spaces and embeddings in `write` changes are core v1 data and
  are not listed.
- LA1 part 2 ([#3](https://github.com/weave-graph/weave-language/issues/3))
  adds the source syntax: `embedding_space`, embedding facts in `write`,
  `nearest`, `similar_candidates` and `expand ... seeds from`
  ([HYBRID.md](HYBRID.md)).

## Protocol 1.0.0 (engine v1.0.0, Phase A)

Engine v1.0.0 ([weave-engine#16](https://github.com/weave-graph/weave-engine/issues/16),
A6) is the Phase A release. Protocol 1.0.0 freezes core v1 as published in
0.24.0 ([version note](contract/v1.0/README.md)): no wire feature is added or
removed, and stored data, content hashes and revision IDs are unchanged.
`weave_contract::versions` is now the one ordered version table
(`SUPPORTED_VERSIONS`, `version_at_least`, `version_rank`,
`is_supported_version`, `is_current`). From 1.0.0 on, a minor version only
adds what core v1 §11.3 allows.

For weave-language:

- Plans, view templates and handler artifacts are labelled `1.0.0`. Their
  commands, data and source revisions are byte-for-byte what 0.24.0 emitted.
  Handler and view template digests change, because the recorded protocol
  is part of the sealed template. The engine still accepts 0.24.0 and older
  programs and artifacts.
- The compiler has no feature gates of its own and copied no version list.
  `describe --contract` labels (`since`) are introduction versions, and a
  test checks each against the contract table with `version_at_least`.
- **Language at Phase A parity.** Every core v1 operator has source syntax
  (LA1), and the Phase A exit questions run from source
  ([HYBRID.md](HYBRID.md#phase-a-exit-questions)).

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
  schema types. It checks the same node and attachment fields in the facts of
  `write` changes. Other data fields, including 0.22 node `valid_time` and
  `provenance` and the 0.23 `as_of` field of `expand`, are core v1.
- The compiler SDK (C ABI and WASM) does not expose `--contract` yet.
- The compiler does not reject author-written `weave:` predicates or IDs. The
  namespace is reserved for the engine.
- The golden example test skips the native scenario `.weave.in` files. They
  are templates with placeholder tokens that the engine harness fills in.
- The language-side fixture check runs the engine's harness on its vendored
  copy. It catches drift between the compiler and the vendored contract. Drift
  between the vendored copy and the engine's main branch is caught when the
  vendor is next updated.
