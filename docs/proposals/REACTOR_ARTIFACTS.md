# Source handler artifacts: proposed execution boundary

Design only, inspected 2026-09-20 against engine `6aa4b8d5621177877b9b6e86350d56e6bb55ac58` and language `bb5b142f7ac1f54aa8dcfe7211612cbcf2215410`. No grammar, protocol version, executable handler, migration or acceptance result is introduced by this document. Engine owns canonical DTOs, installation and execution; language owns compilation, diagnostics and complete artifact output.

This follows language paper [§10–12](../source/Weave_Language_White_Paper_v0.1.md), engine paper [§10–12](https://github.com/weave-graph/weave-engine/blob/main/docs/source/Weave_Engine_White_Paper_v0.1.md), and the engine's [three-peer scenario](https://github.com/weave-graph/weave-engine/blob/main/docs/proposals/THREE_PEER_SCENARIO.md). The immediate three-peer experiment uses trusted native adapter code and persisted caller-built completion requests. It does **not** execute the artifact proposed here.

## Existing behavior and the missing bridge

The native implementation is `crates/weave-engine/src/dispatch.rs` at the engine pin above.

| Boundary | Existing behavior | Proposed source boundary |
|---|---|---|
| Installation | `install_adapter` pins `AdapterManifest` identity/configuration, principal, graph/branch subscriptions, output graphs, destinations and budgets. It records an artifact digest but does not load or validate executable code. | Return an inert, sealed compiler artifact. A new explicit trusted installation call validates and stores that artifact and its host configuration. |
| Delivery | `poll_adapter` authorizes the entire source snapshot, returns an exact graph revision and lease, and hides skipped private offsets. Types are `graph.committed` and `graph.accepted`. | The host binds one authorized event graph input. A serialized envelope or caller-provided graph value cannot establish authority. |
| Completion | `complete_handler` accepts a caller-built `Program`; writes, receipt and checkpoint commit atomically. Retry requires the same serialized command hash and current event authority. | A native preparation/completion bridge evaluates the sealed recipe and persists its exact completion request. There is currently no Program command to commit a computed bound result. |
| Egress | Every written record must have readers exactly equal to the installed principal. No release policy is inferred. | Preserve these restrictions, all proofs and whole-value carriers. The source cannot choose a more permissive output audience. |
| External action | `request_effect`, `begin_effect_dispatch` and `reconcile_effect` maintain a separate ledger; unknown is durable before external I/O. | Keep external effects outside the first graph-handler artifact. No claim that effect intent and handler completion are one transaction. |

`graph.accepted` is the current local storage/integration occurrence type, not a substitute for every governed acceptance event. Governance delivery has its own API. The paper's `engine.MetaGraphRebound.v1` / `org.weave.meta.graph-rebound.v1` is not implemented. Comparing authorized pinned metadata graphs can establish a change, but must not relabel a generic commit as a typed rebound event. The current manifest filters graph and branch, not event type; the first profile consumes both real types.

## Smallest useful source profile

Illustrative grammar, pending canonical review:

```weave
function Diagnose revision "1" (graph input) {
  // Ordinary pure graph computation using the input and pinned metadata.
  // No query of an unrelated graph, host service read, or mutation.
  return input;
}

handler DiagnoseInstallation revision "1" using Diagnose {
  input event graph "Installation" branch "main" metadata depth 4;
  on "graph.committed", "graph.accepted";
  output slot "warnings";
  replay pinned;
}
```

The handler's function has one graph parameter and one graph result. Initial support is an unconstrained graph parameter; a later constrained parameter must compare a complete schema descriptor at execution, not trust a schema label. Ordinary scalar captures are specialized exactly at compilation. Imported functions and rules remain content-pinned pure units. A declaration never installs, starts, subscribes or executes anything.

The compiled recipe is an acyclic list of immutable graph bindings and one result expression. It has a distinguished event input slot, separate from user variable names. Existing expression encodings may be reused inside this new artifact, but engine validation must reject Query, AcceptedGraph, CurrentView, ResolveIdentity, Cluster, mutations and every other host read. Metadata is permitted only over the already resolved, authorized input closure. Pure algebra, explicit context selection, rules, geometry and explanation remain subject to their existing budgets and semantics. No fake graph ID or textual substitution stands in for the event.

The first recipe cannot silently consume wall-clock time, invoke a model, infer absence from missing metadata or perform external I/O. Valid-time literals and captures are ordinary data; lease and capability expiry use the trusted operation clock. Approximate or external/model-backed handlers require separate recorded-output and reproducibility contracts.

Proposed strict DTO outline, not canonical types:

```text
CompiledHandlerTemplate {
  format: "weave-handler-registration/1", protocol,
  name, revision,
  input: { graph_id, branch_id, metadata_depth },
  event_types: ["graph.committed", "graph.accepted"],
  recipe: { bindings: [{ name, expression }], result },
  output_slot, source_revisions, definition_digest
}
```

Event types are a canonical set containing exactly the two currently supported types. Input graph/branch and output slot are literal resource requirements, never grants. The engine must reserve the protocol/profile after reviewing the native bridge; this proposal does not claim that protocol 0.16 can execute these artifacts.

## Installation, preparation and immutable replay

Proposed native operations are explicitly trusted host APIs, not new Program effects:

1. `install_compiled_handler(manifest, template, output_binding, authority)` validates the sealed recipe and source manifests. It checks the manifest artifact digest, exact single subscription, one output slot mapped to one authorized local graph **and branch**, and engine-supported limits. The installed principal, write/egress grants, output binding and limits come from the host. Current manifest output grants are graph-scoped; the new binding must additionally constrain the branch. The first profile disallows subscribing to its own output graph; broader feedback/cycle scheduling needs a separate policy. Manifest lifecycle remains explicit. A different artifact/configuration requires a new adapter identity under current native rules.
2. `prepare_compiled_handler(adapter, event, lease)` looks up the actual installed manifest and event, checks the current lease and whole-source authorization, and loads the exact event snapshot under a stable operation scope. It evaluates the recipe, captures output CAS and every source dependency, then durably records an immutable prepared request before returning a bounded receipt/reference. It never trusts caller graph values, supplied event type/source or a supplied clock.
3. `complete_prepared_handler(adapter, event, lease, preparation_id)` rechecks current authority, lease and output scope, and submits exactly the stored Program through the existing atomic handler completion boundary. It does not regenerate the Program on retry. After committed completion, duplicate replay follows current receipt rules and cannot create another event.

The preparation identity binds adapter/config/artifact, actual event source and ID, exact dependency pins, source manifests, complete output bytes, output target/branch and captured expected head. The stored request includes all bytes needed for replay; the current handler receipt alone stores a hash/results, not the original caller-built request. A head conflict is an explicit failed preparation completion, not permission to update the expected head and reinterpret the same occurrence. Rebase or alternate output requires a separately reviewed explicit policy.

The initial policy is historical event processing, not a claim that output reflects the latest input head. A newer input event can arrive before completion without invalidating the old event's pinned meaning. The output carries its source watermark/pins; a consumer requiring current state must compare freshness through an explicit host policy. A delayed completion whose output CAS has changed fails instead of overwriting newer work. If a lease expires, a newly authorized lease may resume the same prepared request; it cannot change recipe, inputs, captured CAS or output bytes. A changed installation/configuration never reinterprets an old preparation.

Live metadata is a replay hazard even when the outer event graph is pinned. The first profile rejects live attachments in the traversed closure, or must pin and persist their resolved closure before evaluation; it must not resolve them again on retry. The recommended first implementation is rejection. Missing/partial metadata prevents preparation by default and exposes a typed incomplete diagnostic, rather than silently producing a complete warning. Broader explicitly partial handlers remain a later profile.

Output is a derived owned snapshot, with schema, context, assertion/node premises, alternative proofs and whole-value influence retained. Restricting readers must not pretend a changed result is the unchanged object under its original origin envelope. Generated/wrapped records need the same reviewed identity/proof rules as other runtime materialization. The prepared result must be current-authorized again even when the replay Program returns only a commit receipt, because `complete_handler` only has general queried-result revalidation plus event checks today.

An important open bridge case is a wholly empty input with no assertion/node witness. Empty input influence must be retained even when the output is a constant scalar or another empty graph. `GraphInfluence` cannot represent an arbitrary event or graph revision as a fabricated AssertionRef. The implementation must introduce a reviewed exact snapshot/event influence representation; until one is available, it must explicitly reject preparations whose influence cannot be represented. The initial evidence-bearing warning fixture does not justify claiming all empty/event-derived results are safe. This is a required acceptance decision before native implementation, not a reason to erase influence.

## Capability and external-effect ownership

Source effect inspection reports requirements such as consuming the declared event scope and producing one local graph. It does not mint a capability named by a string. The host owns installation, authenticated principal, write scope, lifecycle, lease/retry settings and any remote admission. Receipt/diagnostic access inherits the same private event and evidence boundaries; no global offsets or hidden event counts become source values.

The first artifact has no external destinations or arbitrary effect payload. A later effect artifact must declare destination slots and bounded payload schemas, bind them to host allowlists and release rules, and preserve exact recorded requests. The present `request_effect` requires a current lease; `complete_handler` clears that lease. Requesting an effect before completion therefore has a crash window in which an intent exists without completion. An atomic completion-plus-intents API would be new native work, not an existing guarantee.

The broker marks an intent unknown before I/O. A lost response requires independent reconciliation evidence; handler replay must not resend it. Hashes, signatures, governance approval and model summaries do not declassify private source data. A same-owner three-peer maintenance example is useful evidence; independent-recipient delivery remains denied until an explicit release policy exists.

## Complete compiler artifacts and identity

Extend complete compiler output to retain Program, typed values, view templates **and handler templates**. Legacy `compile`, `specialize`, `values`, `plan`, output fingerprint and execution-oriented describe APIs must reject unreturned handler artifacts with a precise diagnostic. `ast` and schema inspection can remain explicitly non-executing. An explicit selected-handler output command may emit one artifact; complete output must never drop siblings silently.

Use a separately versioned complete-artifact fingerprint when handlers are present, preserving existing no-handler identities. The handler definition binds the canonical compiled recipe, exact parameters, name/revision, declared resource requirements, operator/profile version and conflict-checked source manifests. Its own source label uses a disjoint framed namespace. Dependency module manifests retain exact content pins: changing a module comment still changes its required bytes/pin. Entry formatting and alias renaming may normalize; semantic literal keys named `span`, `value` or `param` must survive.

Event/dependency pins belong to prepared-run identity, not the static handler definition. Hash equality establishes identity under a declared encoding, not authority or algebraic equivalence.

Initial compiler limits should share the existing artifact envelope: at most 16 total host artifacts, at most 1 MiB each and 4 MiB combined, charged before retaining clones. Recipes additionally have at most 1,000 expression nodes and depth 32. Each prepare call processes one event, one recipe and one output, with cumulative read/materialization budgets and a stored completion Program no larger than the existing 16 MiB handler limit. Each complete call loads one bounded preparation and executes one captured commit; it does not re-evaluate a recipe. Runtime graph/read/derivation limits remain stricter where applicable; host settings cannot be enlarged by source. Native preparation storage also needs per-adapter and total quotas plus bounded retention/recovery rules. Transaction errors and unwinds must release operation-clock scopes and roll back preparation/completion state consistently.

Diagnostics need original module/file spans for wrong function kind/signature, forbidden native read/effect, unsupported event type, schema uncertainty, live dependency policy, duplicate artifact/source identity, excessive expansion and legacy artifact loss. Runtime unavailable/missing/private input remains generic where detail would expose denied content. Describe/check should expose required host operations and unsupported replay conditions without claiming installation or approval.

## Proposed executable acceptance

The language owner supplies actual source, complete artifact output and a cross-project script. Engine owns the trusted native fixture and bridge implementation; agree one fixture-file owner before writing. Reuse the existing three-peer data and caches, without broadening the native trace during its current work.

| Case | Required observation |
|---|---|
| Compile/install boundary | Compilation emits no writes or subscriptions. Old output APIs reject artifacts. Explicit installation validates exact bytes and independently supplied host scope. |
| Real event → warning | A committed installation event binds its exact graph; Metadata→Reason derives a principal-scoped warning carrying attachment, evidence, rule and source manifests. A genuine accepted occurrence is a separate input, not duplicate source-event identity. |
| Prepared replay | Kill after durable preparation, before completion commit, and after completion before acknowledgment. Restart reuses byte-identical Program and returns one logical output/receipt. Advance source/metadata heads between attempts; pins do not move. |
| Revalidation/CAS | Expire/revoke source authority or change output head after preparation. Completion fails without partial writes/checkpoint. A current lease and authorized scope are checked; source-provided clock/envelope cannot revive them. |
| Closure/privacy | Private metadata, node-only/scalar/empty outputs and mixed alternative proofs retain restrictions. Readers stripped on a later saved derivative do not expose it. An unrepresentable empty-event influence fails explicitly. |
| Static purity/identity | Unused forbidden bodies and imported effects reject at original spans. Alias renaming preserves identity; changed pinned module bytes change it. Different exact scalar parameters change identity; property keys remain unchanged. |
| Resource/recovery | Oversized recipes, dependency closures and prepared receipts reject before unbounded retained allocation; preparation quotas, expired leases and dead-letter recovery stay bounded and durable. |
| External ledger, separate stage | A fake destination observes one action, loses its response and leaves durable unknown. Replay cannot dispatch twice; reconciliation requires explicit broker evidence. No atomic handler-plus-intent claim. |

These checks would advance L4 and tooling/replay portions of L7. General event schemas, model/external handlers, sandboxed modules, governed-output completion, release authority, arbitrary feedback scheduling and complete reactive language conformance remain required or explicitly researched gaps.
