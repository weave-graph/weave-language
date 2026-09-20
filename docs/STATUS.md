# Verified status

As of 2026-09-20, this is an experimental compiler foundation, not full Weave implementation and not full conformance to the recovered architecture white papers.

## Accepted graphs and compiled view artifacts (protocol 0.16)

[Source service reads and explicit host artifacts](VIEW_ARTIFACTS.md) now cover
exact accepted occurrences, definition-matched RequireCurrent reads, and compiled
Query/Filter view templates. Complete compiler output retains Program, typed values
and templates; legacy execution outputs reject unreturned artifacts. Source-aware
host registration and every recomputation preserve exact module/template manifests.
Host acceptance, registration, enrollment, refresh and scheduling remain explicit.
L15/L17/L24/L26 remain partial; this is not a general reactor or view language.

Local validation: **134 language tests passed**, including nine artifact cases and
two independent module-effect/literal-selector cases. Strict all-target Clippy,
formatting, library WASM compilation, exact 26-file contract verification and the
three original source artifacts passed. The vendored contract is pinned to coherent
engine `8d359df` (protocol 0.16), including its final native registration documentation.

Actual compiler-to-native acceptance passed through the trusted
`source_view_fixture` helper: genuine accepted/empty proof carriers, exact module
artifact identity and alias stability, separate registration, current/stale reads,
explicit refresh and tick expiry, retained source manifests after incremental
refresh, historical acceptance after replacement, persisted proof denial after
policy expiry, and fresh-process atomic rollback of a commit before a stale read.
The helper's fixed clock/keys belong to test host setup, never source authority.

Executed native/Node WASM compiler parity also passed: **6,091 identical JSON bytes**,
four scalar cases, three scalar errors, one complete view artifact and its legacy
rejection diagnostic, with zero WASM host imports. This fixed portable compiler
fixture does not claim browser storage or runtime host installation. Cargo used two
low-priority jobs and existing caches; final source-archive evidence is recorded in
the orchestrator handoff. Public publication remains the orchestrator's responsibility.

Independent root verification passed both source boundary tests, all seven native compiled-view/binding tests, actual compiler-to-host acceptance, schema13-to14 process-death migration and old-binary refusal, and executed 6,091-byte compiler WASM parity. All 26 vendored files match canonical engine8d359df exactly (license from repository root). All 21 historical top-level examples retain identical plans/source identities apart from the declared protocol version. The exact25b1da8 source archive built locked/offline using cached dependencies. Paired hosted CI is tracked in the public project evidence; full gates remain open.

## Ordinary scalar checkpoint (source-only)

[Scalar functions](SCALAR_FUNCTIONS.md) now support exact typed scalar parameters,
results, partial application and explicit unary callbacks. Typed scalar expressions
lower to existing protocol 0.15 properties and literal attachments. Scalar-returning
functions cannot capture/read graphs or emit graph operations; broader mixed-result
functions remain open. L01–L03 remain partial, not complete.

Local evidence: all 122 tests passed before the final scalar graph-input guard;
the changed scalar/root suites then passed all 21 cases, covering 123 total test
cases. Strict all-target Clippy and formatting passed. All 21 previously tracked
top-level examples produced the same complete JSON plans/source manifests as public
compiler `5c3e909`. Exact vendor verification passed for 24 protocol files; all three
original paper artifacts verified. Cargo used two low-priority jobs and one cache.

Executed compiler parity passed: **5,095 identical JSON bytes**, four typed value/plan
cases and three diagnostic cases, native versus Node WebAssembly, with zero WASM
host imports. This is stronger than a WASM compilation check and remains a bounded
fixed compiler profile, not browser persistence. Actual scalar CLI acceptance
passed against the protocol-0.15 schema-12 runtime baseline: canonical Decimal and
Quantity values, unchanged typed graph schema, fresh-process pinned replay and
failed specialization emitting no runnable plan. Source-archive verification is
recorded in the final orchestrator handoff; publication remains orchestrator-owned.

