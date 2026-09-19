# Protocol 0.15 pairing and metadata selectors

The compiler now emits protocol `0.15.0`, using the exact 24-file contract distribution from engine commit `506729278cf3ee42a30eb9ac71336480e386fe89` (including its MIT license). The manifest records every byte hash and the versioned contract documentation. Pure modules, exact schema constraints, live handles and all existing source operators remain available.

The runtime retains whole-graph assertion/node influences, including when a result is empty. Per-derivation node premises preserve alternative proofs; global node influences remain additional conjunctions. Generated records retain their constraints when copied or saved. References constrain visibility; they do not grant authority. Source bootstrap declarations emit no influence/proof-authority fields, and ordinary properties named `influence`, `derived_nodes` or `node_premises` remain data. The source language does not add governance acceptance, release or permission-installation syntax in this pairing.

## Current member IDs and original metadata selectors

A direct target query retains original local node IDs. Navigating through metadata creates a distinct path-qualified node wrapper because its readers and dependencies differ from the original record. Its `id` is a deterministic `metadata-node:...` value; immutable `node_origins` is empty, while exact original NodeRefs remain dependency gates. Entity and space identity remain unchanged. They are not aliases for a current local node ID.

A metadata node-host selector accepts the returned current ID. Existing nested source such as `metadata Next from Evidence on node "b" key "proof";` also retains a narrow compatibility shorthand: the runtime must find one exact current target snapshot, its already-authorized original node-host attachment, the same unchanged current attachment ID, and exact attachment provenance. Missing or ambiguous evidence returns the existing generic partial result. Arbitrary proof references cannot create aliases. Saved wrappers and union-remapped attachment IDs do not gain this shorthand. Graph/edge/assertion/entity metadata hosts retain their existing semantics.

Projection always names current result IDs. After metadata wrapping, `project P from Evidence { node "b"; }` therefore reports `E_PROJECT_MEMBER` if `b` is not a current member; the whole program rolls back. It does not silently return an empty graph. Projection by an available edge ID retains its current wrapper endpoints. Hosts can inspect returned node IDs before projecting. The source language currently has no general dynamic binding for returned node IDs, so a source function that hardcodes original node IDs after metadata must change its selection strategy. No general proof-to-source-identity alias is implied.

## Validation and remaining scope

The actual compiler/runtime workflow suites cover modules, exact schemas, live and fixed metadata bindings, current visibility, temporal filtering, rule closure, geometry, contexts, scalar precision and native graph services. Independent metadata suites compare both runtime versions, exercise every metadata-host kind, restore nested original-node source selection, verify distinct/repeated wrappers, and test reader-stripped node persistence plus projection failure rollback.

This is a coherent experimental pairing, not full paper conformance. General permissions/effects, governed accepted graph results, broader context compatibility, source-addressable object selectors, richer value types, transport and the remaining [acceptance gaps](ACCEPTANCE_GAPS.md) stay open. Persisting runtime results uses the result's reported version; older input profiles cannot carry the new authorization-bearing fields.
