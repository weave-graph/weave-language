# Graph windows and temporal sequence witnesses

Historical accepted design for the bounded protocol 0.19 implementation. See
[the implemented profile](../TEMPORAL_QUERIES.md) for current semantics and limits.
This is a bounded L05 slice after scalar Interval. The language paper §5 requires
simultaneous conclusions to intersect supporting intervals and the query window,
but requires sequence queries to use an explicit temporal relation instead.
It also distinguishes valid time, replica-local recorded time, revision ancestry
and acceptance time. This proposal implements valid-time computation over pinned
graph values; it does not manufacture the other clocks.

Follow-up review: the match-attachment representation below is superseded by
[Temporal attachment proof handoff](TEMPORAL_ATTACHMENT_PROOFS.md). Generated match
details belong in occurrence derivation parameters. Real metadata navigation and
empty-target protection require the coherent alternative-carrier design there;
the experimental pure module is not an approved release of the earlier shape.

## Existing implementation and the missing seam

`QueryPlan.revision` selects an exact stored graph revision; `valid_at` selects a
point. Native query authorizes before materialization/filtering, captures a local
read transaction and returns exact `input_snapshots`. Graph-valued function
arguments are already evaluated values and require no hidden read. Native
`Engine::join_values` and portable `rules::evaluate` intersect premise intervals;
neither can implement a non-overlapping sequence by adding a relation parameter.
Portable `Interval` supplies the existing `{start:i64,end:Option<i64>}` wire shape;
the source's checked Interval remains a distinct scalar type.

Clipping an original assertion's interval while retaining its immutable origin
would misrepresent its payload. Window outputs therefore require derived wrappers,
not an in-place `retain` plus interval edit. Likewise, selecting a public event
only because it has a private sequence partner cannot leave an ungated public
wrapper when the result envelope or match metadata is removed.

## Minimum proposed contract

Two new graph expressions, with pure helpers in a new canonical `temporal.rs`:

```rust
Window { input: Box<GraphExpression>, window: Interval }
Sequence {
    left: Box<GraphExpression>, right: Box<GraphExpression>,
    window: Interval, relation: TemporalRelation, match_on: JoinMatch,
}
// strict snake_case enum; no implicit simultaneous mode
TemporalRelation = Before | Meets | Overlaps | Within
```

All intervals are validated before work or cloning, including Rust-constructed
DTOs. No new scalar schema, observer credential, clock, network operation or
automatic revision lookup is added. Existing `JoinMatch::EntitySpaceToFrom` is the
only match key in this profile. Predicate selection uses existing Filter/lenses;
Sequence consumes positive assertions only, just as the current path Join does.
Negative evidence remains available in the input and Window preserves polarity;
no absence or conflict-resolution inference is introduced.

### Window

For each edge, intersect its interval with the half-open window. Disjoint or
touching intersections omit that edge. Retained assertions get deterministic
derived IDs and clipped intervals, the same predicate, polarity and typed payload,
and explicit `weave:window/v1` derivations recording both original interval and
window. Original source and structural identities remain proof/descriptive
references; changed assertion payloads must not claim original-object identity.
Repeated identical windows are semantically idempotent; plan identities may still
describe different compositions.

Use source alternatives rather than flattening their OR groups into an AND.
Retained endpoint wrappers bind source NodeRefs and the membership proof. Untimed
isolated nodes can remain structural representations, with their existing gates;
their presence does not assert validity throughout the window. This choice must be
explicit in the docs and tests, not inferred from `valid_at`'s endpoint pruning.

Attachments retain their meaning and source dependencies. Intersect attachment
validity with the window too; changing its host/interval makes it a derived
attachment. Carry original attachment premises plus the host's membership gates;
do not preserve an original attachment identity for a changed payload. Graph,
entity, node and assertion/edge hosts need separate closure tests. Resolved
metadata snapshots stay exact and unmodified; their contents are not recursively
windowed and live metadata is never refreshed by this pure operator. Attachment
alternatives require distinct bounded witnesses rather than flattening OR proofs.

