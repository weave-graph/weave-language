# Ordinary scalar function specialization

The compiler supports Boolean, signed 64-bit Integer, String, Time, Interval, exact Decimal
and nominal Quantity parameters and results. Evaluation is deterministic and takes
place during compilation. It introduces no protocol change, hidden graph read,
authority time, network access, or runtime expression service.

```weave
function Add revision "1" (decimal left, decimal right) returns decimal {
  return decimal_add(param left, param right);
}
apply AddTenth from Add { decimal left decimal "0.1"; }
apply Total from AddTenth { decimal right decimal "0.2"; }
value Label string_concat("exact", " ✓");
```

`apply` with missing arguments yields an immutable partial function. A complete
scalar application yields a scalar binding and emits no runtime command.
`value NAME EXPR;` creates another scalar binding. Use `param NAME` for a function
parameter and `value NAME` for an earlier scalar binding. Scalar bodies can reference
their parameters, earlier local scalar bindings, and previously declared pure
functions. They cannot implicitly capture entry-global values.

Scalar-returning functions reject graph parameters, graph callbacks, graph-producing
function specializations and graph-operation body statements, including in unused
definitions. This bounded profile avoids an otherwise hidden graph capture/read.
Graph-returning functions may use scalar parameters, local scalar bindings and
scalar-returning function applications. Mixed graph-to-scalar functions remain open.

A function without `returns` keeps its existing graph return convention. Scalar
results require an explicit type. Quantity parameter and result types include the
entire unit descriptor:

```weave
function Twice revision "1"
  (quantity input dimension "length" unit "metre" revision "1")
  returns quantity dimension "length" unit "metre" revision "1" {
  return quantity_scale(param input, decimal "2");
}
```

Integer and Time are distinct. `time instant 5;` retains the established typed
argument syntax; standalone `5` is Integer, and standalone `time 5` is Time. There
is no implicit String→Decimal, Integer→Time, Float→Decimal, or Quantity→Decimal
conversion. Quantity dimension, unit and revision must match exactly.

## Callbacks and schemas

The existing `function transform` parameter still means a unary graph callback.
Scalar callbacks declare their one remaining parameter, named `input`, and result:

```weave
function Transform revision "1"
  (integer input, function transform(integer input) returns integer)
  returns integer {
  apply Output from transform { integer input param input; }
  return value Output;
}
```

A partially applied function can satisfy this signature. Explicit callback graph
schemas require complete descriptor equality and statically known input/output
constraints. No label-only compatibility or implicit schema erasure is introduced.
Nested callbacks, multi-argument callback signatures, function-valued returns and
schema polymorphism are outside this profile.

Scalar values can supply authored graph properties, nested structured literal
leaves and literal metadata attachments:

```weave
value Amount decimal "0.3";
graph G {
  node "n" entity "n" space "s" property "amount" value Amount;
  attachment "a" on node "n" key "amount" literal value Amount
    valid 0 until infinity;
}
```

Typed property references are checked against the declared property type **before**
conversion to JSON. Decimal's canonical string encoding therefore cannot masquerade
as a String result, or vice versa. Existing authored literal syntax is unchanged.
Untyped properties and metadata use the established JSON encoding; this does not
invent an additional runtime type tag. General typed assertion-property schemas
remain an engine/language gap.

## Operators and exact values

- Boolean: `boolean_not`, `boolean_and`, `boolean_or` (strict operands).
- Integer: `integer_add`, `integer_sub`, `integer_mul`, `integer_div`,
  `integer_equal`, `integer_lt`. Operations are checked; division requires an exact
  integer result. Overflow, zero division and nonzero remainder are errors.
- String: `string_concat`, `string_equal`. UTF-8 is preserved without normalization.
- Decimal: `decimal_add`, `decimal_sub`, `decimal_mul`, `decimal_div`,
  `decimal_equal`, `decimal_lt`.
- Quantity: `quantity_add`, `quantity_sub`, `quantity_scale`, `quantity_div`,
  `quantity_equal`, `quantity_lt`, `quantity_convert`.
- Time: `time_equal`, `time_lt`, identity and forwarding; no Integer arithmetic.
- Interval: checked half-open construction, bounds, containment/intersection and
  before/meets/overlap/within predicates; see [exact intervals](INTERVALS.md).

Decimal keeps the existing exact 18-digit/scale-18 bound, canonical string encoding
and finite exact division. There is no rounding or binary64 fallback. Quantity
conversion uses an explicit authored rational-conversion record and checks exact
source/target descriptors. It is arithmetic on supplied values, not authenticated
real-world conversion evidence. Affine offsets, dimensional products and inferred
unit aliases remain unsupported.

Unused bodies are symbolically type-checked. Closed expressions and known local
constants are evaluated for errors without assigning fake values to parameters.
Thus `integer_div(1, 0)` fails in an unused body, while division by an unknown
parameter remains valid until application supplies a value.

## API, identity and diagnostics

`specialize(source)` returns `SpecializedProgram { program, values }`. Values are
fully evaluated top-level scalars, retaining tagged types in deterministic name
order. `compile(source)` returns the same `program` field. Linked modules expose
`LinkedProgram::specialize`; the explicit local module loader remains unchanged.
Imported units export pure functions, not new top-level scalar constants.

```sh
weave values examples/scalars.weave
weave plan examples/scalars.weave
```

`values` emits typed values and a specialization fingerprint. This fingerprint
includes the ordinary Program, exported typed values and profile identifier.
`weave fingerprint` retains runtime-plan identity; two scalar-only programs may
have the same empty runtime plan but different specialization fingerprints.
Function manifests include scalar signatures/expressions and complete callback
schema descriptors. Whitespace/location changes do not alter normalized function
identity. Module content pins remain exact raw-source pins.

Arithmetic diagnostics identify the body operator and carry an application trace.
Linked errors retain original per-file spans, import trace and application source
locations. User JSON keys named `span`, `value` or `param` remain data.

## Bounds and evidence

Existing source/module and graph expansion bounds still apply. Scalar expressions
and function applications have depth limit 32. A compilation has at most 100,000
scalar evaluation/validation steps, 10,000 scalar bindings, 1 MiB per scalar
materialization and 4 MiB cumulative materialized scalar bytes. Quotas are charged
before value/closure retention and concatenation. Scalar-only applications count
even when they emit no graph commands. Exhaustion returns a diagnostic with no
partial plan or export table.

[Scalar tests](../tests/scalars.rs) and [independent boundary tests](../tests/root_scalars.rs)
cover exact arithmetic, callback partial application, nominal schema checks,
unused-body errors, source identities, module spans, bounded work and no hidden
commands. [CLI acceptance](../scripts/check_scalars.py) checks persistence and
fresh-process pinned replay against the engine. [Executed WASM parity](../scripts/check_scalar_parity.py)
compares typed results, complete plans, specialization identities and diagnostics
for a fixed fixture profile with zero WASM host imports. This is compiler-core
execution evidence, not browser storage or unrestricted application execution.
