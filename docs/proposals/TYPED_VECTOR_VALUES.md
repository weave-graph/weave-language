# Typed vector values: bounded source-only proposal

Status: approved bounded implementation profile; see [implemented source semantics](../TYPED_VECTORS.md). Protocol remains 0.18. The canonical validator-only checkpoint is separately pinned; no runtime vector schema was added.

## Required scope and existing implementation

The recovered language paper §4 lists vectors in the ordinary value family and
requires `Vec<3, BuildingA, Meter>` to differ from encoder coordinates. §4.1 requires
pure ordinary-value functions. §6 distinguishes physical positions/directions,
embedding coordinates, relation direction, identity bridges and display projections.
The current gap rows are L01/L03 and the typed-value portions of L10/L11. This slice
would close ordinary vector construction and function typing; it would not close
full spatial declarations, vector arithmetic, learned mappings or general schemas.

The runtime already accepts assertion property `weave.geometry` with tagged
`coordinates {space, role, values}` and graph-valued distance/transform/projection
operators. `Space`, `Geometry`, `VectorRole` and `Coordinates` are currently defined
in engine `weave-spaces`; they are not in vendored `weave-contract::geometry_types`,
which contains only graph operands/operators. The language can retain the existing
wire shape without claiming that arbitrary source data is authorized `Evidence`.

Recommendation after root review: exactly vendor the existing portable `weave-spaces`
crate (serde/serde_json/weave-contract only) under a separate source/byte manifest.
Request a narrow engine-owned checkpoint exposing pure `Space::validate` and the
existing `Coordinates::validate`; no Evidence construction or authority-shaped input
is necessary for validation. Its DTO bytes remain unchanged. Use those canonical
Space/Geometry/VectorRole types inside a language-owned private checked value wrapper.
The wrapper enforces source bounds before allocation/cloning and invokes the public
pure validators; it never manufactures authorized Evidence for arithmetic. Parent and
engine must approve this precise file handoff before any canonical edits. No protocol,
store or existing contract vendor-set change is required; the additional crate has
its own exact source pin/license verification. Native effects code remains engine-owned.

## Proposed grammar and exact type

Use explicit source type declarations with the existing Space descriptor syntax:

```weave
vector_type Position3 space {
  "id":"physical", "revision":"1",
  "geometry":{"kind":"physical3d","frame":"building-A","unit":"metre"}
} role position;
vector_type Semantic3 space {
  "id":"semantic", "revision":"1",
  "geometry":{"kind":"embedding","encoder":"encoder-v7",
    "preprocessing":"tokens-v2","dimensions":3,"metric":"cosine_distance"}
} role embedding;
function Keep revision "1" (vector input type Position3) returns vector type Position3 {
  return param input;
}
apply Origin from Keep { vector input vector Position3 [0.0,0.0,0.0]; }
value Same vector_equal(value Origin, vector Position3 [0.0,0.0,0.0]);
```

The descriptor object is a strict literal declaration: no scalar substitutions,
unknown/duplicate keys or dynamic source reads. Its type is the complete Space
plus role, not the source alias or dimension count. Physical coordinates have
exactly three components and role position/direction; embedding coordinates require
role embedding, dimensions 1..4096, explicit encoder/preprocessing/metric. Unit
values remain exactly metre/centimetre/millimetre, metrics euclidean/cosine_distance.
Same-length vectors differing in space ID, revision, frame, unit, encoder,
preprocessing, metric or role are incompatible. No unit conversion, learned bridge,
role reinterpretation, identity inference or descriptor fallback occurs.

Within a linked declaration environment, the same Space ID/revision with different
geometry is a conflict. Different revisions remain different types. Two aliases
for the same full descriptor/role are compatible. This is a compiler environment
check, not a read-time global schema registry or installation of a space policy.
These declarations describe coordinates only; they do not implement the paper's
full space schema/identity/context declaration.

## Values, numeric boundary and operations

`vector TYPE [components]` supplies a finite binary64 coordinate vector, using the
existing authored structured-number parsing semantics. It is explicitly approximate
binary64, separate from exact Decimal and nominal Quantity; it never converts an
existing Decimal/Quantity/Integer scalar parameter implicitly. Component literals
may use numeric token forms already supported by structured literals; their vector
context explicitly chooses binary64 representation. Overflow to infinity rejects.
Canonicalize negative zero to zero for source value equality/identity; preserve all
other finite values through ordinary serde binary64 encoding. Existing untyped JSON
literals and existing example identities remain unchanged.