| Gate | Status | Evidence and remaining work |
|---|---|---|
| L0 Source and contract | in_progress | Original papers recovered and reconciled; exact language source and hashes are included. Protocol v0.16 accepted/view artifacts plus schema/metadata/algebra/assertion/rule/context/geometry and graph-influence implementation addresses documented model gaps. Full semantic conformance remains open. |
| L1 Front end | in_progress, provisional | Lexer/parser, versioned scalar schemas, typed endpoint/space validation, JSON diagnostics and check/ast/plan/describe CLI implemented; ordinary scalar specialization and 123 test cases now covered. Rich graph/vector/quantity types, effects and formatter remain. |
| L2 Deterministic semantic kernel | in_progress, provisional | Typed relation/time parameters, partial application, temporal filtering and reusable cross-graph path-join values lower to engine IR. Named intermediate results feed later parameterized lenses and joins without commits. Union, diff, projection and temporal four-valued support now execute. Typed total contextual assignments now preserve exact descriptor witnesses; general higher-order lenses/joins and explicit context compatibility remain. |
| L3 Full knowledge semantics | in_progress, provisional | Named attributable graph attachments, native metadata-value extraction and real cyclic local snapshots now execute through a logical manifest; typed schema meanings are retained through query/join results. Finite signed-evidence rule closure now executes. Declared counterpart bridges and source-level explanation execute; accepted identity resolution now executes with explicit policy/mapping pins; general alignment, stratified absence and richer scenarios remain. |
| L4 Reactive secure integration | proposed | Public bootstrap declarations only; engine must enforce host authorization. Reactive syntax and differential live views remain. |
| L5 Spatial and multiscale language | in_progress, provisional | Assertion-backed distance, transforms and display projections now lower to engine services. Pinned bounded cluster navigation now executes; general vector types, learned mappings and overlapping clustering remain. |
| L6 Distributed offline governance | proposed | Portable source structure only; no mobile, networking or governance conformance claim. |
| L7 Tooling and release | in_progress, provisional | MIT license, independent crate, README and three-platform/WASM CI definition exist. The orchestrator verified public v0.4 CI on Ubuntu/macOS/Windows and the WASM target; later milestone publication remains tracked in PUBLICATION.md. LSP is optional proposed editor tooling, not a user-scope completion gate. |

## Earlier local foundation verification (historical)

- `cargo test --locked`: 102 behavior/conformance tests passed, including half-open interval laws, negative scope/type cases, revision pins, data/code isolation and 5,000 deterministic malformed input cases.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- `cargo run --quiet -- check examples/fleet.weave`: valid, three commands.
- `cargo run --quiet -- plan examples/fleet.weave`: emitted version 0.15.0 protocol plan.

The checks above were run locally on macOS with Rust 1.94.0. A CI definition is not evidence of remote CI success. `cargo check --locked --lib --target wasm32-unknown-unknown` passed; browser-host execution and mobile integration remain unverified. Exact cross-project integration is recorded separately by the orchestrator.

## Publication boundary

The private, partial chat export is excluded by `.gitignore`; it is not an original white paper and must not enter public history. Public docs summarize requirements without publishing unrelated private conversation material. The original language paper is preserved under docs/source with exact provenance; source reconciliation is complete. Full implementation remains incomplete. The public MIT repositories are https://github.com/weave-graph/weave-language and https://github.com/weave-graph/weave-engine. Publication and CI verification are orchestrator-owned; a local commit is not necessarily the current remote head.

## Independent orchestrator review

The orchestrator verified a clean independent clone of foundation commit `573befd` against engine `0cee0e1`: locked tests and actual CLI integration passed for metadata partial coverage, pinned replay after restart, half-open time boundaries, host authorization and optimistic concurrency. The orchestrator also verified the v0.2 join slice: time intersection, both premise revisions, principal-scoped output, disjoint/private/negative/identity-mismatched exclusion and collision-safe local IDs.

## Composed-value integration

The v0.3 example was executed against the native engine CLI in a fresh temporary database. A first join fed a parameterized lens and a second join, followed by a final time filter. Ten commands produced three explicit source commits and one final `needs_fix` edge valid over `[170, 200)`, with three input snapshots and exactly the three leaf premises `uses-model`, `affected-model`, and `repair`. No intermediate derived value was committed. The source's negative repair claim did not generate a positive conclusion.

Reproduce with `python3 scripts/check_composed.py --engine /path/to/weave-engine`. This is an actual cross-project local execution result; separate independent orchestrator review is still required for the new stage.

## Source distribution audit

