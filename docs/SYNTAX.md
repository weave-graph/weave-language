# Proposed bootstrap syntax

This is implemented experimental syntax, **not recovered white-paper syntax**. The compiler has no I/O authority beyond reading the source file in its CLI; output is a plan for the engine to validate and execute. Epoch values are signed Unix milliseconds. Intervals are half-open `[start, end)`.

The core declaration grammar below is an excerpt; later sections define explicit assertions, functions, algebra, rules, contexts and geometry.

```ebnf
program = { schema | transaction | graph | use | lens | bind | join | metadata_query } ;
transaction = "transaction", identifier, "{", graph, { graph }, "}" ;
metadata_query = "metadata", identifier, "from", identifier, "on", host, "key", string, ";" ;
host = ("node" | "edge" | "assertion" | "entity"), string | "graph" ;
schema = "schema", identifier, "revision", string, "{", { node_schema | edge_schema }, "}" ;
node_schema = "node", identifier, [ "space", string ], shape ;
edge_schema = "edge", identifier, "from", identifier, "to", identifier, [ "cross_space" ], shape ;
shape = "{", { "property", string, scalar_type, ("required" | "optional"), ["nullable"], ";" | "open", ";" }, "}" ;
scalar_type = "string" | "integer" | "float" | "boolean" ;
graph = "graph", identifier, [ "schema", identifier ], "{", { node | edge | attachment }, "}" ;
node = "node", string, [ "type", identifier ], "entity", string, "space", string, { metadata | property }, ";" ;
edge = "edge", string, [ "type", identifier ], "from", string, "to", string,
       "relation", string, [ "polarity", ("positive" | "negative") ], "valid", integer, "until", (integer | "infinity"),
       { metadata | property }, ";" ;
metadata = "metadata", "graph", string, "revision", string ;
attachment = "attachment", string, "on", host, "key", string, "graph", string,
             "revision", string, "valid", integer, "until", (integer | "infinity"), [ "required" ], [ "context", "graph", string, "revision", string ], ";" ;
property = "property", string, literal ;
literal = string | integer | float | "true" | "false" | "null" | array | object ;
array = "[", [ literal, { ",", literal } ], "]" ;
object = "{", [ string, ":", literal, { ",", string, ":", literal } ], "}" ;
use = "use", identifier, "graph", string, [ "revision", string ], ";" ;
lens = "lens", identifier, "from", identifier, "{", { filter }, "}" ;
filter = "match", "relation", (string | "param", identifier), ";"
       | "at", (integer | "param", identifier), ";"
       | "metadata", "depth", integer, ";" ;
bind = "bind", identifier, "from", identifier, "{", { binding }, "}" ;
binding = "string", identifier, string, ";" | "time", identifier, integer, ";" ;
join = "join", identifier, "from", identifier, "to", identifier, "relation", string, ";" ;
```

Identifiers use ASCII letters or underscore followed by letters, digits or underscore. Strings use JSON escaping and may contain Unicode. `//` comments end at a newline. Strings used as identities must be nonempty. Metadata depth is 0–32. Source is limited to 1 MiB and 100,000 tokens.

Declarations are sequential; graph/lens names and object IDs within each graph cannot repeat. An edge must reference nodes declared in the same graph, regardless of declaration order. Node and edge IDs share a namespace within a graph. A graph declaration compiles to a **whole-snapshot initial commit**, not a patch; no expected head means the branch must not exist. Re-running the example on an existing database therefore conflicts by design. Use a fresh database for the bootstrap example.

`use` imports a graph reference without reading or writing it during compilation. A `revision` clause pins the system snapshot; `at` filters valid time. Omitting revision asks the engine to resolve the current branch head at query time. Those operations are deliberately distinct.

A bootstrap `lens` produces a graph value from a stored query or a previously computed value. Composing one with an earlier lens combines compatible relation/time filters and inherits metadata options; conflicting filters are rejected. Named string/relation and time parameters use `param name`. A `bind` declaration specializes any subset with typed values. Unbound templates emit no evaluation; fully bound lenses evaluate once and bind an immutable program-local graph value. Unknown, duplicate and incorrectly typed bindings fail compilation. This small filter language is **not yet general graph-valued functions or a rule evaluator**. Unknown operations, including writes or host authority fields inside a lens, fail parsing.

All bootstrap source-created nodes and edges are public because the grammar does not yet expose visibility declarations. The engine protocol supports restricted readers and must enforce host authority independently. Do not use this source subset to express a private graph until permission syntax is implemented; create such fixtures through the explicit protocol instead.