Then an ordinary Join of two windowed values has the required simultaneous
intersection. No changes to existing Join semantics are necessary.

### Sequence

First require that both input occurrence intervals intersect the selection
window. Evaluate the requested relation on their input intervals, **before**
clipping: clipping must not turn an originally non-contained pair into `within`.
For an input already produced by Window, its clipped intervals are its explicit
input intervals; Sequence does not silently recover an earlier temporal scope.

Relations match the source scalar profile: `before` means finite left end less
than right start; `meets` means equality; `overlaps` means symmetric nonempty
intersection; `within` means left interval contained in right, including equality.
Use comparisons, never subtraction or end-plus-one, at all i64 boundaries.

Return a graph of the matching occurrence wrappers and protected graph-host
literal match witnesses. Each occurrence keeps its own clipped interval and
predicate. No new domain predicate spans the gap, and no convex hull is asserted
as simultaneous support. Each selected occurrence wrapper depends on **both**
members of its match, so copying it without the envelope cannot disclose a match
whose other member is private. This is an evidence-selection result, not a newly
inferred application-domain relation.

A witness identifies its left/right current result IDs, original/input intervals,
clipped intervals, relation, match key, window and operator version. Its validity
is the selection window of the report, not an assertion that both events occurred
throughout that window. One witness per authorized alternative combination uses
existing attachment AND gates. Alternatives are not collapsed into one mandatory
proof set. IDs include semantic parameters, exact evidence, context and role,
with stable canonical content rather than recursive rehashing of wrapper IDs.
No arbitrary user literal with this shape acquires trusted derivation semantics.

The minimum result does not promise a new source API for filtering arbitrary
literal witness fields. Ordinary graph composition, persistence and typed literal
inspection remain available. A dedicated match-schema/query API would be a later
extension, not silently included here.

## Observation, provenance and restrictions

Initial acceptance uses explicit graph revision pins for every stored input.
The executing host is the observing replica; host integration must record its
configured replica identity alongside the exact input revision vector. A source
string cannot authenticate or select a remote replica. Current protocol results
do not contain an authenticated replica identity, so a portable recorded-time
`known_at replica(...)` selector requires a separately reviewed host binding and
is **not implemented by these two expressions**. Replaying on another authorized
host means the same pinned data observed there under its current policy, not a
claim about what another replica knew at a wall-clock instant.

No globally atomic multi-peer snapshot claim follows from `input_snapshots`.
Historical pins continue current policy/source authorization. Missing/private
inputs preserve generic unavailable/Partial behavior without hidden match counts.
Two selected contexts must be compatible; contextual assertions require explicit
selection. Retain full typed witnesses, source manifests, diagnostics and merged
declared influence even when no match remains. Never promote descriptive
`input_snapshots` to whole-snapshot authorization gates.

Every generated record receives the existing assertion/node/snapshot restrictions
and principal scope. Parent operator parameters and alternative derivations remain
intact, including explicit assertion/structural identity separation. Ordinary
retained originals are unchanged. Schema transfer must preserve exact descriptors
or reject incompatibility; it must not erase schemas to make the operator fit.

## Source and artifact integration

Proposed syntax, subject to canonical naming review:

```weave
function During revision "1" (graph input, interval span) {
  window Result from input during param span;
  return Result;
}
window Active from Evidence during interval(time 10, time 30);
sequence Events from Started to Finished before
  during interval(time 0, time 100);
```

Both statements produce ordinary graph bindings. Sequence uses the fixed explicit
entity/space endpoint match above; no application-domain output relation is
invented. Interval expressions support values, parameters, partial application and
pure imports. Existing source qualification rewrites binding names only, preserving
selector strings and original module diagnostics. Unused bodies undergo symbolic
type and closed-bound checking. Scalar-returning functions still cannot read graphs.

Engine owns the eventual protocol enum/version, native recursive evaluation,
pre-write old-version rejection, provenance validation and persistence. Language
owns the portable module by coordinated handoff, parser/function/module transfer,
SDK parity and actual fixture. Artifact visitors/fingerprints must handle both
operators. Existing compiled view templates stay Query/Filter-only; do not imply
incremental window refresh or rolling windows. Handler recipe acceptance requires
explicit owner review and recursive validation, rather than accepting new enum
variants accidentally. Existing serialized descriptors and artifact identities
remain byte-identical under their historical protocol.

