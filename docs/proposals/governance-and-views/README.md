# Accepted graphs and living views: source boundary proposal

**Design only.** No syntax, DTO, host API or test described here is implemented by this proposal. The language baseline is `8a914c7` (protocol 0.15); the inspected native service baseline is engine `9b7f192`. Engine owns protocol versions, native execution, migrations and authorization. A future protocol version, provisionally 0.16, must be coordinated before implementation. This document does not reserve that version.

Language paper §§4.1–4.2 require pure computation, declared effects and semantic closure; §§9–10 separate governance from authority and require explicit reactive installation. Engine live materialization already exists, but language L15/L17/L24/L26 remain partial or open. The smallest useful next slice is an exact accepted-decision read, a current cached-view read, and a source-generated registration artifact. Policy installation, acceptance and subscriptions stay separate host operations. This does not complete the paper's reactor language.

## Inspected native behavior

`query_accepted_view(AcceptedViewSelection { view_id, decision_id }, host)` returns original source records with a whole-value acceptance influence, including for empty results. An exact decision is checked against its view, local registry, current policy/source authority and trusted operation time. `None` selects the current decision once; the first source profile deliberately requires an exact decision. Guessing the reserved governance graph ID is not an acceptance mechanism. Source query results exclude metadata expansion in this API.

`register_view(ViewDefinition { id, expression, clock }, tick, host)` installs an immutable principal-scoped definition and computes generation one. `read_view(..., RequireCurrent, ...)` rechecks current authority and rejects stale/partial materializations with `E_FRESHNESS`; `AllowStale` returns an explicitly marked native receipt. `refresh_view` is a separate mutation. `view_changes` retains one transition and requires resynchronization for a cursor gap. View ticks are fact-time inputs, separate from the trusted authorization clock.

`enroll_incremental_view(id, host)` opts an existing view into private selection caching. Only an eligible Query/Filter pipeline benefits; other inputs use full recomputation. `refresh_view_with_work` counters are trusted host diagnostics. The proposed scheduler APIs are not part of this source design. Enrollment, scheduling and cache internals grant no authority and must not enter graph values or source selection DTOs.

Native `ViewDefinition` currently has no source manifest. Source-aware registration must retain source revisions and bind them into immutable definition identity; silently discarding that information would leave L26 unresolved. The source cannot solve this by placing arbitrary labels in graph properties.

## Proposed source grammar

These snippets are acceptance fixtures to implement later, not runnable examples today.

```weave
accepted Approved view "fleet-review" decision "decision-17";
lens Connected from Approved { match relation "connected"; at 5; }
apply Evidence from Preserve { graph input Connected; }

view_current Cached view "fleet-active" definition "sha256:canonical-definition"
  tick 5;
apply CachedEvidence from Preserve { graph input Cached; }
```

`accepted` and `view_current` are top-level effectful reads lowered to `Bind`, producing one immutable program-local graph per ordered execution. Functions consume those graphs as arguments; their bodies cannot declare either read. Imported modules remain pure and cannot contain these reads. A repeated independent execution rechecks authority. Neither statement reads the wall clock, installs a policy, accepts a proposal, refreshes a cache or authenticates source labels.

The exact-occurrence choice preserves the native historical semantics: it pins which acceptance happened while current policy/source checks still apply. Following the current accepted head (`decision_id: None` in the native API) remains a separately named future source handle; it must never be inferred from an omitted decision or a special magic string.

The definition hash is mandatory, canonical lowercase SHA-256 with a fixed prefix. Tick policy is explicit: `fixed;` requests a Fixed view with no tick, whereas `tick INTEGER;` requests a Tick view at that signed fact-time instant. Source constants may specialize the instant using the existing Time type. Integer is not implicitly Time. A mismatch against the stored immutable definition fails; the source cannot reinterpret a Fixed view as Tick.

```weave
live_handle FleetHead graph "Fleet" branch "main";
view_template Active revision "1" from FleetHead clock tick {
  match relation "connected";
  at 0;
}
```

`view_template` emits a separate registration artifact, **no Program command**. The first profile permits exactly a Query with the existing lens predicate/time selectors from one explicit live handle, optionally followed by Filter layers if the parser represents them that way. `clock fixed` retains authored valid-time selectors; `clock tick` uses the engine's existing replacement semantics for selectors when the host supplies a tick. It is not an intersection or rolling-window operator. The host chooses the instance ID and supplies any initial tick when registering.

This deliberately excludes arbitrary graphs captured from prior `Bind`s, accepted-head queries, nested view reads, metadata navigation and arbitrary function expansion in a template. Inlining an already evaluated graph reference could accidentally repeat a read or discard its captured snapshot; reject it rather than quietly convert a value into a recipe. A later standalone expression exporter can broaden templates with explicit source/capture semantics. Existing pure graph functions and current source programs remain unchanged.

