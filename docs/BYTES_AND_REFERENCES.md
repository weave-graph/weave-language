# Bytes and exact reference values

Source-only Bytes and typed pinned references implement the ordinary value portion
of paper §4. They pass through pure functions, explicit unary callbacks, partial
applications, captures and exact pinned source modules. Runtime protocol 0.18 and
its graph schemas are unchanged.

```weave
value Data bytes "0041FF";
value N node_ref("Fleet", "revision-1", "device");
value E edge_ref("Fleet", "revision-1", "relation");
value A assertion_ref("Fleet", "revision-1", "claim");
value S snapshot_ref("Fleet", "revision-1");
value O object_ref(value N);
value Same reference_equal(value N, node_ref("Fleet", "revision-1", "device"));
function Keep revision "1" (node_ref input) returns node_ref { return param input; }
apply Kept from Keep { node_ref input value N; }
```

Bytes is an immutable sequence, at most 256 KiB. Hex literals have an even number
of ASCII hex digits; empty is valid. Uppercase and lowercase normalize to canonical
lowercase hex. `bytes_len` returns Integer, `bytes_equal` returns Boolean and
`bytes_concat` returns Bytes or a bounded error. No implicit String/UTF-8/base64/
numeric conversion occurs. Invalid hex uses `E_BYTES_LITERAL`; oversized values
use `E_BYTES_BUDGET`, with shared `E_SCALAR_BUDGET` for total work/materialization.

NodeRef, EdgeRef, AssertionRef and SnapshotRef have separate scalar types. Their
wrappers contain existing canonical `NodeRef`, `StructuralRef`, `AssertionRef` and
`GraphRef` records. Each field is nonempty and at most 512 UTF-8 bytes. Graph ID,
exact revision and local object ID retain their original bytes, including Unicode;
there is no normalization, head lookup or entity/space inference. EdgeRef names a
structural relation. A legacy asserted edge's claim identity uses AssertionRef.

`object_ref` explicitly widens one of these values (or preserves an ObjectRef) into
a tagged sum. It never silently narrows. `reference_equal` requires identical scalar
types; every identity field and ObjectRef kind participates. Node/edge/claim values
with the same text are different. Constructors accept pure String expressions;
invalid closed constructors in unused functions still fail with `E_REFERENCE_VALUE`.
Symbolic parameters are statically typed without fake evaluation.

## Serialization, metadata and authority

Artifacts preserve ScalarValue type tags. Bytes serialize as a hex string. Concrete
reference payloads serialize with their exact canonical fields; ObjectRef serializes
`{kind,reference}` where kind is node, edge, assertion or snapshot. Public Rust
wrappers have private fields, checked constructors and immutable accessors. Strict
serde rejects missing/duplicate/unknown fields and wrong kinds. Bounded string
visitors check reference lengths before retaining copies; parser escape scratch
requires an outer input budget, provided by the SDK's 8 MiB request limit.

Use `value Name` in a literal property or literal attachment. Bytes lower as
`{"kind":"bytes","encoding":"hex","value":"0041ff"}`. References lower as
`{"kind":"node_ref","reference":{"graph_id":"Fleet","revision":"revision-1","node_id":"device"}}`
with corresponding edge_ref, assertion_ref, snapshot_ref or object_ref tags.
ObjectRef retains its additional nested kind and full reference payload.

These are application data inside `MetadataValue::Literal`, not native navigable
`MetadataValue::Object` (which lacks revision/kind), `MetadataValue::Graph`, proof
carriers, capsule dependencies or reachability/retention roots. A value can name a
missing/private target without reading it or proving that it exists. Successful
compilation/persistence does not grant read access. Existing native graph-valued
metadata remains the explicit mechanism for authorized graph traversal. No reference
resolver, strong/weak reference policy or garbage collector is added by this profile.

Untyped properties can retain these envelopes. Existing runtime scalar schemas reject
them without erasing Bytes/NodeRef/etc. into String. Runtime reference/Bytes schemas,
optional unresolved values and native resolution need separate protocol work.

## Bounded work and verification

Byte copies and concatenation charge the hex-expanded retained size before allocation;
comparison/concatenation charge one work step per started 64 input bytes. Reference
copies charge conservative escaped identity size; equality charges bounded work.
Existing 100,000-step, 1 MiB per value and 4 MiB aggregate specialization limits remain.
Checked concatenation rejects overflow/oversize; errors never truncate data.

[The example](../examples/bytes_references.weave) exercises exact pointers and Bytes
through functions and literal persistence. `scripts/check_bytes_references.py` runs
against an unchanged protocol-0.18 engine, checks fresh-process exact replay, and
shows unavailable/private named targets do not become traversal, dependency or grants.
The SDK conformance harness includes the same source on native/WASM with exact response
bytes. See [STATUS](STATUS.md) for executed checks and remaining full-design gates.
