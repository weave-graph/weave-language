# Compiled handler interface for implementation review

Design only, 2026-09-20. This refines [the approved boundary](REACTOR_ARTIFACTS.md) against language `3ea25e3` and current native dispatch after engine `4e93ae0`. Signed export has since reached its native checkpoint. The approved prerequisite is protocol 0.17 exact snapshot/attachment influence; no **handler** protocol number or canonical handler DTO is reserved by this proposal, and no executable reactor is claimed here.

## Proposed canonical artifact

Engine owns a new shared `handler_registration.rs`; the compiler must vendor its reviewed, tested interface before implementation. Types below have strict unknown-field rejection and bounded validation; expression shapes reuse the existing GraphExpression encoding.

```rust
struct CompiledHandlerTemplate {
    format: String, // exactly "weave-handler-registration/1"
    protocol: String, // negotiated supported expression protocol; value fixed at integration
    name: String,
    revision: String,
    input: HandlerInput,
    event_types: Vec<HandlerEventType>,
    recipe: HandlerRecipe,
    output_slot: String,
    source_revisions: Vec<SourceRevision>,
    definition_digest: String,
}
struct HandlerInput {
    graph_id: String,
    branch_id: String,
    metadata_depth: u32, // 0 means no expansion; at most 8 initially
}
enum HandlerEventType {
    GraphCommitted, // serializes "graph.committed"
    GraphAccepted,  // serializes "graph.accepted"
}
struct HandlerRecipe {
    bindings: Vec<HandlerBinding>,
    output: String,
}
struct HandlerBinding { name: String, value: GraphExpression }
```

The runtime injects one immutable binding named `$event`; it is the only initially defined graph value. Authored source identifiers cannot spell that reserved name. Every recipe binding has a unique name, cannot replace `$event`, and may reference only earlier bindings or `$event`. `output` must name an available binding, including `$event` for identity handlers. Output is one graph value, never an arbitrary Program, list of commands or caller-supplied QueryResult.

`event_types` is a canonical ordered list containing exactly both current types. Selective event type matching is deferred because current AdapterManifest only scopes graph+branch and a mismatched leased event needs an explicit durable acknowledgment policy. The paper's MetaGraphRebound alias is rejected as unsupported rather than relabeled.

Shared helper API:

```text
seal_handler_template(unsealed fields) -> CompiledHandlerTemplate
validate_handler_template(&template) -> Result<(), Diagnostic>
handler_definition_digest(&template) -> Result<String, Diagnostic>
validate_handler_recipe(&recipe) -> Result<(), Diagnostic>
```

Validation must be engine-callable and independent of compiler trust. Seal adds the own source identity in a disjoint framed/hash namespace, merges manifests with same-name/revision conflict rejection, sorts canonical manifests and hashes the complete recipe/configuration excluding only its own digest field. Preserve existing source/module/function revisions; no generic JSON key stripping. Alias or entry whitespace changes may normalize, but imported module byte changes always change pins/manifests. Use a new handler-specific hash domain and fixed vectors. No old view/Program fingerprint changes are required.

## Explicit pure-expression allowlist

Allow Reference, Filter, Join, Union, Diff, Project, Support, Context, Metadata, Reason, Explain, Counterparts and Geometry, recursively validating **every** operand (including each geometry operand). These operations may only consume injected/resolved values. Normal context/time/proof/schema restrictions still apply; the artifact does not loosen them. RuleSet and literal scalar parameters are embedded, bounded and source-identified.

Reject Query, AcceptedGraph, CurrentView, ResolveIdentity, Cluster and **TypedContext**. The latter performs a native descriptor lookup, unlike pure Context selection; permitting it would be a hidden graph read despite an exact reference. Supporting it later requires an explicit descriptor input slot or a reviewed preloaded-descriptor path. Reject future unknown operators until individually reviewed. Mutations cannot occur because recipes contain no Command. Metadata extraction operates only over the authorized resolved input closure; it must not fall back to store reads for missing graphs. Any new expression implementation that starts reading graph state must cease qualifying for this profile.

An engine evaluator can reuse its current expression dispatcher with an immutable seeded binding environment, but must call the complete recursive validator first. No general native services, events, leases or clocks are exposed as graph values. Fact times come from literal/specialized parameters; trusted operation time stays private.

## Source syntax and compiler lowering

Use the already proposed syntax, with an explicit graph-returning function reference:

```weave
function Diagnose revision "1" (graph input) {
  metadata Evidence from input on edge "connection" key "evidence";
  reason Warnings from Evidence using QualityRules;
  return Warnings;
}
handler Diagnostic revision "1" using Diagnose {
  input event graph "Installation" branch "main" metadata depth 4;
  on "graph.committed", "graph.accepted";
  output slot "warnings";
  replay pinned;
}
```

