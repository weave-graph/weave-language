# Schema-constrained pure graph functions

This source-only extension uses the graph schema contract introduced before protocol 0.15; the current compiler emits 0.15. It implements a bounded part of paper §§4–5: the compiler checks visible exact schema compatibility, while the runtime retains its independent validation, identity, visibility, context and budget checks.

```weave
schema Fleet revision "1" {
  node Device space "operations" {}
  edge Connected from Device to Device {}
}
function Select revision "1" (graph input schema Fleet, time instant)
  returns graph schema Fleet {
  lens Result from input { match relation "connected"; at param instant; }
  return Result;
}
```

Only graph parameters accept `schema Name`. A return constraint is optional and follows the parameter list as `returns graph schema Name`. Constraints refer to earlier schema declarations. Existing unconstrained graph/string/time/function parameters keep their previous behavior.

A match requires the complete `GraphSchema` descriptor: nominal ID and revision, every node/edge type, properties and scalar/unit types, optional/required/nullability rules, space constraints and openness flags. A matching label or structurally similar layout is insufficient. There is no implicit subtyping, migration, schema cast or field erasure. Duplicate schema declarations are rejected rather than changing an existing label's meaning.

## What can be proven

A source graph declaration provides an exact known descriptor (or known untyped graph). Filters, parameter binding, projection, exact context selection, typed-context selection and declared counterpart selection preserve their input descriptor. A constrained function's formal graph inputs supply those exact assumptions during definition checking; its callers must prove the assumptions. Return annotations must be justified even for an unused definition, then checked again after specialization.

`use`, a live handle's `pin`, metadata navigation and native service results are dynamically described; this profile does not invent a runtime assertion for them. Union, diff, joins, rules, support, explanations and geometry may synthesize or modify descriptors and are conservatively unknown to this checker. Their runtime graphs still retain their actual schemas. An exact constraint on one of these values produces `E_SCHEMA_UNKNOWN` until a suitable static derivation or explicit runtime schema-check operation exists. A proven different descriptor or untyped input produces `E_SCHEMA_MISMATCH`.

Unknown does not mean untyped. An annotation cannot turn uncertainty into a proof. The current runtime graph schema also does not encode the paper's complete metadata shapes or completeness requirements; this source feature makes no guarantee about those future graph-type components.

## Partial and higher-order functions

Partial application validates any supplied constrained graph immediately and keeps obligations on the remaining parameters. Captured function definitions retain their descriptors and return constraints. Passing a constrained unary graph function through an unconstrained higher-order parameter is allowed; concrete specialization checks the callee's requirements when it is invoked. It cannot bypass a schema requirement by hiding the function in a closure.

Generic function parameters do not yet carry a separate schema-polymorphic function signature. Therefore a definition cannot promise an exact output schema merely because an arbitrary function parameter is called; an unknown output remains unknown. Existing unconstrained higher-order functions can still specialize to concrete constrained functions, as demonstrated by the [example](../examples/schema_functions.weave).

The generated operation commands are identical with or without equivalent static annotations. Constraints introduce no read, write, host authority or schema-stripping operation. The source revision fingerprint of a constrained definition includes the resolved complete descriptors, so changing a referenced schema changes that definition's identity. Source spans, comments and whitespace remain nonsemantic; user schema/property fields remain semantic.

## Evidence and limits

[Compiler tests](../tests/conformance.rs) exercise exact descriptors, mismatches, unknown live/remote reads, return checking, projection/context preservation, descriptor-changing algebra, partial/higher-order/captured calls, label conflicts and precise argument/return/schema diagnostics after earlier comments. [The actual runtime script](../scripts/check_schema_functions.py) compares annotated and unconstrained command plans and results, verifies direct/captured/higher-order schema and provenance equality, and persists an empty result without losing its descriptor.

Dynamic runtime schema assertions, general structural/subtype compatibility, richer value/function signatures, reusable modules and migrations remain open. No shared contract or runtime changes are required for this static slice.
