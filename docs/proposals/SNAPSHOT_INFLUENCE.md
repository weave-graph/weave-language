# Exact snapshot influence before compiled handlers

Design/review only, 2026-09-20. Inspected the protocol-0.16 portable influence, algebra, context, identity and rules code, and current native influence/authorization/dispatch/capsule/admission paths. No canonical fields, version, store marker or implementation are changed here. This replaces the temporary empty-input restriction in [the handler interface](REACTOR_INTERFACE.md) once implemented and verified.

## Why an exact snapshot gate is needed

AssertionRef requires a real assertion/attachment, and NodeRef requires a real node. Neither names the authorization of a genuinely empty graph. A typed-context witness requires a real definition assertion and is not a generic graph-existence proof. QueryResult.input_snapshots is a descriptive pin vector, not a persisted authority gate. Current accepted-governance carriers happen to protect governed empty values, but cannot stand in for an arbitrary input revision. No existing carrier is a general replacement.

Add an exact revision restriction: a result may be released only while the current principal can authorize the **whole immutable referenced snapshot**. It does not grant access, prove an assertion true, establish event producer identity or require the revision to remain the branch head. Adding a serialized reference can only make a value less available. This handles empty, isolated-node, metadata-only and ordinary handler input revisions without inventing assertions.

A whole snapshot gate deliberately denies when even one stored proof alternative/object would be withheld. It is stricter than an authorized partial Query over the same revision. This conservative rule is appropriate for a handler that consumed a complete event input; it must not silently redefine ordinary partial-query authorization.

## Minimal coherent shared delta

```text
GraphInfluence.snapshots: Vec<GraphRef>      // whole-value AND, survives empty output
Node.derived_snapshots: Vec<GraphRef>        // record-level global AND
Edge.derived_snapshots: Vec<GraphRef>        // global AND, outside OR derivations
Assertion.derived_snapshots: Vec<GraphRef>   // explicit persisted equivalent
```

All fields default empty, omit when empty and use bounded strict deserialization. Existing serialized descriptors/revisions remain byte-identical when the fields are absent. Validate graph/revision IDs against the existing 512-byte bounds. Canonicalize exact `(graph_id, revision)` pairs; never merge different revisions of the same graph. The combined assertion+node+snapshot count in one influence/record gate is at most 1,000, not three independent quotas. Precheck raw collection counts before deduplication and charge serialized bytes before retaining clones.

The per-record additions are necessary. Current `protect_generated_result` copies whole assertion/node influence into generated scalar nodes and edges. Without an equivalent snapshot field, clearing only GraphData.influence while saving an isolated scalar would erase the new empty-source restriction. Do not encode a snapshot as a fake assertion or unrelated node. Durable auxiliary guard objects could emulate it only by adding extra stored objects, protected namespaces, lifecycle and writes; that is larger and less direct than these restriction fields.

Keep derivation alternatives structurally unchanged. Snapshot restrictions in this first profile are global AND gates; no `Derivation.snapshot_premises` or alternative-specific snapshot policy is required. A conclusion with one authorized OR proof still cannot bypass its global snapshot gate. A future less conservative alternative-scoped design would require separate review and cannot reinterpret this encoding.

## Shared native authorization context

Do **not** implement snapshot checking by recursively calling current `authorized()` unchanged. `authorized_nodes()` and `authorize_edge_groups()` create fresh visiting sets/budgets, so recursive snapshot gates would reset limits and could loop through mixed node/assertion/whole-graph paths.

Introduce one explicit authorization context passed through snapshot, node, assertion, typed-context, group and endpoint checks:

```text
AuthContext {
  active: set<AssertionKey | NodeKey | ContextDefinitionKey | SnapshotKey>,
  remaining_work,
  depth,
  // Uses the existing operation-scoped cumulative SQL/read/materialization budget.
}
```

SnapshotKey is a distinct namespace containing exact graph+revision; it cannot collide with node/assertion ID strings. Existing keys use kind tags 0/1/2; a dedicated enum or new distinct tag is required. Snapshot checks share the same current host/principal, established database snapshot and trusted clock. No memoized success survives an operation, actor or policy change. An optional per-operation successful-check cache must never mark an active check successful before all descendants complete.

For each snapshot gate:

