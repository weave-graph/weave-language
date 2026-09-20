# Ordinary scalar functions: proposed source-only profile

Status: historical accepted design, now implemented as the bounded [ordinary scalar profile](../../SCALAR_FUNCTIONS.md). Based on language `5c3e909`; protocol 0.15 is unchanged. The `.weave` files beside this document are executed acceptance fixtures. The implementation additionally rejects graph parameters, graph callbacks and graph operations in scalar-returning functions to preserve zero runtime commands; graph-returning functions may use scalar locals and parameters.

## Requirement and existing boundary

Language paper §4 includes ordinary booleans, integers, decimals, strings and typed
quantities; §4.1 distinguishes pure value functions from graph lenses and effects;
§5 requires partial application. This profile advances L01–L03 without closing
bytes, intervals, vectors, references, polymorphism or dimensional product algebra.

Current `syntax::ParameterKind`, `ArgumentValue` and `functions::Value` distinguish
graph/string/time/function. Functions return graph names; function arguments have
one remaining graph parameter named `input`. `syntax::numeric_literal` evaluates
exact numeric literal operators while parsing, but drops Decimal versus String
nominality into JSON afterward. `functions::validate_definition` uses symbolic
graph knowledge and placeholder string/time values. Graph-schema constraints
compare full descriptors. Pinned module linking rewrites syntactic references and
retains original source spans and dependency identities.

Recommendation: add a typed scalar expression tree and specialize it during
compilation. Scalar expressions never inspect runtime graph contents. Successful
specialization emits the same existing runtime graph commands and canonical scalar
property encodings as manually authored literals. No new engine DTO or protocol
version is needed.

## Proposed surface

Existing graph syntax and its default graph return remain valid. New scalar-returning
functions require `returns TYPE`. A Quantity parameter/result includes its entire
nominal unit descriptor. `time` remains distinct from `integer`, even though both
use signed 64-bit storage.

```weave
function Sum revision "1" (decimal left, decimal right) returns decimal {
  return decimal_add(param left, param right);
}
apply AddTenth from Sum { decimal left decimal "0.1"; }
apply Total from AddTenth { decimal right decimal "0.2"; }
value Label string_concat("total=", "0.3");
```

`value NAME EXPR;` creates an immutable scalar binding. `apply NAME from F {...}`
continues to create a partial closure when arguments remain. A full application
creates a graph binding or scalar binding according to the declared return type.
`param NAME` refers only to a current function parameter; `value NAME` refers only
to a preceding scalar binding. A bare string is String, never Decimal. The repeated
`decimal` in `decimal left decimal "0.1"` intentionally distinguishes the argument's
type marker from its exact literal constructor.

```text
scalar-type := boolean | integer | string | time | decimal
             | quantity dimension STRING unit STRING revision STRING
scalar-expr := true | false | INTEGER | STRING | time INTEGER
             | decimal STRING | quantity STRING UNIT-DESCRIPTOR
             | param NAME | value NAME | BUILTIN '(' scalar-expr (',' scalar-expr)* ')'
scalar-binding := value NAME scalar-expr ';'
scalar-return := return scalar-expr ';'
scalar-argument := scalar-type-keyword NAME scalar-expr ';'
```

Quantity arguments carry the descriptor in their value; the full expected descriptor
comes from the parameter signature. Argument `time instant 5;` retains the existing
context-directed Time literal syntax; standalone `5` is Integer and standalone
`time 5` is Time. `time instant value Count;` fails when Count is Integer. No implicit
Integer→Time, Integer→Decimal, String→Decimal, Quantity→Decimal or Float conversions.

Graph properties and scalar literal attachment payloads gain the explicit leaf
form `value NAME` (including inside structured object/array literals). All other
literal syntax keeps its current meaning. Such references are resolved before
schema validation and plan lowering. Ordinary JSON properties named `value`,
`param`, `span`, or a type name remain ordinary data. Quantity/Decimal values lower
to their existing canonical object/string encodings only at this boundary.