The function body uses existing metadata/rule syntax and assumes an earlier pure `QualityRules` declaration. The handler declaration is proposed new syntax, not a claim this complete snippet parses today. The function must have exactly one remaining unbound graph parameter, no schema requirement and a graph result. A partially applied pure function is acceptable if all other scalar/function arguments are already bound and there is no captured graph value. Reject a constrained graph input until an exact execution-time descriptor check is part of the native contract; do not erase that constraint. Existing graph function parameter, schema, scalar and unused-body validation still runs.

Implementation path avoids fake Query source text: extend `functions::expand` with an internal seeded formal graph parameter; lower it to `Reference("$event")` through a separate recipe lowering entry point. Reuse ordinary pure statement lowering and exact scalar specialization; collect only Bind expressions and the resolved function output. Never synthesize a source `use`, persistent graph ID or temporary commit. The normal Program lowering path keeps its existing behavior.

`Statement::HandlerTemplate` records spans for declaration, function reference, event types and host requirements. Module rewriting qualifies the referenced function and handler declaration identity; it never rewrites graph IDs, branch IDs, slot strings or event names. Pure imported modules may define referenced functions/rules; handler declarations themselves remain entry host artifacts and are rejected as imported effects at original spans. Preserve import/application traces through specialization failures.

`CompiledArtifacts` gains `handler_templates`, skipped when empty so old complete output bytes can remain identical. Its aggregate fingerprint keeps v1 for no-handler output and uses a new complete-artifact domain when handlers are present. Reuse conflict-checked module manifests across Program, views and handlers. All legacy execution/output APIs reject unreturned handlers via E_HOST_ARTIFACT_REQUIRED, including compile, specialize, values, plan, fingerprint and execution-oriented describe. Infallible AST/schema inspection stays inspection. `handler-plan --handler NAME` selects one explicitly; `artifacts` returns all values/program/views/handlers; `check` reports the counts and host-registration requirement.

## Trusted native installation

Proposed host-only types are not deserializable authority objects:

```text
HandlerOutputBinding { slot, graph_id, branch_id }
install_compiled_handler(manifest, template, output_binding, authority)
prepare_compiled_handler(adapter_id, event_id, lease) -> PreparedHandlerReceipt
complete_prepared_handler(adapter_id, event_id, lease, preparation_id) -> HandlerReceipt
```

Installation compares manifest.artifact_digest to the sealed template digest, binds exactly the declared graph/branch subscription and one output graph/branch, checks host write permission and installed principal, and rejects external destinations for this profile. The output binding is not inferred from the source slot. Existing manifest resource limits and explicit running/paused/draining/removed lifecycle remain authoritative. Own-output subscriptions reject initially. Cross-adapter cycles are not declared safe; existing queue/attempt limits bound work, and a broader feedback policy remains separate.

Store the immutable template and binding under adapter/config identity. Reinstallation with different bytes/configuration must fail under the existing new-adapter-ID rule. No request installs roots, broadens grants or executes a template merely because it is graph metadata. There is no remote installation endpoint in this slice.

## Preparation and closure: concrete first-profile decision

Preparation runs in an immediate transaction with one trusted clock/read-budget scope. It re-looks-up event source/type/graph/branch and checks the actual current lease through native dispatch authorization; it does not trust a caller envelope. Query that exact event revision without graph filtering and preload its metadata closure to declared depth. Reject unresolved/partial closure and any live metadata handle. No latest-head substitution, implied predecessor or event time field becomes authority.

**Approved prerequisite:** use the reviewed protocol 0.17 exact snapshot carrier after its coherent native/compiler pair freezes. Preparation records every actually consumed immutable event or resolved metadata revision in `GraphInfluence.snapshots`, including genuinely empty and isolated-node snapshots. Existing assertion/node influence and typed-context carriers remain conjunctive restrictions too. These pins originate from the engine's actual authenticated event lookup and authorized reads, never a caller-supplied QueryResult or a descriptive `input_snapshots` array.

Each declared snapshot requires exact revision existence, integrity, current protected-source permission and whole-snapshot authorization through the shared bounded recursion context. A genuinely empty public revision can therefore be a valid input; a missing revision or later unavailable protected scope denies completion and duplicate replay. The gate is not an event identity, proof of truth, latest-head assertion or newly invented ACL. An equal graph or another revision is not a substitute. No fabricated record is necessary and `E_HANDLER_INFLUENCE` for empty-but-authorized revisions is withdrawn.

The final owned output retains the whole-value carrier even when it has no records. Apply global snapshot restrictions to generated nodes, edges, explicit claims and metadata attachments; attachments also retain their explicit assertion/node gates. Preserve original proof alternatives, attribution and exact context, and do not rewrite unrelated OR groups into a flattened proof. The same declared references remain in storage, capsule/admission/export closure and current cached-result checks. Per-operation reference/read/output limits fail explicitly; no truncation or reset of recursion work limits is permitted.