Metadata graph revisions are opaque references. The compiler does not pretend to verify their existence; the engine reports unavailable or depth-limited metadata as partial coverage. Cyclic metadata is never recursively inlined into the plan.

## Commands

```sh
cargo run -- check examples/fleet.weave
cargo run -- ast examples/fleet.weave
cargo run -- describe examples/schema.weave
cargo run -- plan examples/fleet.weave > fleet.plan.json
```

Run the generated plan through the sibling engine's trusted host interface:

```sh
weave-engine run --db /tmp/weave-demo.json --actor demo --write Fleet fleet.plan.json
```

Use a new database path. The engine's current CLI host authority flags are a local development trust boundary; a submitted source file cannot set them. The language does not launch adapters or external programs.

Diagnostics are JSON on stderr with `code`, `message`, and UTF-8 byte `start`/`end` offsets. Lexical/parser offsets identify the failing token. Semantic checks retain AST source spans and identify the actual offending declaration or graph reference, including quoted Unicode identities.

## Parameter binding and graph joins

See [parameters.weave](../examples/parameters.weave) for partial application and [join.weave](../examples/join.weave) for a two-graph temporal path join. Fully bound lens/query inputs join with the explicit `entity_space_to_from` rule: the left target and right source must share exact entity and space identities. Local node IDs can differ. The derived edge spans the intersection of both valid intervals and retains both premises. Engine authorization governs both inputs and the result.

Join outputs can feed later lenses, parameter binding and joins. Each concrete result binds once under its source name; later use refers to that immutable program-local value and performs no hidden commit. See [composed.weave](../examples/composed.weave) and protocol [v0.3 notes](contract/v0.3/README.md). Stored graph imports remain source references and are resolved when evaluated. Arbitrary cross-graph matching, user-defined identity alignment, higher-order graph parameters and recursive rules remain outside this implemented slice.

## Claim polarity and scalar properties

Edges default to positive support; `polarity negative` records explicit negative support without deleting positive evidence. The current path join derives from positive premises only. A source graph can retain both. A full four-valued proposition query/resolution API remains planned.

Nodes, edges and explicit assertions accept repeated `property "key" value` clauses. Values are strings, signed integers, finite floating-point numbers, booleans, null, arrays or structured objects; property keys must be unique and nonempty. Graph-valued metadata continues using `metadata graph ... revision ...`. Literal strings and objects are data, not executable code.

## Named metadata and cyclic local transactions

[metadata_cycle.weave](../examples/metadata_cycle.weave) creates two graph snapshots inside one explicit `transaction boot { ... }`. An `attachment` has its own identity, host kind/identity, key, pinned graph value and valid interval. `required` demands that the runtime validate the referenced snapshot's availability. The source profile currently emits public attachments without custom origin/schema-revision fields; those richer runtime fields remain future source syntax.

The transaction lowers to one atomic `CommitBatch`. A same-batch graph revision is written as `logical:batch-id:graph-id`; the runtime resolves those identities through one snapshot manifest and validates required references after inserting all members. This allows real cyclic references without recursive hashes. The batch ID is an ASCII identifier at most 64 bytes. Nested transactions, empty batches and non-graph statements inside a batch are rejected. Graph declarations currently create new snapshots; source-level expected-head updates/rebinding remain later work.

A query resolves named metadata with `metadata depth N;`. `metadata Proof from Result on edge "connection" key "evidence";` selects the resolved graph as another immutable graph value. It preserves attachment-path provenance and visibility. Missing, ambiguous or unmaterialized metadata yields explicit incomplete coverage from the engine, not fabricated emptiness. It performs no hidden live read. That value can feed ordinary lenses and joins.

Legacy anonymous `metadata graph ... revision ...` remains available for compatibility. Named attachments are the path to the full paper model. Live handles, typed graph-valued schema fields and independently specified attachment origin/policy fields are not yet exposed in source.

## Versioned schemas and discovery

[Schema example](../examples/schema.weave) declares a versioned graph schema with distinct node and edge types. A graph selects it with `schema Name`; each node/edge declares `type Name`. Schemas must be declared before use. Definitions state property types and presence/nullability, optional exact node spaces, and edge endpoint types. Cross-space edges require explicit `cross_space`; undeclared scalar properties require explicit `open;` in the type's shape.