Function-local scalar bindings and scalar applications may feed graph-function
parameters (`string relation value Relation;`, `time instant value Instant;`) and
scalar returns. String/time result references also work in the existing lens
relation/time slots. New Boolean/Integer/Decimal/Quantity parameters can be forwarded
to pure scalar calls, returned, or used by scalar builtins; this profile does not
invent a runtime property predicate/threshold operator.

## Signatures, closure and higher-order checking

- Omitted `returns` still means graph; `returns graph schema S` is unchanged.
  `returns graph` may be accepted as an explicit unconstrained synonym.
- `function transform` retains the current unary graph-function convention.
- Add explicit unary callback signatures, e.g.
  `function transform(integer input) returns integer`. There must be exactly one
  remaining parameter named `input`, of the annotated type, with the annotated
  result. Quantity descriptors compare exactly. Explicit graph constraints compare
  complete descriptors; unknown knowledge cannot satisfy a declared constraint.
- Callback annotations cannot themselves contain function parameters. General
  multi-argument callbacks, function-valued returns and polymorphism stay open.
- A callback may be a partial application of a larger function. Bound values retain
  their types and graph schema obligations; binding does not erase the remaining
  signature. Duplicate bindings still fail.
- Definitions are checked even when unused. Type-check scalar expressions
  symbolically; never substitute fake zero/empty parameter values to evaluate a
  body. For example `decimal_div(param x, param y)` is valid until a concrete zero
  denominator is supplied. Fully closed erroneous literal expressions fail early.
- Definition capture remains lexical for earlier pure function definitions. Scalar
  bodies may use parameters and earlier local scalar bindings, but do not implicitly
  capture entry-global scalar bindings, graph handles or runtime values. Imported
  units continue to export functions/schemas/rules/context schemas; exporting new
  top-level scalar constants is outside this slice. A zero-parameter pure function
  can return an authored constant.
- No recursion, reflection, graph-to-scalar aggregate, dynamic callback lookup,
  conditional effects, graph construction inside a pure body, or hidden source read.

## Arithmetic and data semantics

Use the existing portable Decimal and Quantity implementations, retaining typed
values throughout specialization. Decimal remains bounded exact coefficient/scale
(up to 18 digits and scale 18); division requires an exactly representable finite
result. Overflow, division by zero and inexact results are explicit diagnostics.
There is no binary64 roundtrip or rounding mode. Existing Float graph literals and
schemas remain supported, but Float is not an implicit ordinary function type here.

The initial builtin set is deliberately finite:

| Family | Operations and meaning |
|---|---|
| Boolean | `boolean_not`, `boolean_and`, `boolean_or`; strict operands, no short-circuit effects |
| Integer | `integer_add/sub/mul/div`; checked i64, division requires zero remainder; `integer_lt`, `integer_equal` |
| String | `string_concat`, `string_equal`; UTF-8 bytes preserved, no normalization or automatic formatting |
| Decimal | Existing `decimal_add/sub/mul/div`; `decimal_lt`, `decimal_equal` use exact comparison |
| Quantity | Existing `quantity_add/sub/scale/div`; comparisons require identical dimension/unit/revision |
| Time | Identity/forwarding and explicit literals only; not ordinary Integer arithmetic |

Existing `quantity_convert` remains available for literal conversion records. Its
quantity operand may be a typed scalar expression, but the conversion descriptor
and exact rational factor must remain explicit authored literals; this does not
establish real-world conversion authority. No affine offsets or inferred unit
aliases. A Quantity return type cannot be inferred merely from a matching amount.

## AST, lowering and source identities

Introduce a source-owned `ScalarType`, `ScalarValue` and spanned `ScalarExpr` in
`src/scalars.rs`. ScalarValue contains Bool/i64/String/Time/Decimal/Quantity directly,
not untagged JSON. Add explicit return/signature information with backward-compatible
AST defaults. Do not encode unresolved references as magic JSON objects: use typed
literal-expression leaves or a distinct templated-literal AST variant, and emit
plain existing JSON only after all references have been resolved.