For final serialization, produce an owned derived GraphData snapshot; preserve schemas, contexts, original attribution and alternative groups. Materialization adds the conservative whole-input gate and installed-principal readers to all output records, retaining their original proofs. Do not retain a QueryResult origin envelope that falsely labels altered records as unchanged source payload. Current native source proof traversal must guard both output records and empty output influence after repersistence.

Capture output graph/branch and expected head once. The stored preparation contains template/config digest, event identity, exact full input closure and result provenance, canonical source manifests, captured CAS, and exact single-Commit Program bytes. A preparation digest binds all of them under a separate domain. Store enough bounded immutable content to validate every closure pin against the stored result and template; an unsigned mutable closure vector must not become an authority shortcut. Return a bounded opaque preparation receipt, not unprivileged input inventory.

## Completion, stale work and replay

Completion loads one bounded preparation under the same current transaction/operation clock as validation and writes. Check installation/digest/config/event identity, current source and closure authority, output graph/branch restrictions and lease. The exact stored Program must pass existing output reader checks; no caller-supplied Program is accepted by this bridge. Run the existing handler write/receipt/checkpoint logic internally inside that transaction—do not nest a second BEGIN IMMEDIATE or sample an unrelated time.

A commit-only historical receipt still needs closure revalidation before replay. Existing complete_handler revalidates queried outputs and source event, but that alone cannot validate every dependency of the stored preparation. Add the bridge guard before either fresh execution or duplicate-receipt return. Keep original immutable Program bytes; never run a changed recipe or replace its expected head on retry.

Preparation and completion are separate durable steps. Before/after preparation and before/after completion crashes must recover without ambiguous reinterpretation. A newly leased retry can use the same prepared bytes; an expired lease cannot complete new work. After completion, an idempotent historical receipt may return under current authority without resurrecting a lease. Failure/dead-letter processing remains explicit and cannot silently modify a prepared request.

A pinned event result is historical, not automatically latest. Newer source events do not alter its meaning. If another writer advances the output head, the captured CAS fails with no output/receipt/checkpoint advancement. Rebase or last-writer-wins is not implicit. At-least-once polling does not promise exactly-once external effects. request_effect/begin_effect_dispatch/reconcile_effect remain separate trusted native operations; no completion-plus-intent atomicity is added here.

## Limits, ownership and acceptance

Compiler/shared validation: at most 16 total view+handler artifacts, 1 MiB per artifact/4 MiB combined, 1,000 total expression nodes per recipe, depth32, bounded identifier/source-manifest sizes, and no duplicate/forward bindings. Native: one event/input/recipe/output per prepare, one stored commit per complete, existing cumulative read/materialization limits, at most 999 combined assertion+node+snapshot gates per generated carrier before adding any materialization self-pin (and revalidate attachment origin/reference counts), and a 16 MiB complete stored Program cap. Recheck per-record proof limits after protection. Preparation metadata+stored bytes need explicit per-principal count/byte quotas and bounded pre-decode SQL loads. New storage tables require a reviewed migration/old-binary compatibility decision and unwind/process-death tests.

| Owner | Exact work |
|---|---|
| Engine | canonical handler_registration.rs DTO/helper; native storage, install/prepare/complete, current closure guard, transaction factoring and migration; native security/recovery tests |
| Language | syntax/functions/modules/recipe lowering; complete artifact API/CLI; formatting coverage; modules/identity/diagnostic tests; compiler-to-host source script |
| Single agreed fixture owner | one engine example accepts actual emitted artifacts and drives trusted setup/dispatch/preparation; no duplicate fake DTO fixture |
| Parent | independent adversarial tests, publication pairing and full-scope acceptance accounting |

Must-pass cases: actual source Metadata→Reason warning through engine-injected input; both genuine event types; missing/denied/live closure refusal; typed schema restriction refusal; TypedContext/Query/service sneaking through nested geometry or unused imported body rejection; forged event/lease/recipe/output binding; partial/constant/empty output retaining whole-input gates; genuinely empty immutable input success followed by missing-revision/protected-scope denial; isolated-node input and empty metadata closure pins; stripped-readers repersistence privacy; current revocation between prepare/complete and on duplicate replay; stale output CAS with no checkpoint; changed recipe/body/config rejection; refreshed lease same bytes; crash around both transaction commits; bounded stored load/quota rollback; complete artifact loss guards; module alias identity stability versus changed raw module pins; no source/native external action or silent effect intent.

Compiler preparation seams and executable acceptance ownership are detailed in [REACTOR_LOWERING_PREPARATION.md](REACTOR_LOWERING_PREPARATION.md).