## Proposed shared read DTOs

Engine owns these additions and their version guards:

```rust
// All structs deny unknown fields; identifiers <= 512 UTF-8 bytes.
struct AcceptedGraphSelection {
    view_id: String,
    decision_id: String, // required exact occurrence, never ambient head
}
struct CurrentViewSelection {
    view_id: String,
    definition_digest: String,
    time: ViewReadTime,
}
enum ViewReadTime { Fixed, Tick { valid_at: i64 } }
// GraphExpression additions, names subject to engine review:
AcceptedGraph { selection: AcceptedGraphSelection },
CurrentView { selection: CurrentViewSelection },
```

`AcceptedGraph` delegates to the native API with `Some(decision_id)`. `CurrentView` delegates to a definition-matching native read and **always RequireCurrent**. It unwraps `ViewSnapshot.result` only after matching the immutable definition and proving `current == true`. Exact source revisions, coverage, schemas, influence carriers and provenance remain intact. Host APIs keep generation/current/tick receipts; graph plans do not imply a durable subscription or expose trusted cache work counters. Stale reads require a future explicitly typed receipt/result design and are outside this first profile.

Definition matching must occur in the same operation snapshot as the view read. A read cannot require a cached view to retain the authority it had at registration. Unknown principal-owned view, wrong definition and inaccessible materialization must not introduce a hidden cross-principal discovery oracle; reuse or tighten existing generic native diagnostics. Reads inside a multi-command Program share its operation scope and rollback boundary. A failure after a prior tentative commit must roll back that commit.

Both variants need recursive pre-write rejection under protocol <=0.15, expression budgets, admission resource traversal and explicit handling in view validation/tick/dependency visitors. In the first profile **raw registration rejects either new variant anywhere in a view expression**, including direct JSON plans. This avoids unsupported dependency tracking: current `collect_heads` tracks graph branches, not acceptance heads or cached-view generations. Standalone source reads followed by pure graph composition are sufficient for this slice.

## Separate host registration artifact/API

Proposed artifact format `weave-view-registration/1`:

```rust
struct CompiledViewTemplate {
    format: String,
    protocol: String,
    name: String,
    revision: String,
    expression: GraphExpression,
    clock: ViewClock,
    source_revisions: Vec<SourceRevision>,
    definition_digest: String,
}
```

Canonical definition identity hashes a domain-separated versioned payload containing protocol, template name/revision, expression, clock and canonical conflict-checked source manifests; it excludes the digest itself and host-selected instance ID. Module aliases, comments and whitespace are excluded by existing source normalization. Do not normalize user literals, command order or temporal operators. Include the template's normalized source revision even if its body contains no function. Source hashes identify code and do not establish author authority.

Retain the linker's complete dependency-module manifests in canonical order and its disjoint framed identity namespaces. Deduplicate diamond imports and reject the same module ID/revision with different content. Imported pure functions used after either read resolve through the existing qualified symbol table and keep their original-file diagnostics. The new template manifest needs its own framed kind tag, so a template cannot collide with a module/function label. Instance IDs, decision IDs, graph/branch IDs and string property values are external literal selectors and must never be namespace-rewritten. The first template profile captures no imported function body; broadening it later must preserve transitive schema and source manifests.

Proposed compiler `compile_artifacts` returns `{ program, view_templates }`; existing `compile`/`plan` must reject templates with `E_HOST_ARTIFACT_REQUIRED` instead of silently discarding them. A new CLI `weave view-plan FILE --template Active` emits one validated template and performs no registration or network operation. `check`/`describe` can report required host effects. Pure imports remain declaration-only; exported/imported view templates are a later profile.

Engine supplies a separate native `register_compiled_view(instance_id, template, tick, host)` or equivalent versioned overload. It validates/recomputes the digest, rejects conflicting manifests, persists the manifest and digest atomically with the definition, and attaches source identities to result identities. Legacy `register_view` remains compatible, but a legacy source-less registration cannot satisfy a source definition digest merely by matching an expression. Re-registering the same instance with a different definition fails. No serialized HostContext, trusted clock, write grant, policy installation, enrollment or scheduler instruction is permitted in this artifact.

Host registration, enrollment, refresh and change polling are individually explicit calls; compilation does not bundle them into an implicit execution transaction. The reference integration host first executes a separately authorized setup/acceptance flow, registers the reviewed template, and later runs read-only compiled programs. The host maps artifact format/version to a supported loader. This loader boundary is not a new remote installation protocol.

## Static checking and limits