The independent source archive at `ea484f7f1c24932b177e4a87daf7bb875cecb671` passed contract verification, all 22 tests with locked cached dependencies, CLI installation, and the composed-example compile check without a sibling engine checkout. The archive contained the complete vendored contract and excluded the private source export. [Release readiness](RELEASE_READINESS.md) records exact evidence and publication limits; [acceptance gaps](ACCEPTANCE_GAPS.md) maps all 30 remaining full-design requirement areas.

## Paper-directed v0.4 verification

32 frontend tests cover schema field/type/space errors, discovery, named metadata, local transaction boundaries and graph-value composition. The shared portable validator has four additional engine-contract tests. Strict Clippy and the WASM library target check pass.

Actual CLI runs of `schema.weave` retain the graph schema. `metadata_cycle.weave` commits the two mutually referring snapshots atomically, traverses the real cycle with complete coverage, selects an edge's evidence graph and joins it to an independent catalog. Its final provenance contains the evidence edge, the named host attachment and the catalog review edge. The orchestrator independently ran both examples through the compiler/runtime.

Full schema types/migrations, live-handle source syntax, arbitrary attachment policies, source-level rebinding and the remaining full-design gates are still incomplete. Current docs and source artifacts do not imply those capabilities.

The typed cross-schema join example deliberately reuses type names with different definitions. Its actual runtime result retains two distinct resolvable node types, a declared derived edge type and valid interval `[150, 200)`. The compiler rejects statically known mixed typed/untyped joins; dynamic inputs remain subject to the same runtime check.

## Graph algebra v0.5 verification

35 frontend tests, strict Clippy and the WASM library target check pass. The vendored portable contract has 13 tests (nine algebra and four schema). Actual compiler/runtime execution of `examples/algebra.weave` passes `scripts/check_algebra.py`: endpoint-preserving projection, membership removal with unchanged positive polarity, repeated union deduplication, supported/conflicted/refuted/unknown at distinct instants, joint conflict provenance and principal-scoped output. Typed union retains nominal types across nested composition. Derivation alternatives preserve parent operator parameters and group-specific premise snapshots.

Diff currently annotates node/edge membership, not attachment-only changes or inferred mutation correspondence. Four-valued support does not provide closed-world certificates or confidence arithmetic. General rules, higher-order graph functions and explicit explanation/view identity APIs remain open.

## Pure graph functions verification

40 frontend tests pass. `scripts/check_functions.py` runs the compiled higher-order example against the engine and verifies exact equality of graph data, schemas, snapshots, coverage and provenance against directly specialized and literal lenses. Exactly one source commit is emitted. Graph captures are immutable; function bodies cannot hide reads or writes, and invalid parameter kinds, graph/function return mismatches, recursion and bounded expansion are tested. The existing 0.5 protocol is unchanged.

Schema-constrained signatures, general value types, modules, source-aware runtime identity and explanation APIs remain separate acceptance gaps. Protocol0.6 now carries explicit source function revisions and normalized digests into runtime fingerprint dependencies.

## Explicit assertions and identity v0.6 verification

45 frontend tests and 19 portable contract tests cover explicit structure/claim separation, claimed source/context/property preservation, structural-only unknown evidence, assertion-host metadata, profile rejection, source manifests and deterministic identities. Actual CLI acceptance passes `scripts/check_assertions.py` and `scripts/check_functions.py`, including fail-closed contextual support and exact source manifest propagation. Strict Clippy, formatting, locked dependencies, WASM library and vendor/source verifiers pass.

The portable explanation helper is graph-valued, current-principal-scoped and composable with union. Construction budgets are charged before cloning derivation groups; origin-map changes affect full-result identity. Source normalization removes only known AST spans. Context applicability remains required future work, as do source-level explain, complete policy-aware view registration, schema-constrained function arguments and terminating general rules.

## Finite rules v0.7 verification

48 frontend tests and 27 portable contract tests cover source rule modules, exact unsafe-variable diagnostics, immutable function use, cyclic fixed points, explicit negative evidence, time intersection, typed composition, independent support alternatives and explicit budget exhaustion. Prior derivations cannot introduce empty or foreign leaf support through a supplied trace. Module labels with conflicting digests fail explicitly. Actual `scripts/check_rules.py` execution verifies repeated closure and function equivalence, graph-valued filtering, exact premise lineage and no hidden commit.