`weave describe FILE` validates the file and emits its source schema descriptors as JSON without executing a plan. This is source-level discovery, not a privileged inventory of remote graphs. The engine's graph results carry their own schema descriptor; runtime authorization still governs access.

Both compiler and direct runtime clients use the same portable validator. Supported scalar schema types are string, signed 64-bit integer, finite binary64 float and boolean; nullable values are explicit. Exact decimal, vector/quantity, graph-valued metadata field schemas, variance and disconnected migrations remain open. The grammar's schema revision is an explicit identity, not a claim that a field rename or unit change is safe.

## Graph algebra (protocol 0.5)

```weave
union Combined from Left with Right;
diff Changes from Before to After;
project Selected from Combined { node "current-value-id"; edge "current-edge-id"; }
support State from Combined relation "affected"
  from entity "device-17" space "operations"
  to entity "advisory-9" space "knowledge" at 150;
```

Each declaration binds a reusable immutable graph value; inputs must already be declared and fully bound. Projection IDs refer to the current value (union namespaces IDs using pinned origins). Empty projection is valid. Union/diff preserve schemas or reject incompatible typed/untyped composition. Diff attaches added/removed membership labels without turning removal into negative evidence. Support uses explicit entity/space identities and a required valid-time instant. It yields a typed status graph with evidence derivations for supported/refuted/conflicted states; unknown has no evidence assertion. Coverage remains explicit, and absent evidence never implies falsity. See [the protocol semantics](contract/v0.5/README.md) and [the runnable example](../examples/algebra.weave).

## Pure graph functions (front-end specialization)

```weave
function Select revision "1" (graph input, string relation, time instant) {
  lens Result from input { match relation param relation; at param instant; }
  return Result;
}
function Transform revision "1" (graph input, function transform) {
  apply Result from transform { graph input input; }
  return Result;
}
apply AtFifteen from Select { string relation "affected"; time instant 15; }
apply Specialized from Transform { function transform AtFifteen; }
apply Output from Specialized { graph input Evidence; }
```

`apply` binds named arguments once. Incomplete application yields an immutable function value; complete application yields a graph value. Graph arguments are captured as immutable program-local graph bindings when supplied, including during partial application. Scalar arguments currently support strings and signed Unix-millisecond time values. Function arguments have one remaining graph parameter named `input` and return a graph. Graph parameter schemas are retained through evaluation; explicit schema-constrained signatures and runtime schema guards remain future work.

Bodies may compose input graphs and previously declared pure functions. They cannot read undeclared global graphs, declare sources or writes, recurse, or access host effects. Body variables are hygienically renamed. Function definitions are checked even when unused. Expansion is bounded to depth 32, 10,000 emitted statements and 4 MiB of serialized AST; exceeding a bound is an error rather than a partial executable plan.

Function revisions are explicit, nonempty source labels. Starting with protocol0.6, the compiler carries normalized source manifests into program/result identity. Function bodies specialize to the existing pure graph expressions. See [the runnable example](../examples/functions.weave) and `scripts/check_functions.py` for exact equivalence across literal, direct and higher-order evaluation.

## Explicit structural relations and source claims (protocol 0.6)

```weave
graph Evidence explicit {
  node "a" entity "device-17" space "operations";
  node "b" entity "advisory-9" space "knowledge";
  relation "relationship" from "a" to "b" predicate "affected";
  claim "inspection" on "relationship" source "inspection-system"
    polarity positive valid 10 until 20 property "method" "inspection";
  claim "assessment" on "relationship" source "assessment-system"
    polarity negative valid 10 until 30;
}
```

Use `explicit schema S` to validate structural relation types and endpoint properties. A `relation` can carry `type`, scalar properties and pinned metadata. A `claim` names its structural relation and carries separate source attribution, polarity, validity and properties. `context graph "C" revision "r"` can appear after its source label. Context is preserved, but consuming it in support or joins currently requires an unimplemented explicit selection rule and fails with `E_CONTEXT_REQUIRED`.

Named metadata uses `on edge "relationship"` for the structural relationship or `on assertion "inspection"` for one source claim. A structural-only graph yields no claim edges when queried. Legacy `edge` declarations remain supported in ordinary graphs; they cannot be mixed into an explicit graph. See [the example](../examples/assertions.weave) and `scripts/check_assertions.py`.

Function revision labels now travel with their normalized source digests in program and result manifests. `weave fingerprint FILE.weave` includes those revisions and the ordered expanded plan. Whitespace and comments do not change the normalized source identity; changing a function revision or bound argument does. These labels identify supplied code, and do not authenticate its author.