Initial pure operations: checked construction, identity/forwarding, immutable
partial application, unary typed callbacks, exact `vector_equal` requiring equal
full types, and literal substitution. Arithmetic, component extraction returning a
new Float scalar, cross-space conversion and scalar measurement returns are deferred.
Existing graph-valued native geometry operations remain the path for authorized
physical distance, transforms and embedding similarity. This avoids inventing an
unreviewed numeric/result family merely to complete vector parameter support.

Typed artifacts retain a vector tag plus complete descriptor/role/components.
Literal substitution emits the existing payload directly:
`{"kind":"coordinates","space":...,"role":"position","values":[...]}`.
An explicit source claim may use `property "weave.geometry" value Origin`.
Other untyped properties and metadata may store the same descriptive value.
Existing scalar property schemas reject vector leaves rather than erase nominality;
no runtime Vector schema is introduced.

The engine still checks current authorization, claim positivity/time/context,
manifestation-space anchoring and source proof, then validates the payload before
calculation. A scalar descriptor/value supplies no trusted reader, provenance,
visibility, clock or Evidence wrapper.

## Compiler and resource seams

- New private-field checked vector descriptor/value module, including strict
  bounded Deserialize and validating constructors around canonical weave-spaces
  DTOs. Canonical DTO public fields remain native API data, not the checked source
  wrapper; no unchecked conversion or mutable component slice is exposed. Reject
  nonfinite values and invalid roles/dimensions from Rust and source/module paths.
  ScalarType remains Eq through Space/role; ScalarValue retains Eq through a private checked vector with lawful manual Eq;
  finite components and normalized zero make equality total on accepted vectors.
  Do not derive Eq on an unchecked f64 DTO.
- Add ScalarType::Vector(full descriptor), ScalarValue::Vector(checked value),
  typed literal and vector-equality signatures; preserve existing scalar variants.
- Parser recognizes declaration, type references and literals. Check declaration
  identity strings at 512 bytes and dimensions before component allocation; reject
  the 4097th component before retaining it. Charge descriptor/components before
  cloning into values, callbacks, partial closures or literal payloads.
- Reuse 1 MiB per scalar/4 MiB aggregate retained scalar budget, 100000 steps and
  existing source/AST depth/application limits. Precharge equality by component
  count; do not make a 4096-component scan cost one unaccounted scalar operation.
- Expander resolves vector type names before parameter/body checking, including
  unused bodies and closed literal errors. Graph return schemas remain intact.
- Modules export/qualify only vector declaration/type references, never descriptor
  string literals. Capture full descriptors; retain raw module content manifests,
  original source spans and import/application traces. Type aliases/import order
  do not change normalized function or complete-artifact identity.
- Complete artifact and legacy output guards remain unchanged. Handler recipes
  may capture these pure values without a graph read or added runtime command.

Diagnostics: E_VECTOR_DESCRIPTOR (invalid descriptor/role), E_VECTOR_DIMENSION,
E_VECTOR_VALUE (nonfinite/bad component), E_VECTOR_DESCRIPTOR_CONFLICT, existing
E_SCALAR_TYPE/E_PARAMETER_TYPE/E_BUDGET and exact original spans. Final spelling is
subject to review before implementation.

## Required acceptance

1. Strict constructors/serde: duplicate, unknown/missing fields, mismatched role,
   empty/4097 dimensions, nonfinite values, wrong component count and extreme finite
   values; no bypass via ScalarValue deserialization.
2. Exact compatibility matrix changes every descriptor field and role independently;
   equal lengths never imply compatible frames/encoders. Same full descriptor under
   different aliases succeeds; same label changed geometry conflicts.
3. Partial/unary callbacks, unused definition checking, module original diagnostic
   spans, transitive capture, alias-stable identity, changed exact pin identity,
   complete artifacts and zero-command pure specialization.
4. Literal property/metadata output plus scalar schema rejection; formatter round
   trip/idempotence and old standalone artifacts/fingerprints unchanged.
5. Actual compiler-to-native explicit coordinate claims: distance 3-4-5, explicit
   rigid transform reuse and embedding cosine, with existing runtime mismatch,
   context/time/private-claim and projection-as-distance rejection retained. Compare
   typed compiler emission with the existing hand-authored geometry payloads.
6. Executed native/WASM parity for values, artifacts, diagnostics and fingerprints;
   budget tests for repeated large-vector comparisons and partial captures.

No source clock, model execution, network encoder, conversion authority, similarity
identity, display projection authority or automatic graph effect is added.
