# Graph windows and bounded sequences

Protocol 0.19 adds two pure graph operators over already selected graph values:

```weave
window Recent from Events during interval(time 10, time 30);
sequence Followed from Started to Finished before during interval_open(time 0);
```

`Window` clips assertion and metadata intervals to a nonempty half-open window.
Original untimed nodes remain structure; changed occurrences receive new wrapper
IDs and retain their exact source proofs. Negative assertions remain negative.
An empty intersection emits no occurrence, and declared whole-value restrictions
survive an empty output. After full validation, a window changing no occurrence or
attachment interval preserves record IDs and payloads; repeated identical windows
are idempotent. Its whole-value carrier still protects later generated scalars.
Retained original records follow ordinary Filter/Project semantics, while newly
clipped wrappers carry their restrictions when copied without the envelope.

`Sequence` matches the left target and right source by exact entity/space identity.
Both positive occurrences must intersect the window. It compares their **original**
intervals before clipping: `before` is strict separation, `meets` is equal touching
endpoints, `overlaps` is symmetric nonempty intersection, and `within` is inclusion
including equality. Each match returns two independently clipped occurrences,
never a fact spanning the gap. An explicit prior `Window` changes the input intervals
and therefore can change a later relation test.

Interval values can be function parameters, partial arguments and pure expression
results. Graph operands must be fully bound. Window preserves the complete input
schema; Sequence requires exact descriptor equality. Unknown schemas are checked
by the runtime, not implicitly erased or equated by their labels. Pure functions
and pinned modules retain ordinary qualification and original diagnostic spans.
The operators introduce no implicit store read, source-controlled principal or clock.

Metadata remains directly navigable. Use the current returned node/edge/attachment
IDs when selecting derived hosts. Original local IDs, returned wrapper IDs and
entity/space identity are distinct; proof references never create identity aliases.
Graph-host metadata needs no wrapper-ID discovery. Selection details and occurrence
roles are available in derivation parameters through Explain.

A sequence occurrence, its generated nodes and its metadata retain the pair's
restriction alternatives, including whole-snapshot premises. Private partners do
not become public when an envelope or display readers are removed. Whole-value
carriers preserve selected metadata-path restrictions through empty and scalar
results. Ordinary union conjuncts these carriers; it does not require every proof
alternative. Exhausting the bounded pair/proof/output budget returns a diagnostic
rather than a truncated complete result.

Use exact `use ... revision ...` inputs for replay. A later correction produces a
new source revision; old pinned computation remains distinct and still obeys current
host authorization. A local result is an observation by that runtime, not a universal
recorded-time or cross-replica acceptance claim.

See [the executable example](../examples/temporal.weave) and
[compiler/runtime acceptance](../scripts/check_temporal.py). Source compilation
parity, executed portable-kernel parity, native privacy/persistence and migrations
are separate verification gates. General recorded-time queries, calendar literals,
arbitrary sequence patterns, rolling incremental windows and the rest of L05 remain
open. The scalar Interval type remains separate from graph temporal selection.
