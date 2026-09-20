# Compiler and acceptance work map

Prepared against language `ed31d93`; no implementation or shared DTO changes yet. Canonical 0.16 interfaces remain engine-owned and must freeze before vendoring.

## Compiler insertion points

| Area | Required change | Compatibility check |
|---|---|---|
| `syntax.rs` | Dedicated accepted/current read and view-template AST variants; bounded selectors, explicit Fixed/Tick and template revision; retain original name/source/time spans. | Reject these effects when parsing unused function bodies and imported module units; external selectors remain literal strings. |
| `functions.rs`, `graph_types.rs` | Carry top-level read results as dynamic graph types; preserve template declarations without emitting graph commands; specialize any allowed Time scalar leaf. | Exact schema constraints reject unknown schemas; scalar-returning functions cannot acquire reads/captures. Keep existing normalized function identities unchanged. |
| `lib.rs` | A complete artifact compilation path returning Program, typed scalar values and templates; native reads lower to one Bind; templates emit no Program commands. | Existing compile/specialize/fingerprint/describe paths reject unreturned artifacts. The complete artifact fingerprint uses a distinct versioned domain. |
| `modules.rs`, `modules/rewrite.rs` | Apply canonical identity remapping and conflict-checked dependency manifests to both Program and every template; preserve error/application locations. | Alias rename and diamond import are stable. Module/function/template namespaces cannot collide. Imported effectful declarations remain forbidden. |
| `main.rs` | Explicit `view-plan FILE --template NAME [--modules MAP]`; serialize only the selected reviewed artifact. | Existing plan/values/fingerprint reject templates. CLI describe validates before its schema projection. The infallible `LinkedProgram::schemas()` remains an explicitly documented AST inspection accessor, like `ast()`, not an executable compiler output. |
| Tests/docs | New focused source/artifact tests, CLI runner and syntax/API boundaries. | No claim of policy installation, implicit refresh, automatic subscription or full incremental evaluation. |

All template construction must precharge retained artifact bytes. Legacy APIs should find and reject a template at its source span before returning a partial Program, typed values or fingerprint. Complete artifact compilation must still check ordinary source semantics, source identity conflicts and every unused function; it is not a second weaker compiler.

## Reference host fixture contract

Ask the engine owner to reserve a single new example helper, tentatively `source_view_fixture`, without duplicating runtime implementation. The helper is explicitly trusted setup/test code. Its local test clock and signing keys are fixture construction; neither is loaded from source plans or installed by a compiled template.

Suggested host modes (exact CLI is fixture-owned, not a public runtime protocol):

1. `seed DB`: create the operational graph, an empty source variant, scoped governance policy and accepted occurrences through existing genuine native APIs; emit JSON containing the exact view/decision/source pins. No fabricated protected decision graph.
2. `register DB ARTIFACT INSTANCE FIXED_OR_TICK`: deserialize/validate the canonical template via the new native overload; emit the actual immutable definition digest and native snapshot receipt.
3. `change DB`: replace a source head with a deterministic correction/reader/time change under explicit expected head; emit the new pin.
4. `refresh DB INSTANCE FIXED_OR_TICK`: invoke native refresh explicitly and emit its snapshot. Enrollment and scheduling, if exercised, are separate host flags/calls and never fields in the template.
5. `read DB INSTANCE DIGEST FIXED_OR_TICK`: native definition-matched RequireCurrent oracle for comparison with source plans.
6. `expire` or a fixed test-clock host invocation: test current governance expiry while source/decision pins and cache generation remain unchanged; source literal fact time never sets authorization time.

The compiler runner creates source strings from the helper's emitted exact pins, compiles them with the actual binary, executes the generated Program through the current engine, and compares complete results with the host oracle. It uses a fresh DB per scenario and reopens in independent processes for replay/rollback checks. It must accept existing compiler/helper/runtime paths and never start an uncoordinated Cargo build itself.

## Focused fixture groups

1. **Source read lowering:** exact occurrence and definition/time selectors, empty accepted source, dynamic-schema diagnostics and old-wire rejection. Compare Bind payloads, then real native result envelopes.
2. **Artifact boundary:** template-only source has zero commands; mixing a template with scalar values cannot make legacy values/fingerprint silently succeed; complete artifact output retains both. Duplicate template/name, bad digest, unsupported capture/operator, wrong clock and missing selector diagnostics retain exact spans.
3. **Source identity:** dependency modules used by subsequent graph functions retain canonical names in Program and template manifests; alias changes/diamond dedup preserve artifact identity; literal strings named `span`, `value` and `param` remain data. Changing template predicate/revision, clock, or a linked source digest changes the artifact identity. A conflicting manifest fails before registration writes.
4. **Current lifecycle:** register → compiled current read → source correction → stale source read fails without refresh → native refresh → compiled read matches oracle. Repeat for explicit tick expiry and restart. Registration does not enroll or schedule implicitly.
5. **Proof/authority closure:** accepted empty/nonempty results keep decision/source influences through a pure function, union/project/explain and persisted copies. Expiry/revocation denies through the same current native checks; selectors cannot identify a different principal's same-named instance.
6. **Atomic failure:** a tentative graph commit followed by stale/wrong-definition/unavailable read fails; fresh-process inspection finds no marker commit. Successful read-only plan adds no source events, view generation, enrollment or scheduling records.
7. **Host artifact identity:** source-aware registration retains and conflict-checks manifests through initial, full fallback and incremental recomputation; same instance/different artifact rejects. Legacy source-less registration cannot satisfy a source-aware digest.

The initial integration requires engine-provided native selector/artifact validation and source-aware registration. It must not synthesize those behaviors in the fixture or accept a caller-created result envelope as an oracle.