The rule evaluator accepts only host-authorized materialized values. Trace labels are descriptive execution/provenance records, not authenticated remote attestations. Current context-free evaluation is the default context, never universal applicability. Context selection, typed contextual axes, stratified absence over certified complete scopes, aggregations and richer rule types remain mandatory gaps. Full implementation gates remain in progress.

## Exact context v0.8 verification

50 frontend tests and 33 portable contract tests cover exact selected scopes, default isolation, bounded pins, contextual attachment syntax, function use, scoped support and rules, mixed union/diff behavior, and generic derived-node qualifiers. Support schema revision2 adds readable context fields. Unknown status and explanation nodes retain scope after mixed union, and cannot reappear as default-world results through selection. Schema validation checks claim endpoints against their value scopes. Strict Clippy, formatting and portable WASM checks pass.

Actual `scripts/check_contexts.py` executes a transaction containing world, evidence and source-claim graphs. Default support is refuted, the selected world is supported, and an absent world is unknown with its pin retained. Contextual metadata feeds a join carrying the source claim, attachment and evidence claim as exact premises. A graph function and direct context selection agree; a rule conclusion retains the world pin. The rule fixture also still passes against the updated runtime.

No context compatibility is inferred from names, missing fields or coordinates. Typed axes, explicit broadcast/conversion policies and governed world crossing remain open. Root-owned runtime admission, capsules and remote CI evidence are tracked separately.

## Geometry and source explanation v0.9 verification

54 frontend tests cover finite float schemas, bounded structured literals, duplicate JSON keys, strict integer time, graph argument binding, projection axes and graph-function specialization. The portable schema validator accepts finite binary64 fields without weakening integer fields. Native strict Clippy, formatting and WASM library checks pass.

Actual `scripts/check_geometry.py` execution verifies a 5-metre physical distance; an explicit transform whose reusable coordinate result yields 500 centimetres to another point; exact coordinate/calibration/target premise lineage; cosine distance 1 for orthogonal embeddings; projection revision and approximate status; and rejection of a display projection as a distance input. Source-level explanation produces another graph that composes with union, and direct versus function-wrapped distance produces identical graph data. The source performs one explicit bootstrap commit and no hidden intermediate writes.

Geometry payloads are selected from positive, authorized assertion properties. Their descriptor, role, endpoint-space anchors, context, time and permissions are runtime validation obligations. Compile-time validation covers literal structure/finiteness and operator syntax, not full static geometry typing. General vector/quantity schemas, spatial source declarations, learned bridges, complete identity/counterpart semantics, clustering syntax and platform execution conformance remain open.

The 0.9 node-proof closure adds conservative recorded dependencies to geometry, support and explanation nodes. Portable tests verify unknown status dependencies, per-group explanation dependencies and rejection of altered proof records under the same node origin. This prevents treating an isolated scalar node as provenance-free; runtime visibility and capsule enforcement are engine-owned. Conservative conjunction may produce false denial when alternatives are independently visible.

## Declared counterpart v0.10 verification

56 frontend tests include source syntax, distinct bounded space selectors, strict integer time, undefined graph diagnostics, protocol round trips and function composition. `scripts/check_counterparts.py` executes the typed explicit-assertion example: source assertion and structural identity remain intact, manifestation state stays distinct, a normal join retains both leaf claims and their time intersection, and reverse/out-of-window selection is empty. The portable selector preserves partial coverage, exact selected contexts and endpoint proof dependencies.

This profile selects positive directed bridge claims whose endpoints already declare the same entity identifier in different spaces. It does not establish mapping acceptance, uniqueness or contradiction resolution. Governed mapping snapshots, independently assigned identifiers, supersession/splits and privacy-preserving pairwise identity remain open requirements.

## Isolated exact decimal library verification

The public `decimal` module implements a bounded exact decimal profile without shared-protocol changes. Five tests pass, including a 6,561-pair rational arithmetic oracle, canonical string serialization beyond binary64 integer precision, exact cancellation and explicit error boundaries. Strict Clippy and the WASM library check pass. See [profile and limits](DECIMAL.md).

This standalone-library checkpoint initially left source literal/type syntax and runtime schema integration open. The later numeric and ordinary scalar checkpoints below and above implement the bounded Decimal/nominal Quantity profile; full unit algebra remains open. Existing Float semantics are unchanged.