## Finite rule modules (protocol 0.7)

```weave
rules Reach revision "1" {
  rule Seed { when "link"(x, y); yield "reach"(x, y); }
  rule Step { when "reach"(x, y); when "link"(y, z); yield "reach"(x, z); }
}
reason Closure from Input using Reach;
```

Rule predicates are fixed strings; bare endpoint identifiers are variables, and quoted endpoints are exact current graph-value node IDs. Every head variable must occur in the body. `when negative "p"(x,y);` consumes explicit negative evidence; it never means that a positive claim is absent. `yield negative` produces an explicitly negative conclusion. `cross_space;` is required when a conclusion connects different spaces. Rule modules are immutable code and may be referenced by graph functions declared after them. They are not graph values.

This profile has a finite input-node domain, no term generation, and time intervals formed from premise intersections. Saturation deduplicates the same fact and leaf-premise set; independent support sets remain alternatives. It keeps a bounded canonical witness for recursive support instead of enumerating infinitely many cyclic proofs. Rules, bindings, source revisions and exact leaf premises remain in derivations. Exceeding host step, round, object, byte or alternative limits is an explicit error, never an empty or complete truncated answer. Partial input coverage stays partial. Rule results are graph values, with schemas and origins preserved, and can feed later operators without a hidden commit.

Contextual evidence still requires explicit context selection; this profile rejects its consumption. Stratified absence, complete-scope certificates, aggregations and arbitrary arithmetic recursion are not implemented. See [rules example](../examples/rules.weave) and [executable acceptance](../scripts/check_rules.py).

## Exact contexts (protocol 0.8)

```weave
context Local from Input default;
context Scenario from Input graph "World" revision "r1";
```

A default claim applies in the default context only. A pinned selection matches the complete graph/revision pair; selecting a world does not read its graph or grant access. The selected scope survives empty output. Selecting another world from an already selected value is rejected because discarded claims cannot be recovered; select again from the original value.

Named attachments may end with `context graph "World" revision "r1"`, after an optional `required` flag. Their applicability is independent data. Consuming a contextual attachment requires its exact scope. For the current profile, contextual metadata traversal also requires compatible target claim, attachment and scoped-node qualifiers and retains that scope; incompatible targets fail instead of being relabeled. Ordinary default navigation does not transfer the parent's selected world to independently qualified target content.

Selected support, joins and rules preserve the context on conclusions. A join cannot mix pinned and default scopes. Union retains individual qualifiers and clears its selected scope when inputs disagree; diff rejects mismatched selected scopes. Support status nodes—including unknown results—and explanation nodes retain generic `context_scope` qualifiers, so mixed unions remain filterable even without evidence edges. Ordinary structural nodes remain unqualified representations; selection retains them and filters only qualified derived nodes and claims.

This is exact context selection, not typed scenario axes, automatic compatibility, broadcasts, world conversion or governance. Those remain explicit design and implementation gates. [Context example](../examples/contexts.weave) and [executable acceptance](../scripts/check_contexts.py) exercise the current boundary.

## Geometry, structured literals and explanations (protocol 0.9)

```weave
distance Range from Positions assertion "a" to Positions assertion "b" at 7;
transform Moved from Positions assertion "a" using Mappings assertion "calibration" at 7;
project_axes Display from Embeddings assertion "sample" axes (0,1,2) revision "view-1" at 7;
explain Proof from Range;
```

Operands select positive asserted evidence already present in authorized graph values. Its `weave.geometry` assertion property is a structured object with a domain-validated `coordinates` or `rigid_transform` payload. Coordinates declare a complete space descriptor, vector role and finite numeric values. Their space ID must match the assertion's source manifestation space. A transform's source and target descriptors must match both endpoint spaces. Matching vector length alone is insufficient compatibility. See the complete [geometry example](../examples/geometry.weave).

Structured property literals support JSON-style objects and arrays, strings, booleans, null, signed integers and finite decimal/exponent numeric literals. Object keys are quoted and unique. Nesting is bounded to 32 levels, within existing source/token budgets. Schema `float` means finite IEEE 754 binary64, including its rounding behavior; it does not mean exact decimal arithmetic. Time selectors and interval boundaries still require signed 64-bit integer literals.

