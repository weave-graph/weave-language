# Fact writes, embeddings and hybrid retrieval

Introduced by LA1 part 2 ([#3](https://github.com/weave-graph/weave-language/issues/3))
on protocol **0.24.0**. The source syntax lowers one to one to the engine A2
`write` command and the A4 operators `nearest` and `similar_candidates`, and
adds `seeds from` to `expand`
(engine [EMBEDDINGS.md](https://github.com/weave-graph/weave-engine/blob/main/docs/EMBEDDINGS.md),
core v1 §7 and §8 `query.nearest` and `query.similar_candidates`).
`weave describe FILE --contract` classifies all of it as core v1.

```weave
embedding_space Notes id "hw-notes" revision "1" encoder "text-embedding-3-small"
  version "2024-01-25" preprocessing "weave-chunk/512-tokens/v1" dimensions 3 metric cosine_distance;

write Rollout graph "fleet" {
  correct edge "runs:unit-2:0" from "unit-2" to "fw-2" relation "runs" valid 0 until 2000;
  assert edge "runs:unit-2:1" from "unit-2" to "fw-3" relation "runs" valid 2000 until infinity;
  assert space Notes;
  assert embedding "emb:note-1" in Notes subject node "note-1" vector [0.9, 0.1, 0.0] valid 100 until infinity;
}

use Fleet graph "fleet";
expand Region from Fleet { seeds "unit-7"; hops 2; direction both; }
nearest Similar from Fleet { space Notes; like embedding "emb:incident-7"; k 10; at 1000; candidates Region; }
expand Units from Fleet { seeds from Similar; hops 1; match relation "performed_on"; }
similar_candidates Pairs from Fleet { space Notes; k 1; max_score 0.2; exclude_linked; }
```

## Embedding spaces

```ebnf
embedding_space = "embedding_space", identifier, "id", string, "revision", string,
                  "encoder", string, "version", string, "preprocessing", string,
                  "dimensions", integer, "metric", ( "cosine_distance" | "euclidean" ), ";" ;
```

A top-level declaration of the complete, immutable `EmbeddingSpace`
descriptor (`version` is `encoder_version`). It is checked with the
contract's own validation (`E_SPACE`). One program cannot declare the same
`id`/`revision` with two descriptors (`E_SPACE_MISMATCH`). The declaration
itself emits nothing. `assert space Notes;` in a `write` writes it as a
fact, and `space Notes;` names it in `nearest` and `similar_candidates`,
which lower to its `{id, revision}`.

**The type checker enforces the descriptor.** Every vector is checked
against the declared space at compile time, with `E_SPACE_MISMATCH` at the
exact span of the vector, value or ID:

- a literal vector (in `nearest` or a written embedding) needs exactly
  `dimensions` finite components, and a cosine space rejects the zero vector;
- a typed query value (`vector value Q`, where `Q` has a
  [typed vector](TYPED_VECTORS.md) type) must have the same space `id`,
  `revision`, `encoder`, `preprocessing`, `dimensions` and `metric`. Equal
  dimensions never suffice;
- a query fact (`like embedding "e"`) that this program wrote into another
  space is rejected.

The engine checks again against the space stored in the graph (`E_SPACE`).
The compiler cannot see the store, so a space declared differently in
source and store fails there.

## Fact writes

```ebnf
write = "write", identifier, "graph", string, [ "branch", string ],
        [ "expected", ( "head", string | "new" ) ], "{", change, { change }, "}" ;
change = ( "assert" | "correct" ), fact, ";"
       | "retract", ( "node" | "edge" | "attachment" | "embedding" ), string, ";" ;
fact = "node", string, "entity", string, "space", string, [ interval ], { metadata | property }
     | "edge", string, "from", string, "to", string, "relation", string,
       [ "polarity", ( "positive" | "negative" ) ], interval, { metadata | property }
     | "space", identifier
     | "embedding", string, "in", identifier,
       "subject", ( "node" | "entity" | "edge" ), string, "vector", vector, interval ;
interval = "valid", integer, "until", ( integer | "infinity" ) ;
```

Each `write` lowers to one `write` command, which is one atomic fact
revision. `assert` adds a fact that is not live. `correct` replaces the live
version of the fact with that kind and ID, so it restates the whole fact.
`retract` ends the live version; history keeps every version. `expected head
"r"` requires that head, `expected new` requires a new branch, and omitting
it performs no head check. A node's optional `valid` interval is node valid
time (0.22). The engine owns the writer and the recorded time; the compiler
never supplies either. An embedding space is immutable, so `correct space`
is rejected (`E_CONFLICT`); declare a new revision instead.

A write is not a graph value. Its name is a declaration name only, and a
later operator cannot take it as input.

## `nearest`

```ebnf
nearest = "nearest", identifier, "from", identifier, "{", { nearest_clause }, "}" ;
nearest_clause = "space", identifier, ";"
               | "vector", ( vector | "value", identifier ), ";"
               | "like", "embedding", string, ";"
               | "k", integer, ";"
               | "at", integer, ";"
               | "candidates", identifier, ";" ;
```

Exact k nearest embeddings of the space to the query: a literal vector, a
typed vector value, or a visible embedding fact (`like embedding`, whose own
subject is excluded). `k` is 1 to 1,000. `at` keeps embeddings valid at that
instant. `candidates V` restricts the ranking to the subjects of the graph
value `V` (its nodes, their entities and its edges), so
`candidates Region` with `expand Region ...` is graph-to-vector retrieval.
The source is a stored reference, as for `expand`: its revision or `as_of`
recorded time selects the state. The result is a graph value with
`neighbors`, `exactness: exact` and `fact_provenance` for every hit.

## `expand ... seeds from`

`seeds from V;` adds the nodes of the value `V` (by origin in the graph) to
the seeds. `seeds` may then be omitted. `expand Units from Fleet { seeds
from Similar; ... }` is vector-to-graph retrieval.

## `similar_candidates`

```ebnf
similar_candidates = "similar_candidates", identifier, "from", identifier, "{",
                     { candidates_clause }, "}" ;
candidates_clause = "space", identifier, ";" | "k", integer, ";"
                  | "max_score", number, ";" | "at", integer, ";"
                  | "sources", identifier, ";" | "targets", identifier, ";"
                  | "exclude_linked", ";" | "limit", integer, ";" ;
```

Candidate relations `weave:similar_to` between visible embeddings: for each
source embedding (all of the space, or the subjects of `sources`), its `k`
nearest targets (all, or `targets`) at most `max_score` apart.
`exclude_linked` keeps only pairs not already joined by a visible edge. The
`limit` defaults to 1,000 and is at most 100,000. Candidates are result
values, never facts. To persist one, write an ordinary edge.

`candidates`, `sources`, `targets` and `seeds from` take any fully bound
graph value, but not a change set, which is terminal (`E_CHANGES_OPERAND`). They lower to a reference to the earlier binding, so
the value is the one bound before, in program order. The engine's JSON
examples inline the expression instead. Both forms give the same result, and
the acceptance scripts check this.

## Diagnostics

| Code | Cause | Span |
|---|---|---|
| `E_SPACE_MISMATCH` | A vector, typed vector value or query fact that does not fit the declared space; one space with two descriptors | The vector, value, ID or declaration name |
| `E_UNKNOWN_SPACE` | A space name that is not declared | The name |
| `E_SPACE` | An invalid descriptor (empty label, `@` in the revision, dimensions outside 1 to 4,096) | The declaration name |
| `E_NEAREST` | `nearest` without `space`, a query and `k`; `similar_candidates` without `space` and `k`; `k 0`; a negative `max_score` | The name or the number |
| `E_BUDGET` | `k` above 1,000, a candidate `limit` above 100,000 | The number |
| `E_NEAREST_SOURCE` | A source that is not an unfiltered stored graph reference | The source name |
| `E_VECTOR_VALUE` | A vector that is not a nonempty list of numbers | The list |
| `E_SCALAR_TYPE` | `vector value X` where `X` is not a typed vector | The name |
| `E_CONFLICT` | `correct space` | `space` |
| `E_INTERVAL` | An empty valid interval | The change |
| `E_FUNCTION_EFFECT` | `write`, `nearest`, `similar_candidates` or `embedding_space` inside a pure function | The name |

## Phase A exit questions

[examples/phase_a](../examples/phase_a) answers each exit question of the
[engine roadmap](https://github.com/weave-graph/weave-engine/blob/main/docs/ROADMAP.md#phase-a-rock-solid-shared-core)
in at most 15 lines. `scripts/check_phase_a.py --compiler PATH --engine PATH`
runs all five on a real engine over a synthetic fleet:

1. `q1_fact_writes.weave`: a rollout and a correction. The script applies
   1,000 updates as individual source writes (1,001 fact revisions), checks
   them against the same writes made in JSON, and compares the head for two
   readers.
2. `q2_as_known.weave`: the units at a site with their firmware, as known at
   K, and what changed for a unit. Checked against the JSON reference.
3. `q3_hybrid_retrieval.weave`: `nearest` over the 2-hop `expand` at the
   incident time. Checked against an independent ranking, the JSON
   reference (with the expansion inline), the hits' provenance and a replay.
4. `q4_candidate_relations.weave`: `similar_candidates` with
   `exclude_linked`. Checked against an independent nearest-pair oracle, the
   JSON reference and the candidates' embedding provenance.
5. `q5_restricted.weave`: a restricted unit is invisible to the analyst in
   `expand`, `similar_candidates`, `seeds from` and `changes`. For the
   analyst, `nearest` on the restricted unit's own query embedding fails
   exactly as a missing embedding does. The script also checks that no
   restricted ID appears in any answer to questions 2 to 4.

The embeddings come from a deterministic word-hashing embedder in the script
(the approach of the engine's `fleet_hybrid.rs`). Vectors are data written
by the caller; the language has no encoder.

`examples/hybrid` mirrors the engine core v1 fixtures
`embedding-write-and-nearest`, `nearest-composition` and
`similar-candidates`. `tests/hybrid.rs` checks that they compile to the
fixtures' expressions, and `scripts/check_traversal.py` checks the results
on a real engine.

## Limits

- **Writes are public.** Source writes cannot set `readers`, gates or
  claimed provenance; permission syntax is LB1 (Phase B). Correcting a
  restricted fact from source would therefore make its new version public.
  Write restricted data through the protocol.
- Per-fact `expected` concurrency (`absent`, `unchanged_since`) and
  attachment and structural-relation facts are not yet in the write syntax.
  Retraction accepts attachments.
- Vectors are literals or typed vector values. Typed vector types carry no
  `encoder_version`, so that field is checked only by the engine.
- `nearest`, `similar_candidates` and `write` are top-level declarations
  only. The engine rejects the A4 operators in view and handler recipes.
- Hybrid retrieval is exact (the engine's exhaustive ranking); approximate
  search is engine C5.