## Pinned node influence v0.11 verification

The compiler now uses the reviewed 0.11 contract with default-empty `Node.derived_nodes`. Public bootstrap syntax does not manufacture runtime origin envelopes; identically named data properties remain ordinary data. Runtime materialization attaches exact source NodeRefs, and subsequent composition retains their restrictions. Both assertion and node dependencies use a conservative AND policy; cycles, missing/denied dependencies and exhausted proof bounds fail closed. Node proofs and assertion proofs have separate identity namespaces.

All four actual CLI workflows (`check_geometry.py`, `check_counterparts.py`, `check_contexts.py`, `check_rules.py`) pass against the reviewed 0.11 engine. The counterpart script also checks exact source node pins. Root runtime tests independently cover copied isolated nodes, capsules, scalar influences, endpoint proxy bypass, signed scope retries and dependency bounds. Derived results requiring more than 1,000 combined assertion/node influences per node return an explicit budget error rather than silently truncating proof. Conservative AND gates can overrestrict alternatives; no release/declassification policy is implied.

## Exact numeric values v0.12 verification

65 language tests (60 conformance and five exact-arithmetic tests) pass. Source schemas now declare Decimal or an exact nominal Quantity descriptor; literals normalize exact strings without Float conversion. Bounded pure literal operators perform exact scalar and quantity arithmetic, including rational conversion with cancellation before representability checks. The canonical portable module is shared with the runtime; old schema descriptor bytes remain unchanged.

Actual `scripts/check_quantities.py` verifies compiler output persisted by the native engine, exact values beyond binary64 integer precision, pure expression results, pinned replay in a fresh process, unit revision mismatch/noncanonical-string/JSON-number rejection, and whole-program rollback. Old protocol profiles reject the new schema variants before writes. These are literal arithmetic and data validation capabilities: a graph-service conversion bound to authorized evidence, time, context and provenance remains open, along with quantity products, affine offsets, general numeric function parameters and vector schemas.

## Native graph services v0.13 verification

68 language tests (63 conformance and five exact-arithmetic tests) pass. Source `resolve_identity` and `cluster_navigation` bind reusable graph values from explicit stored snapshot pins and an explicit default/pinned context. Accepted identity requires an exact policy and mapping decision revision; plans cannot install policy, propose or accept mappings. Runtime current authorization/revocation remains authoritative. Clustering returns conservative scoped partial navigation, retains isolated source nodes and does not prune ordinary exact queries.

Actual `scripts/check_services.py` uses the engine's explicitly trusted `native_services_fixture` example to seed/accept/revoke an independent-ID mapping. It verifies native/source graph equality, pure function and nested-union reuse, no hidden events, revocation rollback, pinned clustering and old-profile preflight rollback. Contract 0.13 is an exact 18-file vendor snapshot at `8c488fbb0ddffe0521dda07e5af71c9566c8409a`. Computed-input clustering, pure-function service reads, source governance authority, private pairwise identity policy and full multiscale semantics remain open. See [native service usage and boundaries](NATIVE_SERVICES.md).

## Typed total contexts v0.14 verification

80 language tests (67 compiler conformance, eight portable axis tests and five Decimal tests), strict Clippy, formatting and WASM library checking pass. The exact 22-file contract distribution is pinned to engine `52a8f35d5b53ba477d957e59c753aa443cc39a05`; its separate portable suite validates strict embedded decoding, bounded carriers and generated proof propagation.

`python3 scripts/check_typed_contexts.py --engine /path/to/weave-engine` executes the actual compiler and runtime. It checks canonical Decimal axes, attributed descriptor commits, exact typed selection, support/unknown/explanation/function/union composition, private persisted scalar and empty values after record readers are cleared, generic descriptor denial/schema mismatch, old-protocol rejection and whole-program rollback. Source declarations remain public bootstrap data; the harness uses trusted setup to restrict the descriptor.

The carrier deliberately gates the whole graph value with all retained descriptor witnesses. Mixed unions and empty projections can therefore deny more than a minimal per-record policy would; they cannot erase a private context by dropping records. This is a documented conservative first profile. No implicit compatibility, broadcast, authority installation, reference-valued axis, hypothetical-world or complete-scope absence behavior is claimed. See [typed contexts](TYPED_CONTEXTS.md).

