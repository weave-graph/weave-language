# Time travel, traversal and change sets

Introduced by LA1 part 1 ([#3](https://github.com/weave-graph/weave-language/issues/3))
on protocol **0.23.0**. The source syntax lowers one to one to the engine A3
operators: the `as_of`, `expand` and `changes` graph expressions
(engine [TRAVERSAL.md](https://github.com/weave-graph/weave-engine/blob/main/docs/TRAVERSAL.md),
core v1 §5 `time.as_of` and §8 `query.expand` and `query.changes`).
`weave describe FILE --contract` classifies all three as core v1.

```weave
use Known graph "fleet" as_of 1759000000000;     // what the store knew at K
use Fleet graph "fleet";                         // the branch head

lens Located from Known { match relation "located_at"; at 1758000000000; }

expand Near from Known {
  seeds "unit-7", "unit-8";
  hops 2;
  direction both;
  match relation "installed_at", "runs";
  at 1760000000000;
  limit 500;
}
explain Why from Near;

changes Delta from Fleet { between as_of 1759000000000 and head; entities "UNIT-7"; }
changes Feed from Fleet { since cursor "fact:sha256:…"; }
```

## Recorded time and valid time

Two time axes stay separate, as in the engine:

- **Recorded time** (`as_of T`): the branch as this replica knew it at engine
  recorded time `T` (Unix milliseconds). It is a property of the graph
  reference, written on `use`.
- **Valid time** (`at T`): what was true in the world at `T`. It is a filter
  clause, written on `lens` (as before) and `expand`.

The compiler never supplies recorded time; `as_of` only selects one. The
engine resolves it through its recorded-time index, fails with
`E_HISTORY_TIME` for a time after its trusted clock, and authorizes the
selected revision record by record.

## `use ... as_of`

```ebnf
use = "use", identifier, "graph", string, [ "branch", string ],
      [ "revision", string | "as_of", integer ], ";" ;
```

`branch` defaults to `main`. `revision` pins an exact revision; `as_of` reads
the branch as known at a recorded time. Giving both is `E_HISTORY_SELECTOR`
at the second clause.

A `lens` over an `as_of` reference lowers to
`{kind: "as_of", query: QueryPlan, unix_millis}`, with the lens's
`match relation` and `at` filters in the query. Its result equals the same
query pinned to the revision selected at that time. It differs from
`recorded_handle ... known_at` (protocol 0.20), which lowers to
`recorded_query`: that form requires the reader to see the whole selected
revision and returns an observation witness for replay; `as_of` returns the
visible subset and no witness. See [history selection](HISTORY_SELECTION.md).

## `expand`

```ebnf
expand = "expand", identifier, "from", identifier, "{", { expand_clause }, "}" ;
expand_clause = "seeds", string, { ",", string }, ";"
              | "hops", integer, ";"
              | "direction", ( "out" | "in" | "both" ), ";"
              | "match", "relation", string, { ",", string }, ";"
              | "at", integer, ";"
              | "limit", integer, ";" ;
```

The exact k-hop neighborhood of the seed node IDs: every node within `hops`
hops of a visible seed along visible edges whose predicate is listed
(`match relation`; absent follows every predicate) and that are valid at `at`
(when given), plus every such edge leaving a node fewer than `hops` hops away.

| Clause | Required | Lowers to | Bound |
|---|---|---|---|
| `seeds` | yes | `seeds` | 1 to 1,000 IDs of 1 to 512 bytes |
| `hops` | yes | `hops` | 1 to 16 |
| `direction` | no | `direction` (default `out`) | `out`, `in` or `both` |
| `match relation` | no | `predicates` | at most 1,000 |
| `at` | no | `valid_at` | signed Unix ms |
| `limit` | no | `limit` (default 10,000) | 1 to 200,000 nodes plus edges |

The source must be a stored graph reference: a `use` declaration or a
`graph` declaration, with no lens filters applied (`E_EXPAND_SOURCE`). Its
graph, branch and `revision` or `as_of` become the expansion's own fields, so
`expand ... from Known` above reads the branch as known at K. A larger result
than `limit` fails with `E_BUDGET` in the engine; it is never truncated.

The result is an ordinary graph value. Later `lens`, `union`, `diff`,
`project`, `support`, `join`, `window`, `explain` and pure functions take it
as input like any other.

## `changes`

```ebnf
changes = "changes", identifier, "from", identifier, "{", { changes_clause }, "}" ;
changes_clause = "between", bound, "and", bound, ";"
               | "since", bound, ";"
               | "entities", string, { ",", string }, ";"
               | "match", "relation", string, { ",", string }, ";"
               | "limit", integer, ";" ;
bound = "as_of", integer | "cursor", string | "head" ;
```

The facts asserted, retracted or corrected between two states of the branch,
as a change set in `QueryResult.changes`. `since B` means
`between B and head`. Exactly one of them is required (`E_CHANGES_BOUND`).

| Bound | Lowers to | Meaning |
|---|---|---|
| `as_of T` | `{kind: "as_of", unix_millis: T}` | The branch as known at recorded time T; empty before the branch existed |
| `cursor "R"` | `{kind: "cursor", revision: "R"}` | An exact revision, typically the `to_revision` of an earlier change set |
| `head` | `{kind: "head"}` | The branch head when evaluated |

`entities` keeps changes about those entities; `match relation` keeps edges
with those predicates and attachments with those keys. `limit` (default
10,000, at most 200,000) bounds the entries; more fails with `E_BUDGET`.

The source must be an unfiltered stored graph reference without `revision`
or `as_of`, because the bounds select the states (`E_CHANGES_SOURCE`).

**A change set is terminal.** No declaration may take it as input: using it
in `lens`, `explain`, `union`, `join`, `expand`, a graph function argument or
any other operator is `E_CHANGES_OPERAND` at the reference, the same rule
and code as the engine. To continue a pull feed, read `to_revision` from the
result and pass it as `cursor "R"` in the next program.

## `explain`

`explain Name from Value;` (protocol 0.9) works over `expand` and `as_of`
values unchanged and lowers to `{kind: "explain", input}`. Results of 0.23
programs also carry `fact_provenance` for every cited source fact: the
engine-recorded writer and recorded time plus the claimed provenance. A
change set cannot be explained (it is terminal); its entries' `before` and
`after` facts are already listed in its `fact_provenance`.

## Diagnostics

| Code | Cause | Span |
|---|---|---|
| `E_HISTORY_SELECTOR` | `use` with both `revision` and `as_of` | The second clause |
| `E_EXPAND` | `expand` without `seeds` or `hops`, or `hops 0` | The name, or the number |
| `E_BUDGET` | More than 16 hops, 1,000 seeds, 1,000 relations or entities; a limit outside 1 to 200,000 | The number or clause |
| `E_ID` | An empty ID, or one over 512 bytes | The string |
| `E_DUPLICATE` | A clause given twice | The clause |
| `E_PARSE` | An unknown clause, direction or bound | The token |
| `E_EXPAND_SOURCE`, `E_CHANGES_SOURCE` | The source is not an unfiltered stored graph reference; for `changes`, also one with `revision` or `as_of` | The source name |
| `E_CHANGES_BOUND` | `changes` without `between` or `since` | The closing brace |
| `E_CHANGES_OPERAND` | A change set used as an input | The reference |
| `E_FUNCTION_EFFECT` | `expand` or `changes` inside a pure function | The declaration name |
| `E_NOT_YET_SUPPORTED` | `nearest` or `similar_candidates` (engine A4) | The keyword |

## Examples and acceptance

- [examples/traversal](../examples/traversal): four sources that mirror the
  engine core v1 fixtures `expand-k-hop`, `expand-valid-time-as-of`,
  `changes-between-times` and `as-of-partially-visible`, and
  `fleet_questions.weave`, Phase A exit question 2 in 14 lines.
- `tests/traversal.rs`: golden source-to-plan tests, strict
  `decode_program` round trips, equality with the fixtures' expressions,
  core v1 classification and every diagnostic above.
- `scripts/check_traversal.py --compiler PATH --engine PATH`: runs the
  fixtures' setup on a real engine, executes the compiled sources and
  compares every result with the fixture's recorded result and with the
  fixture's own JSON plan. It then answers question 2 over a fleet written
  with fact writes, as known at K and at the head, for an analyst and an
  admin, and checks the JSON reference plan, an independent oracle,
  restricted-unit hiding, fact provenance, `explain` and a replay in a fresh
  process.

## Limits

- Seeds, relations, entities, times and limits are literals. They are not
  yet function parameters, scalar values or `bind` parameters.
- `expand` and `changes` read the store, so they are top-level declarations
  only: they are rejected inside pure functions, handler and view recipes
  (the engine rejects all three operators in views and handlers too).
- No intersection operator exists, in the engine or the language. Question 2
  ("which units at site S ran firmware F") is answered by one expansion that
  holds the units at S, the firmware each ran and the supporting edges; the
  units running F are those with a `runs` edge to F in that result.
- Fact-level writes (assert, correct, retract) still have no source syntax.
- **Hybrid search is not yet supported.** `nearest` and `similar_candidates`
  are reserved declaration keywords that fail with `E_NOT_YET_SUPPORTED`.
  The engine A4 operators exist since protocol 0.24.0
  ([weave-engine#13](https://github.com/weave-graph/weave-engine/issues/13)).
  They will be added as further `expand`-style
  declarations over a stored reference (LA1 part 2).
