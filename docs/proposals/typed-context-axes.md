# Typed context axes: proposed bounded next stage

Historical design proposal, now implemented as the bounded protocol 0.14 profile. See [the current contract and source guide](../TYPED_CONTEXTS.md); later sections retain proposal history rather than override that guide. This document separates a portable type-validation stage from the runtime evidence boundary it needs. Neither alone completes L18.

## Source requirements and design choices

The recovered language paper §1.1 distinguishes representation spaces, branches and contextual labels. Sections 3.2 and 4.2 require attributable metadata and graph results that preserve assumptions, provenance and boundaries. Section 5.1 defines support relative to a selected proposition, context and evidence policy; §6 allows spaces to declare applicable context. Engine §5 and §10 require snapshot/policy dependencies in results and restrictions on actual access paths.

The papers do not prescribe an axis wire format, value lattice or compatibility algorithm. The following exact nominal model is a proposed implementation choice. It must not be presented as a verbatim paper grammar. Tenant, jurisdiction, scenario or observation mode can be application-defined axes; none grants authority, defines a representation space or establishes a real-world fact.

Current Default/Pinned selection remains valid. Default is one exact context, not a wildcard. An omitted axis is not universal applicability. Two independently authored descriptors with equal values do not become the same context: exact graph/revision identity is preserved until a separate explicit compatibility mapping is implemented.

## Portable profile A: typed total assignments

A new independent portable `context_axes.rs` owns these provisional types and pure validators:

```rust
struct ContextSchemaRef { id: String, revision: String }
struct ContextSchema {
    reference: ContextSchemaRef,
    axes: BTreeMap<String, ContextAxisType>,
}
enum ContextAxisType {
    Boolean, Integer, String, Decimal,
    Quantity(UnitDescriptor),
    Enum { members: Vec<String> },
}
struct ContextDefinition {
    schema: ContextSchema,
    values: BTreeMap<String, serde_json::Value>,
}
```

The module must reject unknown fields. Boolean, Integer and String use their strict JSON types. Decimal is the existing exact canonical string; Quantity is the existing exact amount/nominal unit object. Enumeration values are exact strings, with no case folding or Unicode normalization. Integer values are signed i64. Float, null, arbitrary nested values and implicit scalar conversions are unsupported in this bounded profile. Object-reference axes remain a later extension because they require exact dependency and authority treatment, not string coercion.

An assignment is total over its schema: every declared axis occurs once and no extra axis occurs. There are initially no optional axes, missing-value semantics, defaults, inequalities, subtyping, interval-valued axes or wildcards. This makes the first profile finite to validate without introducing implicit broadcasts. Equal quantity unit spellings with different revisions remain incompatible.

Proposed limits: 1–32 axes, 1–128 distinct enum members per axis, identifiers/nonempty enum members at most 512 UTF-8 bytes without controls, String values at most 4096 UTF-8 bytes, and a complete descriptor at most 64 KiB canonical JSON. Decimal bounds stay 18 digits/scale 18. Count/byte limits must be checked before cloning or walking an oversized collection. Schema labels bind to complete descriptors; reusing an ID/revision with changed axis kinds or enum members is an explicit conflict, not an update.

Canonical encoding uses ordered maps and sorted enumeration members. Reordering declarations does not change a descriptor fingerprint; changing an axis name, nominal unit revision, enum membership or value does. A fingerprint is content identity, not authorization or semantic equivalence across distinct context pins.

## Source proposal

```weave
context_schema OperatingWorld revision "1" {
  axis "jurisdiction" enum ["EE", "FI"];
  axis "scenario" string;
  axis "simulated" boolean;
  axis "load" decimal;
}

context_value EstoniaLive schema OperatingWorld {
  axis "jurisdiction" "EE";
  axis "scenario" "baseline";
  axis "simulated" false;
  axis "load" decimal "0.75";
}

// Proposed explicit checked read, separate from ordinary exact-pin selection:
typed_context Selected from Claims
  graph "EstoniaLive" revision "exact-snapshot-revision"
  schema OperatingWorld;
```

`context_schema` is an immutable compile-time declaration. `context_value` proposes one ordinary context snapshot, requiring the same explicit host write grant as a graph declaration; it can participate in an explicitly named local transaction to permit known logical pins. Neither declaration installs permission or acceptance policy. Static diagnostics identify the actual duplicate/invalid axis token, including after Unicode strings or comments.

`typed_context` must be an explicit read whose context descriptor reference appears in the plan. A pure function can consume an already selected graph value; it cannot hide a descriptor read under an ordinary scalar parameter. A future effect-typed function can declare that dependency explicitly. General graph/function type parameters remain independent work.