Distance outputs `measurement`; transform outputs reusable `coordinates`; projection outputs `navigation_projection` with original space and projection revision. Each result is an ordinary typed graph with assertion ID `value`, context, temporal interval and exact source premises. Display projections and measurements are rejected as coordinate inputs. A transformation produces a synthetic result identity and does not assert that two manifestations identify the same entity.

`explain` calls the portable graph-valued explanation operator. Its nodes and edges retain each conclusion's context and isolated alternative provenance; it reads no hidden graph or policy. It can appear in graph functions and feed normal graph operators. Its record proves derivation from recorded premises, not truth or authenticated remote execution.

Compile-time checks cover literal finiteness/nesting, graph binding, integer time and three distinct nonnegative projection axes. Runtime checks additionally validate complete geometry descriptors, payload roles/dimensions, frames, units, transforms, context, temporal applicability and current visibility. General vector/quantity schema types, typed spatial source declarations and learned mappings remain gaps.

Derived scalar and explanation nodes carry `derived_from` proof references as well as reader restrictions. Runtime persistence and query checks retain these dependencies, so clearing a result node's readers does not discard its recorded restrictions. Support summaries conservatively require all recorded input dependencies; a shared explanation conclusion requires all its alternatives, while individual explanation groups retain their own conjunction. This can deny a result even when one alternative remains available. A more permissive release policy is not implemented.

## Declared counterpart bridges (protocol 0.10)

```weave
counterparts Bridge from Evidence relation "counterpart" entity "device-17"
  from space "physical" to space "operations" at 7;
join Traversal from Bridge to Conditions relation "physical_affected";
```

The source graph must already contain visible positive directed bridge assertions at the selected instant. Both endpoints must declare the exact selected entity identity in distinct requested spaces; a matching bridge to a different identity is `E_IDENTITY`. The result retains its schema, endpoint state, assertion attribution, context, snapshot pins and proofs, and can feed ordinary graph functions and joins. Selection never retargets or synchronizes state. Reverse direction is a different query; intervals are half-open. Contextual evidence requires an exact prior context selection. Scoped derived endpoints cannot cross that selection.

Empty results and partial coverage retain their existing meaning without an external identity catalog lookup or hidden-match counts. A positive bridge remains positive evidence even if negative evidence also exists: this operator does not infer unique membership or policy acceptance. Accepted mappings between independently assigned identifiers, supersession/splits and private pairwise identities require the subsequent identity resolver. See [the runnable example](../examples/counterparts.weave).

## Exact decimals and nominal quantities (protocol 0.12)

```weave
schema Measures revision "1" {
  node Measure {
    property "exact" decimal required;
    property "length" quantity dimension "length" unit "metre" revision "1" required;
  }
}
graph Measurements schema Measures {
  node "n" type Measure entity "reading" space "lab"
    property "exact" decimal "9007199254740993"
    property "length" quantity "1.2300" dimension "length" unit "metre" revision "1";
}
```

Exact Decimal literals are quoted and explicitly prefixed; normalization produces canonical string values without passing through Float. Quantity literals contain an exact amount and nominal dimension/unit/revision. Schema validation requires that full descriptor. A familiar unit spelling does not establish compatibility, and ordinary numeric literals retain their existing Integer/Float semantics.

Pure literal operators are `decimal_add`, `decimal_sub`, `decimal_mul`, `decimal_div`, `quantity_add`, `quantity_sub`, `quantity_scale`, `quantity_div` and `quantity_convert`. Each takes two comma-separated literal operands in parentheses and evaluates during compilation. Nesting shares the structured-literal depth limit. Numeric errors are explicit diagnostics, with no rounding or Float fallback. Canonical string/object operands use the same representations as exact literals; a noncanonical string must first use an explicit Decimal literal to normalize it.

```weave
// Property values:
decimal_add(decimal "0.1", decimal "0.2")
quantity_scale(quantity "1.25" dimension "length" unit "metre" revision "1", decimal "2")
quantity_convert(quantity "3" dimension "length" unit "metre" revision "1", {
  "from": {"dimension_id":"length", "unit_id":"metre", "revision":"1"},
  "to": {"dimension_id":"length", "unit_id":"declared-third", "revision":"1"},
  "numerator": decimal "1", "denominator": decimal "3"
})
```

The last expression applies the caller's explicit rational factor; its labels do not establish a real-world conversion. This is pure arithmetic on authored literal data, not an authenticated conversion service over stored assertions. Graph-service conversion authority, time/context/provenance, affine offsets, quantity products/dimension algebra, general numeric function parameters and vector types remain separate requirements.
