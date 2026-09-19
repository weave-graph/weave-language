# Live references and explicit snapshot replacement (protocol 0.14)

A live handle selects a branch head when evaluated. A pinned graph value records the exact snapshots used for one evaluation. This source slice uses the existing 0.14 Query, Bind, LiveGraph metadata and compare-and-swap Commit operations; it adds no runtime authority or protocol variant.

```weave
live_handle FleetHead graph "Fleet" branch "main";
pin Snapshot from FleetHead at 5 metadata depth 2;
lens Active from Snapshot { match relation "connected"; at 5; }
```

`live_handle` is a compile-time deferred selector and emits no read. Its graph and branch must be explicit bounded identifiers. It is not an ordinary graph argument: lenses, joins and pure graph functions require a materialized graph value. `pin` executes one query and binds its immutable result for reuse later in the program. Optional `at` selects the root graph's valid time; optional `metadata depth` requests bounded traversal (0–32). Without them, no valid-time selector or metadata expansion is added. Neither wall-clock time nor host authority is read by the compiler.

A pin captures once at its ordered position in each program execution. A commit before a later pin may change the selected head. Re-executing identical source resolves the then-current head again; this construct is not a durable pin across executions. The output manifest contains the exact resolved revisions. Use the existing `use Historical graph "Fleet" revision "exact-revision";` form for a durable explicitly named revision. Legacy `use` without a revision remains a deferred query source for compatibility; it does not promise materialization once.

Pure function bodies reject `live_handle`, `pin`, graph reads and snapshot effects. They can consume an already pinned graph argument. Named bindings are immutable within source; redeclaring a handle is not a rebinding operation.

## Named metadata bindings

```weave
attachment "fixed" on node "asset" key "fixed"
  graph "Evidence" revision "r1" valid 0 until 20;
attachment "following" on node "asset" key "following"
  live graph "Evidence" branch "main" valid 0 until 20;
```

An existing pinned attachment keeps its target revision when the target graph advances. A live attachment resolves to an exact target revision for that query; the original stored attachment stays live. Multiple references to the same graph/branch within one traversal share its resolved pin. Optional metadata traversal records its resolved graph values for later extraction without hidden reads.

Query time filters the root graph and its attachment intervals. Extracted metadata is a graph with its own claim intervals: apply a subsequent `lens ... { at ...; }` to select its claims at a particular instant. Metadata navigation also intersects output evidence with the attachment path's interval and preserves its restrictions. Time-filtered root queries retain edge endpoints, so an isolated node-host attachment needs an unfiltered root query or a relevant surviving relation; this slice does not change existing projection semantics.

A missing top-level head gives generic `E_UNAVAILABLE`. Missing metadata targets or attachments retain the existing partial-result behavior. Hidden records remain absent under current principal authorization; source does not request hidden counts or infer target existence from an empty authorized result. A newly private head does not retroactively rewrite an earlier pinned public revision. Current policy/proof revocation remains enforced by the runtime's existing authorization boundary.

## Replacement is an explicit full snapshot effect

```weave
graph Catalog branch "main" replace revision "expected-old-head" {
  node "asset" entity "asset" space "operations";
  attachment "fixed" on node "asset" key "fixed"
    graph "Evidence" revision "new-exact-revision" valid 0 until 20;
}
```

The order is `graph Name [explicit] [schema Schema] [branch "branch"] [replace revision "expected"] { ... }`. Without `replace`, a declaration requires an absent branch. Replacement requires a nonempty expected revision and never reads the head implicitly to bypass compare-and-swap. Omitting branch selects `main`. The body is the complete next snapshot, not a patch; omitted records are absent from that snapshot. Existing immutability/identity validation still applies. Named local transactions can contain these replacement declarations and are atomic under the existing engine batch protocol.

Changing an attachment's target in a new host snapshot is an explicit metadata rebinding as described by language paper §3.1. It is implemented here by full snapshot replacement, not by a first-class attachment patch command or rebinding an immutable source handle. Old host snapshots retain their earlier bindings. Host write grants are required separately, and a stale expected revision rolls back the entire program, including earlier commits.

## Host live views remain a separate effect

The existing native engine API provides `register_view(ViewDefinition, tick, host)`, `refresh_view`, `read_view` with `RequireCurrent`/`AllowStale`, and durable change reads. A view definition contains a standalone GraphExpression and explicit Fixed/Tick clock policy; registration is principal-scoped and the definition immutable. Hosts must register, supply ticks, refresh, and consume changes explicitly. Refresh uses full recomputation as the correctness oracle and rechecks current authority.

The compiler does not register a view, schedule refresh, subscribe to events or promise push/incremental watch delivery. `pin` is not `watch`. Standalone query expressions can be supplied by a trusted host to the existing view API; a program-local Reference is not independently resolvable as a view definition. Adapter/subscription syntax, checkpoints, effect types and source-level watch registration remain L17/L24 gaps.

The [example](../examples/live_handles.weave), [compiler tests](../tests/conformance.rs) and [actual engine acceptance script](../scripts/check_live_handles.py) cover the source boundary. This implements part of paper §3.1 and engine §5; it does not complete reactive subscriptions or general metadata lifecycle semantics.
