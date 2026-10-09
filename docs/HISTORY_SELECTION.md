# Local recording and accepted-view history

Recording, source revision ancestry, valid fact time and governed acceptance time
are separate axes. The source supplies read criteria; the native host owns the
actual observer and trusted clock.

```weave
accepted_history Past view "team" accepted_at 25 at 7;
accepted_history Exact view "team" decision "decision:actual" observer "actual-local-observer";
lens Reuse from Past {}

recorded_range R graph "Facts" branch "offline" observer "actual-local-observer"
  between 10 and 20 limit 100 at 7;
accepted_range A view "team" observer "actual-local-observer"
  between 10 and 30 limit 100 at 8;
```

Each accepted declaration emits one bound read. Copying/composing that value reuses
its exact source and protected occurrence. `accepted_at` resolves on the actual
local runtime; an explicit decision requires its actual observing identity, as
returned in an authorized runtime observation. Receiving a graph alone cannot
create an accepted occurrence. Independent valid time controls facts, so a result
can be empty while retaining its complete selection witness.

Ranges produce named `history_ranged` response collections, not graph bindings.
Each collection contains an authorized start state and every actual occurrence in
the finite half-open interval in ancestry order, including all equal-time changes.
A transition exactly at start is also in `changes`. Each state carries its own
normal graph result, input pins, observations and source manifest. The optional
`at` filter applies to every state's valid facts. No inaccessible version is
skipped to return a seemingly complete collection. Limits are1–1,000; the runtime
also bounds aggregate serialized work and current authorization.

External graph, branch, observer, view and decision labels are literals and are
not rewritten as module aliases. Reads and ranges are rejected inside pure
functions/handlers and unused imported modules. Collections cannot be passed as
graph arguments. Existing lazy `recorded_handle`, explicit `pin` and sealed
recorded/live view recipes remain supported; accepted values do not expand the
current sealed-view registration profile.

The compiler emits0.21; runtimes reject unsupported new expressions/commands
before preceding writes. The complete0.20 contract/artifact identities remain
valid on the new runtime. Full retention, reactor/source effects, lifecycle,
incremental/transport, portable host and assurance requirements remain open.
