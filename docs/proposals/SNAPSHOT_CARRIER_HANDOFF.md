# Snapshot carrier: coordinated implementation handoff

Proposed 2026-09-20 after signed-export engine freeze `4b06f72`. This is the exact delta and ownership request for [snapshot influence](SNAPSHOT_INFLUENCE.md). No version, store marker or shared source is changed by this document. The parent owns the build slot until explicitly released.

## Exact fields and meaning

| Canonical owner/file | Addition |
|---|---|
| Language owner: `crates/weave-contract/src/influence.rs` | `GraphInfluence.snapshots: Vec<GraphRef>` |
| Engine owner: `crates/weave-contract/src/lib.rs` | `Node.derived_snapshots`, `Edge.derived_snapshots`, `MetadataAttachment.derived_snapshots`: each `Vec<GraphRef>` |
| Engine owner: `crates/weave-contract/src/assertions.rs` | `Assertion.derived_snapshots: Vec<GraphRef>` |

Each field is `default`, `skip_serializing_if = "Vec::is_empty"`, strict bounded decode and restriction-only. It is a conjunction of current whole-snapshot access gates, independent of record readers and outside derivation OR groups. Empty vectors preserve old canonical bytes. Exact graph/revision identity is mandatory; no branch head or caller authority object appears in these fields.

The attachment field is required now. A graph-host literal attachment can be moved independently of any node/edge host, so a graph-level envelope or host-node gate alone does not protect it after repersistence. The native attachment visibility and attachment-as-AssertionRef proof path both enforce derived_snapshots, whether its host is graph, entity, node, edge or assertion. Metadata traversal/copying preserves the field. Generated protected attachments receive these gates just like generated nodes/edges. This closes the unsupported movable graph-host case in the prior proposal; no fake origin assertion is necessary.

There is no StructuralEdge addition or Derivation snapshot alternative field in this profile. Whole snapshot influence remains conservative AND; alternative proof groups are neither flattened nor rewritten.

## Portable helpers and propagation

Language owner implements these in canonical `influence.rs`, with engine review before native integration:

- Extend `validate`, `canonicalize`, `merge`, `validate_graph` and existing `snapshots(data)` for whole/record/attachment gates.
- Add `validate_record_refs(assertions, nodes, snapshots)`; keep existing `validate_refs(assertions, nodes)` as a compatible wrapper for groups with no snapshot field. Combined raw references per gate ≤1,000; IDs use current limits.
- Add `input_influence(data: &GraphData) -> Result<Option<GraphInfluence>, Diagnostic>`. Merge the existing whole-value carrier with declared snapshot gates from its nodes, edges, explicit assertions and attachments, deduplicating before bounded clones. Do not promote descriptive `QueryResult.input_snapshots` or unrelated record assertion/node proofs into new whole-snapshot gates.
- Extend `protect_generated_result` to copy whole snapshot gates into generated nodes, edges and attachments, with precharged output and per-record limits. Preserve edge group boundaries; snapshots are global AND. Synchronize descriptive input snapshot indexes without fabricating AssertionRefs.

All consuming pure operators merge `input_influence` from each operand before losing filtered records, including context/filter/project and empty output. Retained source records remain byte-unchanged under their origin envelopes. Generated record identity keys and origin-conflict comparisons bind derived_snapshots. Explicit Assertion→Edge materialization copies the field.

The language owner owns portable propagation, constructors and tests in contract `algebra.rs`, `context.rs`, `context_typing.rs`, `counterpart.rs`, `identity.rs`, `rules.rs`, and compile-required fixture adjustments in other contract-only modules except engine-owned files above. The engine owner handles native filter/join/metadata/geometry/clustering/governance/view paths and their generated attachment constructors. Changes to scope beyond these files are coordinated first.

## Native owner obligations

Engine owns shared lib/assertions field insertion, package/protocol version and versioned docs; native authorization context/refactor; exact protected reference and snapshot integrity checks; query/proof/attachment gates; snapshot storage and migration; capsule formats and signed export/admission closure; cache/current-proof guards; native service constructors/tests. Engine also owns the canonical protocol/store/capsule version choices after root review. Language does not hand-edit vendor or claim compatibility before the native profile is coherent.

Whole authorization must share one recursion/work context across snapshot, node, assertion, attachment and typed-context paths; never call the old resetting `authorized()` recursively. Active snapshot keys have a separate namespace. Snapshot cycles deny; ordinary metadata cycles remain supported. Use the reviewed whole_visibility comparator without relaxing proof groups or explanation snapshots. Missing/denied exact references remain generic, bounded, fail-closed results.

Raw plan/capsule references only constrain. Every older Program profile rejects nonempty new fields recursively before any write. Admission/export must scope and reauthorize declared gates, including attachment-only and empty outputs. Cached signed responses recheck the original full closure. A receive operation never installs policy or accepts a head.

## Required review gates and build ordering

1. Engine and parent acknowledge fields, helper names, file ownership and version/profile decision; only then shared edits begin.
2. Language implements portable propagation/tests in the existing canonical engine worktree after the owner inserts its fields. Native work can proceed independently in engine-owned files. No new worktree, cache or agent.
3. Run one owner-coordinated low-priority/two-job compile/test batch at a time, using current caches. Portable and native tests prove empty/isolated and graph-host attachment closure before source vendoring.
4. Native candidate must pass envelope-stripped/readers-cleared repersistence for scalar node, legacy edge, explicit assertion **and graph-host literal attachment**; mixed proof cycles/depth/budget; ordinary metadata cycle control; every operator's empty/filter behavior; old-wire rollback; store migration/previous-binary refusal; capsule/receipt scope and current revocation.
5. Freeze canonical crate/docs and native semantics together. Language vendors that exact revision, fixes source constructors/defaults/versioned examples, runs exact vendor and compiler-to-runtime acceptance, and hands a paired candidate to the parent. No source syntax is needed to manufacture snapshot authority; eventual handler preparation supplies its actual authorized input pins.
6. Only after this carrier acceptance does the source handler bridge proceed. Empty event inputs then use real exact snapshot gates, eliminating the temporary E_HANDLER_INFLUENCE restriction for ordinary authorized empty revisions.