## Explicit live references on v0.14

83 language tests pass (70 compiler conformance, eight axis and five Decimal tests). `scripts/check_live_handles.py` runs actual source plans against the unchanged 0.14 runtime: ordered pin-before/after-head replacement, immutable reuse, separate-execution repinning, advancing live metadata versus historical fixed metadata, explicit CAS full-snapshot attachment replacement, current-principal filtering, explicit half-open time and stale-write rollback all pass. Missing top-level heads fail generically.

No shared contract bytes changed. Pinning captures one evaluation rather than installing a durable view; host registration, explicit refresh and tick policy remain separate native APIs. The source does not claim automatic push, incremental watch, policy revocation administration or per-field metadata patches. [Live references](LIVE_REFERENCES.md) records the complete supported boundary.

## Exact static schema function constraints

88 language tests pass: 75 compiler conformance, eight axis and five Decimal tests. The unchanged protocol 0.14 commands support graph parameter/return constraints checked by complete schema descriptor equality. Negative tests cover unknown/untyped/mismatched inputs, unsafe returns, synthesized algebra schemas, partial/higher-order/captured bypass attempts, nominal label conflicts and semantic source identity. Unknown runtime schemas produce an explicit diagnostic rather than an invented runtime check.

`scripts/check_schema_functions.py` compares actual annotated/unconstrained compiler commands and native runtime results. Higher-order, partial and captured calls agree with direct filters on graph/schema/provenance/snapshot data; an empty typed result retains its exact descriptor after persistence. Annotations add no command or effect. Strict lint, formatting, WASM and vendor/source verification pass. General dynamic schema assertions, graph-type completeness/metadata shapes and schema-polymorphic function types remain open.

## Source-only pinned modules (protocol unchanged at 0.14)

Implemented explicit content-pinned pure source linking, canonical module namespaces, dependency manifests, original-file diagnostics and a bounded local CLI module map. All imported effects remain forbidden; exact schema-constrained transitive functions, rules and context schemas compose without descriptor erasure. Modules do not install runtime authority. Full L01–L03/L26/L27 gates remain open.

Local verification: 102 tests pass (75 conformance, 8 context-axis, 5 Decimal, 10 linker, 2 CLI loader, 2 independently authored linker regressions); strict all-target Clippy, rustfmt, WASM library check, exact 22-file v0.14 vendor and three original paper hashes pass. The real `check_modules.py` compiler→engine run passes transitive capture, diamond manifest deduplication, direct-operation/result equivalence after intentional nominal schema ID alignment, and empty typed output preservation. Root publication/hosted verification remains separate. [Source module guide](MODULES.md) records supported behavior and bounds.

## Protocol 0.15 influence and selector pairing

The compiler emits `0.15.0` and vendors the complete 24-file canonical contract from engine `506729278cf3ee42a30eb9ac71336480e386fe89`, including `influence.rs`, its portable tests and MIT license, plus exact versioned documentation. Source bootstrap node/edge/claim values keep dependency fields empty; identically named ordinary properties remain data. The pinned-module implementation and all existing source operators remain intact.

Local checks pass: all 102 language tests, strict all-target lint, formatting, WASM library compilation, exact vendor verification and the three paper hashes. All fourteen actual source/runtime workflow suites pass: modules, schema functions, live handles, typed contexts, contexts, assertions, geometry, counterparts, quantities, finite rules, algebra, functions, composed values and native services. Independent engine metadata scripts also pass unchanged nested source selectors, all five metadata-host kinds, distinct/idempotent wrappers, cyclic paths, node-only privacy and projection error rollback. Original-node projection after metadata uses the documented current-result-ID rule; it does not gain a proof-based alias or silently return empty. See [pairing and compatibility](PROTOCOL_015.md). Full-design gates and remote publication remain separate.

Root review independently executed the three scalar boundary regressions, actual compiler/runtime persistence and fresh-process replay, and native/WebAssembly specialization parity (5,095 identical JSON bytes, zero WASM host imports). The exact 24-file protocol0.15 contract and three original-source artifacts were independently verified. The agent also verified a clean source archive with locked offline dependencies using the existing build cache; all123 tests are covered by the 122-test full run plus the 21-test scalar/root run after the final graph-input guard. Hosted full-suite confirmation follows publication.