The expander holds separate graph/scalar/function symbol tables and one shared
name-collision set. Partial closures store immutable typed scalar values. Full
scalar evaluation populates the scalar table and emits **zero** runtime commands.
Property substitution then invokes ordinary graph-schema validation. Scalar code
cannot cast a dynamic graph to a statically proven schema. Graph transfer analysis
and exact callback schema checks remain mandatory.

Proposed API: `specialize(source) -> SpecializedProgram { program, values }`, where
values contains only fully evaluated top-level named scalars in deterministic name
order, with tagged type information. `compile(source)` delegates and returns the
unchanged Program field. Linked modules gain the same specialization entry point;
CLI `weave values FILE [--modules MAP]` emits the typed values. No serialized closure
or runtime scalar evaluation endpoint is added.

Keep existing runtime plan fingerprint semantics. A scalar-only program may have
no runtime commands. A separate specialization fingerprint hashes the complete
Program plus typed exported values and a versioned compiler-profile tag; changing
a returned scalar must change this fingerprint even when the runtime plan is empty.
Function manifests include parameter/result types, nominal descriptors, normalized
scalar expressions and captured definition dependencies. Strip only known AST span
fields; user payload keys are never recursively removed. Module alias/order
invariance and original file/span diagnostics remain unchanged.

## Bounded work and diagnostics

Preserve current source/module budgets, expansion depth 32, 10,000 emitted
statements and 4 MiB expanded AST. Add scalar-expression depth 32, 100,000 evaluated
expression/call steps across the whole compilation, at most 10,000 scalar bindings,
1 MiB per retained scalar/string value and 4 MiB cumulative materialized scalar
bytes. Charge source-sized arguments, closure captures, concatenation output and
exported values before cloning or retaining; sharing Arc values does not erase the
logical quota. Check scalar steps even when expansion emits zero graph statements.
No loop, arbitrary recursion or arbitrary-size integer library is needed.

Retain existing `E_PARAMETER_TYPE`, `E_FUNCTION_SIGNATURE`, `E_FUNCTION_RETURN`,
`E_FUNCTION_SCOPE`, `E_DUPLICATE`, schema and module diagnostics. Add
`E_SCALAR_TYPE`, `E_SCALAR_UNKNOWN`, `E_SCALAR_RETURN`, `E_SCALAR_BUDGET`; arithmetic
uses existing exact numeric diagnostics plus an Integer overflow/inexact category.
Primary spans point to the offending argument/operator/reference; an evaluation
failure carries the originating body span and application trace. Linked diagnostics
map each trace location back to its original source unit. Exceeding any bound
produces no partial executable plan or partial export table.

## Acceptance gates and implementation ownership

1. Portable typed expression/parser tests: no JSON/string type laundering, nominal
   descriptor equality, exact arithmetic and precise spans. Test large integers
   above 2^53, Decimal 0.1+0.2, division errors, UTF-8 and zero/negative values.
2. Specialization: direct/partial/higher-order equivalent typed values; scalar result
   feeding an authored schema-checked property; wrong return/argument/callback and
   Integer-versus-Time errors; unused definitions symbolically checked; no hidden
   graph commands. Resource exhaustion also tested for scalar-only workloads.
3. Modules/identity: imported scalar function, transitive captured function,
   alias-renaming stability, changed result identity, reserved namespace protection,
   per-file error traces and original literal keys retained.
4. Actual compiler→engine: canonical scalar property equality to handwritten
   literals, quantities retained under exact schema; graph function schema checking
   unchanged; a failing source specialization produces no runnable plan. Engine
   rollback is not a compile-time feature and is not newly claimed.
5. Native/WASM compiler parity, source archive build, old fixtures/identities and
   exact vendored contract hashes unchanged; independent review before publication.

Implementation ownership remains language-only: new `src/scalars.rs`; syntax,
functions, graph-types/module visitors, API/CLI and tests/docs. Do not edit vendored
contract or engine files. Representative inputs and expected outcomes are in
`fixtures/`; they now run as part of compiler conformance.
