# Bytes and pinned object values

Accepted source-only protocol-0.18 profile; see [implemented values](../BYTES_AND_REFERENCES.md). No runtime/schema/authority changes.
Paper §4 explicitly includes bytes and object references and requires node and edge
references to be different types. §4.1 requires ordinary pure value functions;
§9 states a reference/content identifier is not proof of access. This closes the
missing ordinary value slice of L01–L03, not optional/resolvable-reference semantics
or complete graph type/schema evolution.

## Values, syntax and existing identities

Add checked immutable source values and scalar signatures:

| Source type / constructor | Exact payload |
| --- | --- |
| `bytes` / `bytes "00ff"` | Up to 256 KiB; even ASCII hex, case normalized to lowercase |
| `node_ref` / `node_ref("G", "r1", "n")` | Existing canonical `NodeRef {graph_id,revision,node_id}` |
| `edge_ref` / `edge_ref("G", "r1", "e")` | Existing `StructuralRef {graph_id,revision,edge_id}` |
| `assertion_ref` / `assertion_ref("G", "r1", "a")` | Existing `AssertionRef {graph_id,revision,assertion_id}` |
| `snapshot_ref` / `snapshot_ref("G", "r1")` | Existing `GraphRef {graph_id,revision}` |
| `object_ref` / `object_ref(value N)` | Explicit tagged sum of the four reference types |

Every reference retains an exact graph/revision pin and object-kind namespace.
`edge_ref` names a structural relation; an asserted legacy edge's claim identity
uses `assertion_ref`. Equal node/edge/claim text never equates their types or values.
References do not carry entity/space identity: those require authorized graph reads.
An object wrapper does not implicitly narrow back to a node/edge, or widen another
argument; `object_ref(...)` is the explicit widening operation.

Example:

```weave
function Keep revision "1" (node_ref input) returns node_ref { return param input; }
value N node_ref("Fleet", "r1", "device");
apply P from Keep { node_ref input value N; }
value O object_ref(value P);
value Payload bytes "00FF";
value Count bytes_len(value Payload);
```

Pure operations: `bytes_len -> Integer`, `bytes_equal -> Boolean`, bounded
`bytes_concat -> Bytes`, and `reference_equal -> Boolean` for two identical scalar
reference types (including ObjectRef; its tag participates in equality). No numeric,
UTF-8, base64 or reference-string coercions. No dereference, existence check, head
lookup, grant, schema assertion, provenance fabrication or host I/O is introduced.
Unknown/remote/private pins remain ordinary authored data; successful compilation
makes no claim about their availability. Empty Bytes is valid.

## Checked representation and lowering

New source modules `bytes.rs` and `references.rs` use private fields, checked Rust
constructors and custom/strict deserialization. Reference wrappers contain the
existing canonical DTOs rather than divergent identities; all identity strings are
nonempty and at most 512 UTF-8 bytes, matching canonical influence validation. Their
bytes are otherwise preserved exactly: no Unicode normalization/case folding.
Derive lawful Eq from immutable byte sequences/checked canonical references.

Byte serde is a canonical lowercase hex string; validate length/parity/ASCII digits
before allocation, accepting uppercase only as alternate source/decoder spelling.
The deserializer uses a string visitor (no numeric array allocation). Reference serde
retains exact canonical fields; object serde is `{kind,reference}`, strict per variant.
Reject unknown, missing and duplicate fields before any generic JSON Value can erase
them. Raw JSON parser scratch allocation is a documented outer-input responsibility;
SDK requests already enforce 8 MiB. Public wrappers enforce retained-value bounds on
all Rust/serde/source construction paths; no mutable DTO accessors are exported.

`ScalarValue` artifacts retain their ordinary type tags. Literal metadata/property
lowering uses explicit application data envelopes:
`{"kind":"bytes","encoding":"hex","value":"00ff"}` and
`{"kind":"node_ref","reference":{...}}` (corresponding distinct tags for other
references; ObjectRef retains its nested kind). These use existing `MetadataValue::Literal`
and generic untyped JSON properties. They never become `MetadataValue::Object`,
which currently lacks revision and kind, nor `MetadataValue::Graph`, proof carriers
or capsule dependencies. Thus no pins are silently discarded or promoted into
traversal/retention/authorization. Existing runtime scalar schemas cannot accept these
new nominal source values by erasing them to String/object. Adding runtime schema
variants or native reference resolution is a separate reviewed protocol change.

## Bounds and compiler integration

Retained byte accounting charges canonical hex expansion plus envelope overhead;
precharge byte copying, concatenation/equality work and reference cloning before
allocation, sharing existing 100,000-step / 1 MiB-value / 4 MiB-materialization limits.
References charge all exact identity strings plus tagged JSON overhead. Checked
arithmetic rejects overflow; never truncate. Diagnostics: `E_BYTES_LITERAL`,
`E_BYTES_BUDGET`, `E_REFERENCE_VALUE`, ordinary `E_SCALAR_TYPE` and shared budget errors.
Unused definitions reject closed malformed constructors while genuinely symbolic
parameters remain type-checked without fake evaluation.

Extend parser scalar types/literals/builtin signatures, argument binding, unary
callbacks/partial applications, captured definitions, module qualification and literal
visitors. Reference fields are string expressions/data; never namespace-rewrite their
contents. Constructor syntax is pure scalar expression syntax, not graph selection.
Formatter remains token preserving. Full artifact/SDK output and fingerprints include
new values; old values/artifacts, vendor bytes and protocol remain unchanged.

## Acceptance before freeze

- Rust, serde and source boundary tests: all byte values incl NUL/FF, empty/odd/bad
  hex, exact max/over-max, duplicate/unknown/missing reference fields, Unicode and
  512-byte limits; original private invariants cannot be bypassed.
- Distinct node/edge/assertion/snapshot kinds, exact graph/revision/id equality,
  explicit ObjectRef widening, wrong callback/argument types, transitive modules,
  partial application, unused closed errors and original spans; no implicit reads.
- Literal attachment and untyped property compiler→native persistence roundtrip;
  nonexistent/private named targets do not cause resolution, extra dependencies or
  authority. Typed scalar properties reject nominal erasure.
- SDK arbitrary-input native/WASM response parity, complete artifact identities,
  bounded repeated operations, formatting idempotence, historical artifact equality,
  full source suite/lint/exact vendors and archive build using the existing cache.
