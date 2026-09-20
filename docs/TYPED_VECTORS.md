# Typed physical and embedding source vectors

Ordinary vector values retain their complete native Space descriptor and role through
pure function parameters, results, partial application and unary callbacks. Values
lower to the existing `weave.geometry` coordinate payload; protocol 0.18 and runtime
schema types are unchanged. A source descriptor is data, never authorized Evidence.

```weave
vector_type Position3 space {
  "id":"physical", "revision":"1",
  "geometry":{"kind":"physical3d","frame":"building-A","unit":"metre"}
} role position;
function Keep revision "1" (vector input type Position3) returns vector type Position3 {
  return param input;
}
apply Origin from Keep { vector input vector Position3 [0.0,0.0,0.0]; }
value Same vector_equal(value Origin, vector Position3 [0,0,0]);
```

`vector_type` is a top-level pure declaration, available in explicitly pinned modules.
It is resolved before static checking and identity hashing. Types compare complete
space ID/revision/geometry plus role. Source declaration aliases do not change the
resolved type or normalized function identity; raw module pins still bind exact bytes.
Functions cannot capture a descriptor by an undeclared module alias.

Physical descriptors use `kind: physical3d`, a nonempty `frame`, and exactly one
`unit`: `metre`, `centimetre`, `millimetre`. Components have dimension three; role
is `position` or `direction`. Embedding descriptors use `kind: embedding`, nonempty
`encoder` and `preprocessing`, explicit `dimensions` from 1 through 4096 and `metric`
`euclidean` or `cosine_distance`; role must be `embedding`.

Every descriptor field matters. Identical dimensions with different encoders, units,
frames, revisions or roles are incompatible. Within one linked declaration
environment, the same space ID/revision with different geometry is an error. This
check installs no global space registry or identity/context policy. Equal descriptors
under different declaration names are compatible.

Vector component literals explicitly choose finite IEEE 754 binary64, including
rounding, even when written with integer tokens. They do not silently accept an
existing Integer, exact Decimal or Quantity scalar. Negative zero normalizes to
positive zero. Other finite values retain normal binary64 serialization. There is
no implicit unit conversion or role conversion. The current pure operations are
checked construction, identity/forwarding and same-type `vector_equal`; general
arithmetic, indexing and scalar measurement returns remain open.

Typed artifact values use `{"type":"vector","value":{"space":...,"role":...,"values":[...]}}`.
Untyped property and literal attachment substitution adds the existing coordinate
payload tag: `{"kind":"coordinates","space":...,"role":...,"values":[...]}`.
Use that value in an explicitly authored claim's `weave.geometry` property. Ordinary
scalar graph schemas reject vector leaves rather than treating them as a String or
Float. This does not introduce a runtime vector property schema.

The existing runtime independently validates manifestation-space anchoring,
positive claims, current permissions, time, context and provenance before native
geometry operations. Distance/similarity/transform/projection remain graph-valued
operations over authorized assertions. Physical direction is distinct from point
position and relation direction; a display projection does not become a metric or
identity bridge. See [the complete geometry fixture](../examples/vectors.weave).

The canonical pure geometry crate is separately vendored at the exact source pin
in `vendor/spaces-manifest.json`. Source `VectorDescriptor` and `CheckedVector` have
private fields and checked constructors/serde. `ScalarValue: Eq` is preserved:
checked vector components are finite and no mutable access can introduce NaN.
Duplicate JSON descriptor keys reject before interpretation; unknown/missing fields,
invalid role/dimensions and nonfinite components reject too. Source parsing rejects
duplicates before constructing a generic JSON object.

Limits are 256 vector declarations, 512 bytes per descriptor string, 4096 components,
existing 1 MiB per scalar / 4 MiB cumulative materialization and 100000 scalar work
steps. Equality charges each component and values are charged before cloning into
applications/literals. Serde consumes bounded sequences and rejects an excess element
without traversing it. Diagnostics include `E_VECTOR_DESCRIPTOR`,
`E_VECTOR_DESCRIPTOR_CONFLICT`, `E_VECTOR_DIMENSION`, `E_VECTOR_VALUE`, and existing
scalar type/parameter/budget errors, preserving original module/application spans.

This is partial implementation of paper §4/4.1/6 and gates L01/L03/L10/L11. General
vector schemas/arithmetic, full space declarations, learned bridges, uncertainty,
authorized conversion policies and identity inference remain outside this profile.