## Runtime profile B: descriptor assertion and scope evidence

Recommended storage reuses explicit graph structure and attributed claims rather than adding unguarded graph-level metadata. One context snapshot contains a canonical descriptor assertion (fixed ID `definition`) on a context anchor. Its `assertion_properties["weave.context"]` contains the validated `ContextDefinition`. The descriptor assertion and anchor retain ordinary readers, assertion/node dependencies, source attribution and capsule closure. The intrinsic validates the exact descriptor schema and canonical shape; a user property with this name is merely data until the explicit typed-context operation interprets it.

Descriptor interpretation does not endorse the underlying context or its claims. Authored axes cannot mint tenant access, policy authority or permission to cross worlds. A negative or conflicted descriptor is not silently selected as a unique accepted meaning. The first profile should require exactly the positive canonical definition record and reject unsupported alternatives; later governed interpretation remains explicit.

Proposed versioned delta, subject to engine-owner review:

1. Add a typed-context expression carrying input, exact context GraphRef and expected complete nominal schema (or a validated schema descriptor reference resolved under host authority). Existing Default/Pinned wire bytes stay unchanged.
2. Runtime resolves the descriptor once under the current principal, validates shape/assignment and expected schema, then filters claims using the existing exact context pin. No matching by equal axis values or application labels.
3. Carry a distinct context-type witness in the result: exact context pin, validated schema fingerprint, descriptor AssertionRef and any required anchor NodeRefs. The witness is runtime-derived; raw source/plans cannot provide an authoritative result envelope.
4. Charge descriptor loading and witness retention to existing read/materialization budgets. Missing, denied, malformed or unresolved descriptors fail with a generic typed-context-unavailable diagnostic; do not return a complete empty graph or reveal hidden descriptor counts.
5. Empty selected results retain their type/scope witness. Support of an absent proposition in a typed context remains scoped and depends on the descriptor; it is not a public default-world fact.

The exact witness field/API must be reviewed before wiring. A bare schema label or snapshot vector is insufficient: it records a name but does not enforce visibility when a derived value is copied or persisted. Existing `QueryResult.provenance` must not be silently repurposed as an authorization guarantee without auditing all consumers.

Every derived scoped node, including support/unknown and explanation nodes, needs the descriptor's assertion/node influence gates. Derived assertion groups must retain the descriptor premise for the relevant alternative. Projection and union preserve those per-record dependencies; a mixed union may clear a selected-scope envelope but must not erase record-level qualifiers or descriptor gates. Diff requires compatible exact scopes. Joins and rules must propagate witnesses for the exact selected context and reject conflicting schema identities. Metadata traversal cannot transfer one typed context onto independently qualified metadata claims. Geometry, accepted identity and cluster services must validate any typed context before consuming it; legacy exact pins remain an explicitly separate, untyped profile.

Serialization, repersistence, capsules, signed-admission scope checks, cache keys/current revocation checks and explanation identities must include these new dependencies. Reserved descriptor identity alone is not trusted evidence. A trusted writer can author a descriptor, but cannot remove source restrictions from a derived typed conclusion merely by clearing its readers.

## Module ownership and integration order

| Stage | Language-owned work | Engine-owned work | Acceptance gate |
|---|---|---|---|
| A | New isolated portable `context_axes.rs` and tests; source AST/declaration validation proposal | Review exact serialization/limits and schema-label conflict policy | Pure validation/canonicalization tests; no runtime-conformance claim |
| B | Parser/lowering and fixtures after agreed DTOs | Shared lib/version wiring; descriptor lookup/witness types; proof, query, persistence, capsule and admission closure | Compiler→runtime typed-context selection, private descriptor propagation and atomic rejection |
| C | Function signatures/axis access and explicit comparison proposals | Reviewed compatibility/broadcast/acceptance service | Explicit mapping authority and no implicit world crossing |

No current shared file is owned by this proposal until engine/root assign it. Canonical `lib.rs`, protocol version and runtime wiring remain engine-owned. Language can independently implement the new pure module in an isolated worktree once profile A is approved. Vendor only after a paired freeze.

## Required tests before claiming profile B

