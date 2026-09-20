# Source handler lowering: implementation preparation

Design inspection only, against current source branch after formatter `3ea25e3` and the approved [handler proposal](REACTOR_INTERFACE.md). The compiler still vendors protocol 0.16 while the engine's 0.17 carrier is being reviewed. No new canonical handler DTO, wire version, parser production or executable behavior is introduced here.

## Exact existing seams

| Existing implementation | Bounded change after shared handler interface review |
|---|---|
| `src/syntax.rs`: `Statement::ViewTemplate`, declaration parser and span visitor | Add entry-only handler declaration with spans for function/name/event selectors. Preserve literal graph, branch, slot and event strings; expose no host principal, clock, lease or grant syntax. |
| `src/functions.rs`: private `Definition`, `Closure`, `Value`, `Expander::process`, `expand` | Resolve the handler's function at its declaration point, capture its immutable closure, validate exactly one remaining unconstrained graph argument and graph return, then specialize against an internal graph binding. Do not resolve a later shadowed function. |
| `src/functions.rs`: `Value::Graph` application currently emits a capture Lens | A handler argument must use a seeded internal value rather than a source `use`/Query. An internal Lens alias may lower to `Reference($event)`; it must never become a persisted query or host input. |
| `src/lib.rs`: `compile_artifacts_parsed`, `Lens::expression`, `Lens::emit` | Extract a private lowering context shared by ordinary Program and recipe lowering. Seed the recipe's name map with a Reference input, require every emitted command to be Bind, then return bindings plus resolved output. Validate the complete shared recipe recursively before sealing. Ordinary compilation stays unchanged. |
| `src/lib.rs`: `reject_artifacts`, `CompiledArtifacts::fingerprint` | Include handler templates in complete output; reject them from every output API that cannot return them. Retain the old complete fingerprint domain/serialization when the added handler map is empty. |
| `src/modules.rs`: `kind`, import effect validation, `canonical_sources`, `compile_artifacts`; `src/modules/rewrite.rs` | Qualify function references and declaration identities only. Imported modules may contain pure function/rule/schema dependencies, not entry handler artifacts. Merge and normalize pinned source manifests into every sealed handler, then recompute and validate its digest. |
| `src/main.rs`: artifact command dispatch and `show_artifacts` | `handler-plan` explicitly selects an inert artifact. `artifacts` remains complete; `check` reports handler count and host-registration requirement. No command installs or executes an adapter. |
| `src/format.rs`, formatter examples/tests | Lexer-driven formatting should need no new semantic branch. Add actual handler grammar to syntax/format round trips once it exists; invalid new syntax remains an error until implementation. |

## Specialization invariants

A separate private expansion result should carry ordinary expanded statements/scalar values and pending handler recipes. Avoid running an entire entry Program again for each handler: that would duplicate statements, expansion charges and source effects. At the handler declaration, the existing expander already has prior definitions, captured closures, rules and exact scalar values; create a bounded child recipe expansion from that snapshot, not a second source evaluator.

The child has one reserved graph seed that source identifiers cannot spell. It shares the parent expansion/scalar work budget, uses fresh hygienic names, and retains original definition/application spans. No seeded graph statement is emitted. Its output graph must be a real seeded/derived graph value. A remaining function/scalar, unbound parameter, constrained unknown graph or missing output is a precise error.

Inspect **all** closure bindings recursively, including function-valued captures and definition captures: any pre-bound graph value is prohibited. A callback must not smuggle a graph captured by a partial application. Scalar/function captures are allowed only if their complete specialized recipe remains on the pure allowlist. Captured schemas and exact nominal quantities retain their existing obligations. Definitions still undergo unused-body validation; dead code cannot hide a forbidden host read or future unknown expression.

The current `Lens` type stores a query-shaped filter carrier even when `input` is present. Recipe lowering must always seed `input=Some(Reference)` and never serialize its unused query storage. A small private constructor or separate materialized-value representation can make that invariant explicit; do not introduce a fake persistent graph name or source Query. The shared recursive validator provides a second check independent of this compiler invariant.

Whole-source manifests continue to come from `functions::source_revisions`; linked exact module manifests are added afterward using the existing conflict-checked normalization. New handler identity must include its own sealed recipe and configuration, plus all source manifests. Function aliases and entry whitespace can normalize; changing raw imported module bytes changes pins and aggregate identity.

## Acceptance fixtures before release

1. Actual `Diagnose` source performs Metadata→Reason over the injected event. Compile artifact → trusted fixture install → real commit/accepted event → lease → prepare → complete. The source compiler emits no setup/commit authority on its own. Compare the actual stored graph and prepared bytes, not a handwritten DTO substitute.
2. Compile an identity handler and a recipe returning an empty projection. Feed a genuinely empty event revision and an empty resolved metadata revision. Their exact snapshot restrictions remain in empty output and every generated scalar/attachment; subsequent disappearance or protected-scope denial fails fresh completion and duplicate replay.
3. Exercise partial scalar/function application and a nested callback; reject bound graph captures, schema-constrained event input, scalar/function returns, missing output and free symbols. Preserve original module and application spans.
4. Recursively reject Query, TypedContext, AcceptedGraph, CurrentView, ResolveIdentity and Cluster, including nested geometry operands and unused definitions. Unknown future variants default to rejection. A source handler never gains an implicit store read from pure function syntax.
5. Every legacy execution/output API rejects handler artifacts; complete artifacts return scalar values, Program, views and handlers together. No-handler outputs retain existing serialization and fingerprint behavior. Linked schemas/AST remain documented inspection only.
6. Module alias renaming preserves normalized recipe identity; exact module byte changes require new pins and change aggregate identity. Literal strings resembling `alias::name`, reserved-prefix attacks and invalid event aliases are not rewritten or accepted.
7. Bounded recipe/manifest/artifact counts fail before large clones. Multiple handlers share total expansion/byte budgets; importing a large diamond graph does not duplicate effects or produce undeclared references. Format idempotence and semantic artifact round trips cover the final syntax.
8. The single engine fixture owner drives stale output CAS, changed recipe/configuration, lease renewal, retry after restart, denied metadata pin and current authorization after migration using the actual compiler-emitted artifact. Native atomicity/clock/signature tests remain engine-owned; this fixture proves the cross-project boundary.

## Prerequisites still open

The 0.17 carrier must pass native review and exact source vendoring first. The engine must then publish the canonical bounded handler DTO/sealing/recursive-validation API and the explicit install/prepare/complete/storage semantics. Until that interface is agreed, source grammar and runtime handler version remain unimplemented. This preparation does not claim adapter isolation, automatic effects, release/declassification or full reactive gate completion.