## Bounds and acceptance

Precharge checked candidate-pair products (at most current Join's 1,000,000),
alternatives (at most existing 128), shared object/output byte budgets and combined
1,000 proof references before copying or retaining output. These are ceilings,
not promised capacities: lower host/module budgets still apply. No truncation to
an apparently complete result. Errors include E_INTERVAL_BOUNDS for invalid DTO
bounds, E_TEMPORAL_BUDGET for work/output exhaustion, existing context/schema/proof
errors, and original-span E_SCALAR_TYPE diagnostics in source.

Required fixtures before approving a coherent freeze:

- Disjoint `[0,5)` then `[10,15)` matches before but ordinary Join emits nothing;
  `[0,5)`/`[5,10)` matches meets, not before or overlaps. Open ends and both i64
  extremes use finite-set/comparison oracles with no arithmetic overflow.
- `[0,20)` and `[10,30)` within window `[12,18)` remain not-within despite equal
  clipped intervals. Explicit prior Window changes inputs and is tested separately.
- Simultaneous windowed Join equals intersection of both original intervals and
  the window. Negative claims clip without becoming positive; four-valued support
  retains its existing meaning. Structural isolated nodes follow the stated rule.
- Legacy/explicit assertions, all attachment hosts, metadata cycles, schema
  conflicts, exact contexts, repeated Window/union/diff, alternatives and Explain
  keep origin integrity. Strip envelopes/readers/companions and repersist each
  generated node/edge/attachment: a withheld partner still hides match membership.
- Empty results retain declared influence and private typed descriptors. Revocation,
  unavailable premises, capsule transfer and restart cannot release protected output.
- Late correction creates a new exact revision: old/new pinned computations differ
  predictably while historical reads still enforce current authority. Host receipts
  name the observing replica; no universal recorded-time/acceptance-time claim.
- Compiler→native persistence/restart, strict recursive old-wire rejection with
  multi-command rollback, SDK native/zero-import WASM exact plans/diagnostics,
  pure operator native/WASM execution parity, modules/formatter/partial functions
  and budget failures. Preserve all pre-extension artifacts under old versions.

This advances L05 valid-time windows and bounded binary sequence selection.
Recorded-time range queries/corrections, authenticated cross-replica observation,
calendar literals, arbitrary sequence patterns and incrementally maintained rolling
windows remain explicit gaps.

## Parser checkpoint (not executable temporal support)

The source AST accepts `window W from G during interval(time 0, time 10);`
and `sequence S from L to R before during param span;`, with `meets`, `overlaps`
and `within` as the other relation tokens. Scalar Interval inference and checked
constant validation apply even inside unused functions. Graph function expansion
captures both operands through the existing pure bindings, substitutes Interval
parameters and renames local results hygienically. Window preserves known schema;
Sequence requires exact complete descriptors when both inputs are statically known.
Module diagnostics retain the original file and operand/relation spans.

Fully instantiated temporal operations currently return `E_TEMPORAL_PROTOCOL`;
no plan or SDK success artifact is returned. Unused definitions and partial
applications are inspectable and do not read a store. The parser fixture is
[windows.weave](temporal-fixtures/windows.weave), deliberately outside executable
examples. Six integration tests plus an internal expansion test cover these
boundaries, alias/whitespace identity, literal preservation and formatter
idempotence. The focused module/formatter/Interval regression batch passed 33
checks; the additional expansion test passed separately. Strict workspace/all-target
Clippy and formatting checks passed. No runtime or temporal execution claim follows
from this syntax evidence.

Temporal metadata acceptance will use current returned wrapper IDs. Derived
node, edge and attachment IDs do not create aliases for original local IDs or
entity/space identity. A native fixture may inspect the returned ID and then issue
explicit navigation; the existing narrow original-node shorthand is unchanged.
