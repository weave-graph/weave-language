# Weave protocol v0.2.0 integration notes

The authoritative types live in the engine's `crates/weave-contract`. This repository vendors an exact copy with hashes in `vendor/manifest.json`. Run `python3 scripts/verify_contract.py` to validate the snapshot.

This historical compiler stage emitted version `0.2.0`; the current compiler emits [v0.3.0](../v0.3/README.md). The engine continues to accept `0.1.0` commit/query plans but must reject joins declared under that old version. Foundation commit/query semantics are described in [v0.1](../v0.1/README.md).

## Path join

`Command::Join` accepts `left` and `right` query plans, an `output_predicate`, and `match_on: "entity_space_to_from"`. It joins left-edge target to right-edge source by **exact stable entity identity and space identity**, not graph-local node spelling, labels, similarity, or an inferred mapping. This is the only implemented join policy.

The engine independently authorizes both input queries. For two positive premises with compatible identity keys and overlapping valid intervals, it emits a derived edge from the left source to the right target. The interval is their intersection, with an absent end representing infinity. Empty intersections emit no conclusion. Result identities are deterministic and source-namespaced. Provenance records both exact premise graph/revision/assertion references. Negative premises do not derive a positive path.

The join is a pure graph result, not a commit. It carries coverage and diagnostics from its inputs, restrictions, and `input_snapshots: Vec<GraphRef>` to preserve multiple revisions of the same graph without key collisions. `snapshots` remains the legacy primary lookup; replay must use the complete vector and metadata references. No global atomic observation is implied by resolving remote inputs separately.

At the v0.2 stage, language join output was terminal. Protocol v0.3 adds reusable program-local results; consult its notes for current behavior. General arbitrary joins remain planned.