- Valid scalar/enum/Decimal/Quantity assignments; missing/extra/duplicate fields, wrong nominal units, Float fallback, oversized descriptors and noncanonical exact values rejected.
- Canonical ordering stable; descriptor mutations under one schema label conflict; distinct equal-valued context pins remain distinct.
- Default, untyped pinned and typed pinned profiles cannot silently broadcast or erase qualifiers; selected empty output retains context type.
- Two claims in different jurisdictions/scenarios never produce a cross-world join or support conflict without explicit compatibility.
- Private descriptor with otherwise public source claims cannot leak axes through support/unknown nodes, explanation, projection, union, copied saved output or capsule receive; current proof revocation is rechecked.
- Schema/descriptor/current-policy changes affect view/result identity, while old exact snapshots remain historically pinned.
- Descriptor denial/missing/malformed cases do not reveal hidden existence; no positive conclusion from a negative definition or unsupported ambiguous definition.
- Contextual metadata paths preserve witness restrictions, and typed geometry/identity/clustering results remain ordinary reusable graph values.
- Unsupported old protocol rejects before any writes; later failures roll back graph/event/schema registries.
- Actual source→native CLI fixture and independent root acceptance; portable module WASM check, with no browser/mobile runtime claim inferred from compilation.

## Still mandatory beyond this slice

Explicit compatibility mappings, broadcast declarations, governed world crossing, hypothetical assumptions/branch masks, object-reference axes, richer function signatures, complete-scope absence and aggregation remain open. Typed labels alone do not satisfy those requirements. They also do not replace representation spaces or solve offline acceptance coordination.

## Profile A implementation checkpoint

The isolated language library now exports `context_axes` with the types above, bounded `ContextSchema::from_json` / `ContextDefinition::from_json`, validation, canonical bytes, SHA-256 fingerprints and a bounded local `ContextSchemas` conflict registry. The actual Quantity variant is `Quantity { unit: UnitDescriptor }`, serialized with `kind: "quantity"` and a `unit` descriptor. Other variants use `kind` names `boolean`, `integer`, `string`, `decimal` and `enum` (with `members`). At the initial profile A checkpoint the public map-bearing structs did not implement generic Serde `Deserialize`. Protocol 0.14 adds custom strict deserialization that preserves duplicate-key rejection and a separate cumulative 64 KiB budget even when embedded inside a larger plan. Constructed Rust values must pass `validate` or a canonicalization/fingerprint operation before use.

The decoder rejects duplicate keys at every object depth, including escaped aliases of the same key, rejects unknown fields and bounds raw input before parsing. Canonicalization validates complete sizes/types before cloning. Schema registration is an in-memory local declaration/composition aid; arbitrary graph property storage or typed-context reads do not install a global schema registry. Equal values/fingerprints do not equate separate context GraphRefs, which remain outside this pure module.

Eight focused tests cover exact total assignment, nominal quantities, forbidden conversions, duplicates/unknown fields, ordering, schema conflicts without replacement, size/depth/count limits, malformed inputs and finite registry capacity. The full isolated language suite has 76 passing tests; strict Clippy, formatting and the WASM library target pass. That historical profile A checkpoint preceded the implemented 0.14 grammar, descriptor lookup, persisted witness carrier and wire integration.

## Profile B DTO review constraints

The proposed expression should carry `input`, exact context `GraphRef` and the complete expected canonical `ContextSchema`. Comparing only a caller-supplied schema hash or label is insufficient for type validation. Runtime resolves the descriptor and compares full canonical descriptors; reads do not install global schema identities. Same-label conflicts are rejected within the relevant program/composition environment.

A witness proposal must specify both its materialized-result carrier and its persisted graph carrier. A `QueryResult`-only field disappears when the graph is saved, so it does not establish semantic closure. Each scoped record needs an unambiguous association to its exact context and expected descriptor identity; an empty selected result also needs a scope witness so subsequent `support`/`explain` can retain descriptor restrictions. Witnesses cannot be serialized into ordinary user properties and then treated as trusted origins.

Suggested conceptual DTO (field placement remains engine review):

```rust
struct TypedContextWitness {
    context: GraphRef,
    schema: ContextSchema,
    definition: AssertionRef,
    anchor_nodes: Vec<NodeRef>,
}
```

Runtime-derived witnesses use the exact descriptor assertion and its actual anchor nodes, never caller-supplied proof envelopes. Full schema retention allows downstream type interpretation; canonical fingerprint is computable and need not be a second independently trusted field. Either a bounded graph-level witness catalog keyed by exact context or explicit per-record references can avoid repeated full-schema copies, but must preserve privacy when mixed-world union/projection removes records. Limits must charge witness bytes and object counts before cloning, with at most one compatible schema per exact selected context. Old protocol variants must reject these fields before writes.

Before finalizing the DTO, audit every constructor/consumer: query, union/diff/project/filter, join/rules/support, metadata paths, geometry/explain, identity/clustering, save/read/capsule, view cache/revocation, signed admission and source fingerprints. Missing witness dependencies must fail closed, not degrade a typed result to an untyped/default claim. The next shared version remains unassigned until this boundary is approved.
