# Weave Language documentation

The compiler targets engine contract **0.23.0** and its core v1 profile
([CORE_V1.md](CORE_V1.md)). Many feature documents were
written when the feature was introduced and name the protocol version of that
release ("protocol 0.18", "0.15 candidate" and so on). Read those numbers as
*introduced in*, not as the current version. The current version and supported
behavior are in [STATUS.md](STATUS.md).

## Direction

| Document | Use it for |
|---|---|
| [Weave vision](https://github.com/weave-graph/weave-engine/blob/main/VISION.md) | The three goals of Weave and the role of this language |
| [Engine roadmap](https://github.com/weave-graph/weave-engine/blob/main/docs/ROADMAP.md) | Phases A, B and C. The language roadmap follows it |
| [Language roadmap proposal](ROADMAP_PROPOSAL.md) | Approved changes to the L0–L7 epics |
| [Status](STATUS.md) | What works today, and its limits |
| [AGENTS.md](../AGENTS.md) | Architecture, commands, conventions, definition of done |

## Language reference

| Document | Topic | Introduced in |
|---|---|---|
| [Syntax and execution](SYNTAX.md) | Grammar excerpt, CLI, running plans on the engine | 0.1+ |
| [Pinned source modules](MODULES.md) | `--modules` maps, content-pinned imports | 0.15 |
| [Schema functions](SCHEMA_FUNCTIONS.md) | Static graph schema constraints on functions | 0.15 |
| [Scalar functions](SCALAR_FUNCTIONS.md) | Boolean/Integer/String/Time/Decimal/Quantity functions, `weave values` | 0.15 |
| [Decimal](DECIMAL.md) | Exact decimals and quantities | 0.12 |
| [Typed contexts](TYPED_CONTEXTS.md) | Nominal contexts and axis assignments | 0.14 |
| [Live references](LIVE_REFERENCES.md) | Live handles, per-execution pins, snapshot compare-and-swap | 0.14 |
| [View artifacts](VIEW_ARTIFACTS.md) | Accepted graph reads and host view registration | 0.16 |
| [Handler artifacts](HANDLER_ARTIFACTS.md) | Sealed pure event handlers (`handler-plan`) | 0.18 |
| [Intervals](INTERVALS.md) | Checked half-open intervals and predicates | 0.18 |
| [Typed vectors](TYPED_VECTORS.md) | Physical and embedding vectors with descriptors | 0.18 |
| [Bytes and references](BYTES_AND_REFERENCES.md) | Bytes and typed Node/Edge/Assertion/Graph/Object refs | 0.18 |
| [Compiler SDK](COMPILER_SDK.md) | Rust/C/WASM byte API, `sdk/` headers and JS adapter | 0.18 |
| [Temporal queries](TEMPORAL_QUERIES.md) | Graph `Window` and `Sequence` operators | 0.19 |
| [History selection](HISTORY_SELECTION.md) | Recorded handles, accepted history, recorded/accepted ranges | 0.20–0.21 |
| [Time travel, traversal and change sets](TRAVERSAL.md) | `use ... as_of`, `expand`, `changes` and `explain` over them (LA1) | 0.23 |
| [Native services](NATIVE_SERVICES.md) | Cluster navigation and other native service bindings | 0.13 |
| [Formatter](FORMATTER.md) | `weave fmt` | — |
| [Core contract v1](CORE_V1.md) | 0.21 → v1 migration note, 0.22 fact provenance, 0.23 traversal operators, feature classes, `describe --contract` | 0.21–0.23 (core-v1) |
| [Shared protocol](contract/v0.23/README.md) | The current contract notes. Earlier versions are in `contract/v0.*` | 0.23 |

## Planning and release

- [Release readiness](RELEASE_READINESS.md)
- [Publication](PUBLICATION.md)
- [Acceptance gaps](ACCEPTANCE_GAPS.md): a requirement-by-requirement audit
  through protocol 0.17. The requirement rows remain useful; the version
  framing is historical.

## Historical records

These are accurate as dated records:

- [Implementation plan](IMPLEMENTATION_PLAN.md), [workflow](WORKFLOW.md) and
  [workflow.json](workflow.json): the original white-paper gate plan
  (superseded for scheduling).
- [Status history](STATUS_HISTORY.md): the per-version ledger of verified
  checkpoints through 0.23.
- [Protocol 0.15 pairing](PROTOCOL_015.md), [verification 0.17](VERIFICATION_017.md)
  and [source reconciliation](SOURCE_RECONCILIATION.md).
- `proposals/`: design proposals. Most are implemented; each says so in its
  header.
- `source/`: the original white paper and its provenance hashes.
