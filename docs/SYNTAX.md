# Proposed bootstrap syntax

This is implemented experimental syntax, **not recovered white-paper syntax**. The compiler has no I/O authority beyond reading the source file in its CLI; output is a plan for the engine to validate and execute. Epoch values are signed Unix milliseconds. Intervals are half-open `[start, end)`.

```ebnf
program = { graph | use | lens | bind | join } ;
graph = "graph", identifier, "{", { node | edge }, "}" ;
node = "node", string, "entity", string, "space", string, { metadata }, ";" ;
edge = "edge", string, "from", string, "to", string,
       "relation", string, "valid", integer, "until", (integer | "infinity"),
       { metadata }, ";" ;
metadata = "metadata", "graph", string, "revision", string ;
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

A bootstrap `lens` produces a graph query. Composing one with an earlier lens combines compatible relation/time filters and inherits metadata options; conflicting filters are rejected. Named string/relation and time parameters use `param name`. A `bind` declaration specializes any subset with typed values. Unbound templates emit no query; fully bound lenses emit a query. Unknown, duplicate and incorrectly typed bindings fail compilation. This small filter language is **not yet general graph-valued functions or a rule evaluator**. Unknown operations, including writes or host authority fields inside a lens, fail parsing.

All bootstrap source-created nodes and edges are public because the grammar does not yet expose visibility declarations. The engine protocol supports restricted readers and must enforce host authority independently. Do not use this source subset to express a private graph until permission syntax is implemented; create such fixtures through the explicit protocol instead.

Metadata graph revisions are opaque references. The compiler does not pretend to verify their existence; the engine reports unavailable or depth-limited metadata as partial coverage. Cyclic metadata is never recursively inlined into the plan.

## Commands

```sh
cargo run -- check examples/fleet.weave
cargo run -- ast examples/fleet.weave
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

Join output is currently a terminal graph result and cannot be used as another graph source. Arbitrary cross-graph matching, user-defined identity alignment, higher-order graph parameters and recursive rules remain outside this implemented slice. Protocol [v0.2 notes](contract/v0.2/README.md) specify the runtime behavior.