1. Validate IDs; charge work before loading. Reject depth over32, exhausted work/read bounds and already-active exact SnapshotKey. Push an unwind-safe active guard.
2. Apply `protected_reference_allowed` for the exact revision under current identity/governance policy. Integrate its reentrant protected-read traversal with the same outer limits; its existing separate guard is useful but not a substitute for shared mixed-path accounting.
3. Load and establish integrity of the exact immutable revision. Missing or invalid content never becomes authorized empty data. Preserve logical revision/manifest-content binding and ordinary digest integrity; reuse a verified load invariant only if its enforcement is demonstrated.
4. Authorize the entire original GraphData with that same context: primitive readers/endpoints, typed-context and graph influence, all node proof gates, all edge/assertion groups, attachment origin/host restrictions. Use the reviewed whole_graph_visible comparator: only flat derived_from order/dedup may normalize; object/group/metadata/snapshot loss or any incomplete flag denies.
5. Pop the active key on success, error and unwind. Return true only for a complete authorized snapshot.

Ordinary metadata pointers are data, not automatic whole-snapshot influence. Do not recurse through every metadata value solely because it exists in the referenced graph; real cyclic metadata must remain valid. Authorization follows actual context/proof/influence dependencies. A handler that reads metadata targets injects those target revisions separately, so their current authorization remains a gate. Required-metadata/export completeness keeps its own explicit checks.

A gate on self, two snapshots gating each other, or a mixed Snapshot→Node→Assertion→Snapshot cycle fails closed. A logical same-batch reference to another acyclic input can be valid; logical names remove hash fixed-point problems but do not authorize circular proofs. Retrying an old pinned revision after head advance checks that exact old revision, not the new head. Current source policy revocation still denies it.

For query output, denied/missing influence returns the existing generic unavailable/partial behavior without exposing the denied pin or graph contents. Whole export/dispatch remains a generic denial. Resource exhaustion remains explicit where already promised; never report complete evaluation after a budget fallback.

## Complete propagation without changing original payload identity

`influence::validate`, canonicalize, merge, snapshots and budget accounting must include the new whole-value references. A new bounded collector should gather **only declared snapshot gates** from GraphInfluence and per-record derived_snapshots. It must not promote every descriptive QueryResult.input_snapshots pin into a whole-visibility requirement: that would incorrectly deny existing authorized partial queries.

At graph-operator boundaries, conservatively merge the declared snapshot gates of consumed inputs into the output whole-value carrier, including gates on records filtered away. This prevents filter/project/context selection followed by unknown/support or an empty result from losing a restriction. Retained original records keep their payload/record identities; adding a whole-value envelope does not falsely relabel them. Generative operations additionally copy the resulting whole snapshot gates into every generated node/edge via protect_generated_result, with precharged bytes and per-record combined limits. An empty result retains the carrier even though there are no records to copy into.

| Path | Required implementation |
|---|---|
| Algebra envelope, union/diff/project/support | Union consumed input gate sets; preserve on zero selected records; protect generated status/diff records; snapshot-gate fields participate in synthetic record keys and same-origin payload conflict checks. |
| Filter/context/counterparts | Preserve conservative consumed gates through removed nodes/edges and empty selections; do not turn default context into universal scope. |
| Join/rules/geometry/explain | Propagate direct declared snapshot gates as well as existing leaf proofs. Global snapshot gates remain AND across alternative groups. Explanation must expose real snapshot dependencies as snapshot records, not AssertionRefs, and protect those explanation records. |
| Typed context and metadata navigation | Preserve source/path gates and separately resolved target gates. No private path loses its restriction on returning an independently stored target. |
| Explicit assertion materialization | Copy Assertion.derived_snapshots to its materialized Edge and preserve through repersistence; structural relationship alone is not a claim. |
| Native query / node/assertion proof resolution | Enforce record gates even when its GraphData envelope was cleared; endpoint node gates also guard assertion/attachment proxy access. |
| Native identity/governance/clustering/view services | Constructors default empty where appropriate; generated records retain declared gates; cached result and current-policy validators traverse them. |
| Result identity and source attribution | Serialized snapshot fields affect result/record identities. QueryResult.input_snapshots includes their pins for inspection; provenance AssertionRef vectors stay honest. Function/rule/source manifests remain unchanged. |

Generated graph-host metadata has no per-record snapshot field in this proposal. The current generated metadata forms must either be hosted by a protected node/edge or carry an actual origin reference to a stored protected object; whole envelopes protect complete graph values. Before freeze, audit each generated attachment constructor and reject unsupported independently movable generated graph-host payloads rather than silently claiming record-level closure. No additional StructuralEdge gate is proposed: independent structural descriptions are not derived assertion evidence, and existing endpoint/context behavior must remain explicit.

