# Weave contract 0.1.0

Historical protocol notes. The current compiler emits [v0.3.0](../v0.3/README.md); these rules describe plans explicitly declaring v0.1.0.

The original engine source path is `crates/weave-contract/src/lib.rs`; the current vendored types are `vendor/weave-contract/src/lib.rs`. The crate contains only portable serde types, no I/O. All plan inputs reject unknown fields. Top-level `Program.version` must exactly equal `0.1.0`.

`commit` replaces one graph branch's entire snapshot, records an immutable revision and creates an event in one transaction. `expected_head` is optimistic concurrency: null requires a new branch; a string must equal its current revision. Valid times are signed integer milliseconds and half-open `[start,end)`; null end is unbounded. System time is host assigned and revisions are pinned by ID. Root revisions preserve prior snapshots. Empty node/edge readers means public; otherwise host principal must appear. Host authority is never provided by the plan. IDs must be nonempty strings.

`query` selects a graph and pinned revision or resolves the branch head at query start. Optional edge filters are conjunctive. Result nodes are all visible nodes when there is no edge filter, otherwise visible endpoints of selected edges. An edge is visible only when its own policy and both endpoints permit it. Metadata traversal is pinned, bounded and cycle-safe; unresolved references return generic partial diagnostics without distinguishing nonexistent from denied objects. References on inaccessible objects are not returned.

Graph identifiers, revisions, branches, entities and manifestations have separate fields. Metadata graphs are referenced from nodes AND edges. Derived assertions retain explicit references; a reference is evidence lineage rather than a claim of truth. A graph query includes source assertion references as provenance. This initial contract is an explicitly limited executable checkpoint: n-ary relations, general joins, leases/signed capabilities, distributed acceptance, geometry, clustering and effectful adapters remain planned.

Canonical revision bytes use Rust serde serialization of the typed graph with BTreeMap properties and caller-ordered node/edge lists, domain-separated SHA-256 plus graph, branch and parent context. Array order is significant in this initial version. A future cross-language canonicalization specification is required before external implementations produce revision IDs.