Read statements and templates have original spans and module/application traces. Unused function bodies must reject these effects symbolically. Proposed diagnostics: `E_FUNCTION_EFFECT` for a read/template in a pure function; `E_HOST_ARTIFACT_REQUIRED` for ordinary compilation of templates; `E_VIEW_TEMPLATE` for unsupported capture/operator/clock; existing identifier, duplicate, module-effect and budget errors for structural violations. Runtime authorization/freshness failures remain runtime failures, not compile-time certificates.

Accepted/current-view schema is dynamically unknown to the compiler. Unconstrained graph functions and graph operations may consume it. An exact schema-constrained function rejects it with the existing unknown-schema diagnostic; do not invent a static schema from a view name or accept a caller assertion as runtime validation. General checked dynamic schema parameters remain an explicit gap.

Use existing 1 MiB source and bounded AST/expansion rules. Additionally permit at most 16 templates, at most 1 MiB per serialized artifact and 4 MiB aggregate artifacts, with precharge before cloning retained expressions/manifests. Single-selector IDs stay <=512 bytes; template identifiers retain existing language identifier bounds. Existing native expression depth32/node1000 and materialization/read budgets remain authoritative. No request carries an attacker-selected execution quota.

## Executable acceptance plan

Tests below must execute the real compiler and native runtime, not assert hand-authored IR alone. A trusted Rust fixture host owns setup, policy install/acceptance, registration, refresh and revocation. CLI read plans never perform those effects.

| Fixture | Observable acceptance |
|---|---|
| Exact accepted occurrence | Authorized source with a negative assertion and empty-source variant; compiled read equals direct API, acceptance influence survives union/project/explain/save/reopen. |
| Acceptance replacement/revocation | Later accepted occurrence does not retarget an exact prior selection; current policy expiration/revocation still denies old/current/copied reads with no privileged metadata/count leak. Fact-time source literals cannot extend acceptance authority. |
| Wrong view/occurrence | Mismatched occurrence, unavailable occurrence and fabricated reserved graph/caller fields cannot mint acceptance. Version 0.15 plan with a new nested expression rejects before any write. |
| Pure boundary | Unused function containing either source read or a template fails at its original span. Ordinary use of an already read graph in a pure function is equivalent to direct composition. Template-only source emits zero Program commands. |
| Registration identity | `view-plan` output has no host authority; whitespace is stable, predicate/clock/source revision changes alter digest; tampered digest, conflicting manifests and redefinition reject atomically. |
| Current living value | Host registers Fixed and Tick fixtures. Compiled current read equals native RequireCurrent and full query oracle, including exact result pins, schema and provenance. Wrong definition/clock rejects. |
| Staleness and explicit refresh | Source head correction or tick advancement causes `E_FRESHNESS`; a source read does not refresh or advance generation. Host refresh produces the expected insertion/removal/expiry and then source read succeeds. |
| Current permission | Revocation while result is fresh denies the current result and prior transition under current authority. Different principals with the same instance ID cannot read each other's definition or values. |
| Optional enrollment | Host explicitly enrolls the eligible Query/Filter fixture. Output/generations equal unenrolled recomputation after correction, expiry, deletion, restart and fallback; no work counters leak into results. |
| Transition/restart | Host polls one retained transition, restarts, refreshes twice; expired cursor reports resynchronization. Read plans make no exactly-once/push delivery claim. |
| Atomic mixed Program | Tentative authorized commit followed by stale/wrong-definition/unauthorized read fails and marker commit is absent in a new process. |
| Recursive registration rejection | Direct native/raw expression attempts to register a cached-view or accepted-graph expression, even nested in Filter, fail clearly rather than omit dynamic dependencies. |

The integrated paper scenario can then be: compile an evidence lens; host accepts its exact source snapshot; source reads that occurrence and composes an explanation; separately register a simple operational living lens; host refreshes and polls its changes; source consumes the current materialization. Joining the two returned graphs preserves both evidence and acceptance influence. This does not yet implement offline acceptance, reactor syntax, generalized rolling windows, acceptance-head-following views or general incremental evaluation.

## Ownership and sequencing

1. Root/engine review this boundary and reserve a coherent protocol version. No current vendor change.
2. Engine owns shared selectors/variants, recursive guards, native source-aware registration/read identity, clock/dependency validation and tests. Selection cache/scheduler remains separate host work.
3. Language owns grammar, static effects, template artifact generation, original-span diagnostics, examples and actual cross-project fixture runner; vendors only a stable coherent engine freeze.
4. Root independently checks authority/revocation, stale reads, atomic rollback and source identity before publication. L15/L17/L24/L26 status remains partial with the exact limitations above.
