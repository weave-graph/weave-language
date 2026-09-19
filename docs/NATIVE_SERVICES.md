# Pinned native graph services

Protocol 0.13 exposes the engine's existing accepted identity resolver and cluster navigation as ordinary reusable graph results. The selectors carry explicit immutable source pins, an explicit default or pinned context and an integer evaluation instant. They carry no policy installation or acceptance authority.

`resolve_identity` selects an existing accepted mapping decision with its exact policy revision and source NodeRef. The runtime preserves independently assigned entity IDs, source manifestations and decision/source restrictions. Current policy revocation and source authorization are checked even for historical decisions. The returned wrappers are typed graph values; they can feed ordinary graph functions, union, projection and explanations. Missing or unavailable identity evidence has the native disclosure-safe behavior; a source string does not certify a mapping.

`cluster_navigation` selects one stored GraphRef, a relation, instant and bounded level count. It returns navigation nodes and relationships with exact source lineage, including isolated visible source nodes. Coverage is deliberately partial and scoped to the host-authorized input. No cluster output authorizes pruning an exact source query, and no absence of a visible member establishes global absence. Navigation is untyped in this profile, so a direct union with typed identity wrappers correctly requires explicit schema-compatible treatment.

Both source forms are top-level reads. Their results can be captured or supplied as immutable graph arguments to pure functions. A function body cannot hide either service read. The initial profile has no computed graph operand for clustering; an arbitrary intermediate graph cannot be substituted for the stored source selector.

## Reproduce the paired acceptance

Build the engine CLI and its explicitly trusted administrator fixture:

```sh
# In a compatible weave-engine checkout:
cargo build --locked -p weave-engine --bin weave-engine --example native_services_fixture

# In weave-language:
python3 scripts/check_services.py \
  --engine /path/to/weave-engine/target/debug/weave-engine \
  --fixture /path/to/weave-engine/target/debug/examples/native_services_fixture
```

The fixture installs a test policy, creates two independent source entities, submits and accepts their mapping using native host APIs, then revokes it for a negative case. It is test setup with trusted administrative authority, not a public peer API or source-language capability. The script substitutes real source/decision revisions into `examples/accepted_identity.weave`. `examples/cluster_navigation.weave` uses a logical transaction pin and needs only the engine host's explicit write grant for its source graph.

Acceptance checks native/source graph equality, preserved independent IDs, immutable function and nested-union reuse, no service-generated events, current revocation, whole-program rollback, isolated-node navigation and recursive old-protocol rejection before writes. Engine native tests separately exercise clocked views, current source restrictions and native request validation.

## Remaining scope

This stage does not complete the full identity or multiscale design. Source-level proposal/acceptance effects, richer ambiguity and private pairwise identity policies, computed-input clustering, automatic overlapping hierarchies, semantic zoom policy and certified exact-query pruning remain separate gates. Trusted host policy governs service execution; selectors and local provenance records do not authenticate a remote sender.