The trusted handler preparation pipeline injects the exact event revision plus every actually consumed resolved metadata snapshot into its input whole-value carrier after complete authorization. Pure recipe output therefore retains those pins even for an empty source or empty result. It does not require an input node, and cannot manufacture permission by naming an accessible but unrelated revision. Recorded event/config identity still belongs in the preparation receipt; a graph snapshot gate is an access condition, not an event signature.

## Admission, export, capsules and persistence

Extend central `refs(data)`/`influence::snapshots` and every semantic dependency walker with both whole and record snapshot fields. Admission Read/Traverse checks must verify exact pin reachability under an allowed accepted branch, including empty and node-only inputs. Signed export requires complete authorized scope of these pins and their ancestry/manifest dependencies; its cached original-closure replay must recheck current source guards. Query Result metadata alone is not a substitute for this transport closure.

Native capsule export includes the referenced revisions; denied/missing pins prevent whole publication or remain explicit unavailable boundaries according to the particular existing local profile. Signed whole export remains no-external/fail-closed. Receiving bytes validates new fields and immutable digests without granting authority or advancing heads; later source proofs and current policy still decide reads. Reserved policy/decision registries remain host-owned and cannot be installed as authority by a capsule.

This is a shared semantic change, unlike the preceding formatter or signed-export endpoint. Reserve a new protocol version with the engine after export freeze. Recursively reject nonempty new fields under every older Program profile **before any write**, including later commands in CommitBatch/program sequences. Unknown fields must never be ignored. Update portable crate/package/version/CLI docs and exact language vendoring only at a coherent native freeze.

Persistence now contains authorization-bearing fields unknown to old readers. Recommend the next explicit store marker with atomic migration and an actual previous-binary refusal test; do not rely on accidental deserialize failure in selected paths. Old revision/schema/capsule bytes remain unchanged when the new vectors are empty. Review a new capsule profile/feature marker for capsules containing nonempty snapshot gates, while preserving old capsule output for old data. The signed response's contract/capsule versions must advertise supported semantics; receiver version negotiation cannot accept fields its proof engine ignores. No concrete version numbers are reserved by this document.

## Acceptance required before reactors

1. A truly empty source revision gates an empty result and a generated scalar. The result becomes unavailable when the exact source disappears/is denied or its protected policy expires; another public empty graph cannot substitute.
2. Envelope-stripped, readers-cleared saved scalar node and saved edge/explicit assertion remain gated through derived_snapshots. An assertion/attachment whose endpoint node is gated cannot proxy around it.
3. A partial-visible source with one withheld alternative fails whole-snapshot authorization even if another proof is visible. Harmless flat-index permutations preserve immutable bytes and authorize when complete.
4. Empty union, projection, context/filter, support unknown, rules, geometry and graph-valued explanation retain declared gates; synthetic identities are stable on repeated/nested union but differ for different exact snapshot gates.
5. Metadata cycles without influence cycles still work. Self, mutual and mixed snapshot/node/assertion/typed-context influence cycles terminate fail-closed. Duplicate references do not amplify work; depth/work/read limits do not reset through any nested path.
6. Raw snapshot refs only restrict: nonexistent refs do not make empty data public; forged unrelated refs grant nothing; graph/branch/policy identity never comes from caller authority fields.
7. Original revision pin remains fixed across head changes, and current revocation affects historical/cached/prepared reads. Context scope and source schema meanings remain intact.
8. Capsule/receive/re-export retains exact fields and bytes; required scope omissions for snapshot-only dependencies deny query/export/replay, including ancestors and logical siblings. Receive alone changes no accepted head or policy.
9. Old wire preflight rolls back earlier writes; store migration/death recovery and old-binary refusal pass; old empty-field serialization/hashes stay exact.
10. An actual compiler/native handler can subsequently consume a whole empty event input and persist its dependent empty/scalar output without fabricated assertions. This is a handler acceptance follow-up, not implemented by the carrier alone.

## Sufficiency and remaining boundaries

The coherent whole-value plus per-record delta is sufficient to **name and enforce current whole-snapshot access** for empty and nonempty handler inputs, provided the shared recursion, propagation, integrity and transport checks above are implemented. It does not authorize installation, authenticate an event, promise latest projection freshness, implement release policy, distinguish every negative/unknown coverage class or provide selective replication. Runtime budgets may conservatively deny large/deep closures; they must never erase gates to finish a computation. There is no demonstrated existing generic carrier that avoids this change without extra persistent guard objects.
