# Proposed bootstrap syntax

This is implemented experimental syntax, **not recovered white-paper syntax**. The compiler has no I/O authority beyond reading the source file in its CLI; output is a plan for the engine to validate and execute. Epoch values are signed Unix milliseconds. Intervals are half-open `[start, end)`.

```ebnf
program = { schema | transaction | graph | use | lens | bind | join | metadata_query } ;
transaction = "transaction", identifier, "{", graph, { graph }, "}" ;
metadata_query = "metadata", identifier, "from", identifier, "on", host, "key", string, ";" ;
host = ("node" | "edge" | "entity"), string | "graph" ;
schema = "schema", identifier, "revision", string, "{", { node_schema | edge_schema }, "}" ;
node_schema = "node", identifier, [ "space", string ], shape ;
edge_schema = "edge", identifier, "from", identifier, "to", identifier, [ "cross_space" ], shape ;
shape = "{", { "property", string, scalar_type, ("required" | "optional"), ["nullable"], ";" | "open", ";" }, "}" ;
scalar_type = "string" | "integer" | "boolean" ;
graph = "graph", identifier, [ "schema", identifier ], "{", { node | edge | attachment }, "}" ;
node = "node", string, [ "type", identifier ], "entity", string, "space", string, { metadata | property }, ";" ;
edge = "edge", string, [ "type", identifier ], "from", string, "to", string,
       "relation", string, [ "polarity", ("positive" | "negative") ], "valid", integer, "until", (integer | "infinity"),
       { metadata | property }, ";" ;
metadata = "metadata", "graph", string, "revision", string ;
attachment = "attachment", string, "on", host, "key", string, "graph", string,
             "revision", string, "valid", integer, "until", (integer | "infinity"), [ "required" ], ";" ;
property = "property", string, (string | integer | "true" | "false" | "null") ;
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

Nodes and edges accept repeated `property "key" value` clauses for scalar metadata. Values are strings, signed integers, booleans or null; property keys must be unique and nonempty. Graph-valued metadata continues using `metadata graph ... revision ...`. Scalar strings are data, not executable code. Floats, arrays and structured scalar objects are not in this grammar yet.

## Named metadata and cyclic local transactions

[metadata_cycle.weave](../examples/metadata_cycle.weave) creates two graph snapshots inside one explicit `transaction boot { ... }`. An `attachment` has its own identity, host kind/identity, key, pinned graph value and valid interval. `required` demands that the runtime validate the referenced snapshot's availability. The source profile currently emits public attachments without custom origin/schema-revision fields; those richer runtime fields remain future source syntax.

The transaction lowers to one atomic `CommitBatch`. A same-batch graph revision is written as `logical:batch-id:graph-id`; the runtime resolves those identities through one snapshot manifest and validates required references after inserting all members. This allows real cyclic references without recursive hashes. The batch ID is an ASCII identifier at most 64 bytes. Nested transactions, empty batches and non-graph statements inside a batch are rejected. Graph declarations currently create new snapshots; source-level expected-head updates/rebinding remain later work.

A query resolves named metadata with `metadata depth N;`. `metadata Proof from Result on edge "connection" key "evidence";` selects the resolved graph as another immutable graph value. It preserves attachment-path provenance and visibility. Missing, ambiguous or unmaterialized metadata yields explicit incomplete coverage from the engine, not fabricated emptiness. It performs no hidden live read. That value can feed ordinary lenses and joins.

Legacy anonymous `metadata graph ... revision ...` remains available for compatibility. Named attachments are the path to the full paper model. Live handles, typed graph-valued schema fields and independently specified attachment origin/policy fields are not yet exposed in source.

## Versioned schemas and discovery

[Schema example](../examples/schema.weave) declares a versioned graph schema with distinct node and edge types. A graph selects it with `schema Name`; each node/edge declares `type Name`. Schemas must be declared before use. Definitions state property types and presence/nullability, optional exact node spaces, and edge endpoint types. Cross-space edges require explicit `cross_space`; undeclared scalar properties require explicit `open;` in the type's shape.

`weave describe FILE` validates the file and emits its source schema descriptors as JSON without executing a plan. This is source-level discovery, not a privileged inventory of remote graphs. The engine's graph results carry their own schema descriptor; runtime authorization still governs access.

Both compiler and direct runtime clients use the same portable validator. Supported scalar schema types are string, signed 64-bit integer and boolean; nullable values are explicit. Exact decimal, vector/quantity, graph-valued metadata field schemas, variance and disconnected migrations remain open. The grammar's schema revision is an explicit identity, not a claim that a field rename or unit change is safe.
