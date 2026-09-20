# Weave language implementation plan

Status: proposed execution plan, 2026-09-19. No implementation is claimed by this document.

## Evidence and scope

The original [language white paper v0.1](source/Weave_Language_White_Paper_v0.1.md) and companion engine paper were supplied on 2026-09-19 and reconciled in [SOURCE_RECONCILIATION.md](SOURCE_RECONCILIATION.md). Exact source integrity is recorded in [provenance.json](source/provenance.json). They are architecture proposals: illustrative syntax is not a ratified grammar. Source recovery is complete; conformance and implementation gates remain open.

The user's scope includes the full multidimensional graph language and its cooperation with the decentralized event-driven runtime. The first executable release is one milestone, not a redefinition of completion. All rows below must be implemented and independently verified, or explicitly amended by the owner, before calling the requested scope complete.

The user's later clarification governs the design: a dimension is a space in which an entity has a manifestation linked to its counterparts. Contextual axes remain useful but do not substitute for this requirement. Space, branch, replica, entity, manifestation, assertion, graph, and revision are separate concepts.

## Ownership and architecture

The language project owns grammar, typed AST, diagnostics, schema and module checking, pure graph-producing computation, rule semantics, plan lowering, tooling, and the executable language conformance suite. The engine owns persistence, commit recording time, atomic effects and events, capabilities, subscriptions, adapters, distribution, materialization, clustering execution, and device lifecycle. Optional LSP/editor enhancements are proposed conveniences and do not block completion of the user-required language/runtime semantics. Both projects own the versioned protocol and cross-project conformance fixtures; neither may weaken the other's semantics through a private shortcut.

Use a Rust workspace because the semantic core can compile to native CLI and WebAssembly while avoiding an unrelated mobile implementation. Proposed crates:

- `weave-syntax`: tokens, spans, parser, lossless syntax where useful, and formatter.
- `weave-types`: schema/type/context/effect checking and stable diagnostic codes.
- `weave-ir`: versioned serializable plans and protocol validation.
- `weave-eval`: deterministic pure evaluation against an abstract snapshot source; no ambient network, clock, filesystem, or credentials.
- `weave-cli`: `check`, `fmt`, `describe`, `plan`, `run`, and structured output.
- `weave-wasm`: bounded host interface with the same conformance fixtures.
- `weave-lsp`: schema completion, errors, references and hover information.

Start with a deliberately small hand-written parser and precise EBNF. A parser library is a replaceable implementation choice. Avoid coupling semantics to a grammar framework or database. Native and WASM use the same typed IR. External adapters are engine-hosted effects, not hidden evaluator calls. Core algorithms must accept cancellation, fuel and output limits. A development in-memory host is a reference oracle, not a substitute for engine integration.

Dependency selection requires current upstream documentation, license review and a checked-in lockfile when implementation starts. Portable core code must not assume a process, socket, wall clock, filesystem path, or operating-system thread.

## Requirement to deliverable and verification matrix

Class **U** means an explicit recovered user requirement; **P** means a proposed semantic commitment necessary to make that requirement precise. Proposed commitments require approval through versioned architecture decisions rather than being misrepresented as original user mandates.

| ID | Class | Requirement | Deliverable | Required acceptance evidence | Gate |
|---|---|---|---|---|---|
| L01 | U | AI-authored complex graph language | EBNF, parser, structured diagnostics, CLI and schema discovery | Valid programs parse; malformed programs report stable code and exact span; no host effects from input text | L1 |
| L02 | U | Graphs compose and are first-class values | Graph types, graph references, graph-returning functions/lenses, bind and composition | A lens result feeds another lens; bound and unbound evaluations agree at identical snapshots | L2 |
| L03 | U | Parameterized graphs | Typed value, graph and function parameters | Parameter mismatch is rejected; parameter identity contributes to a view's identity | L2 |
| L04 | U | Joinable graphs | Named argument matching, union, joins and explicit resolution | Union preserves assertions; joins cannot silently resolve contradiction or equate entities | L2 |
| L05 | U | Temporal knowledge | Valid-time operators, pinned system snapshots and temporal IR | Half-open boundary, late correction and disjoint-time fixtures; old query remains reproducible | L2 |
| L06 | U | One entity in multiple dimensions | Space, manifestation and protected counterpart operations | Shared entity identity with independent per-space state; incomplete/offline membership reported | L3 |
| L07 | U | First-class nodes and edges | Stable node/edge IDs and directed edges; optional n-ary extension | Metadata can target either node or edge; moving coordinates does not reverse an edge | L1 |
| L08 | U | Metadata may itself be graphs | Typed GraphRef metadata, pinned revisions, nested query/traversal | Query graph-valued edge evidence; shared references preserved; cycle and missing-reference cases terminate with explicit status | L3 |
| L09 | U | Automatic recursive clustering and zoom | Typed clusterize/expand/inspect plans and graph-valued clusters | Arbitrary requested finite depth under fuel limit; membership/source/method retained; no invented detail | L5 |
| L10 | U | 3D graphs and vector directionality | Frame, unit, coordinate, displacement and transform types | Compatible transforms compose; incompatible frames/units rejected; physical and relational direction differ | L5 |
| L11 | U | Embedding dimensions and bridges | Embedding-space descriptor, similarity operator, typed bridge plans | Encoder/version/metric/preprocessing mismatch rejected even at identical vector length; learned bridge cannot claim identity | L5 |
| L12 | U | Distribution and decentralization | Remote graph references and snapshot vectors | Multi-peer query records per-peer revisions; unavailable peer yields partial coverage, never false emptiness | L6 |
| L13 | U | Attachment and detachment | Distinct mount, unmount, replicate, fork and integrate effects | Mount never accepts claims; unmount does not delete retained snapshots; operation diagnostics match engine | L6 |
| L14 | U | Permissions and visibility | Capability-aware effects, discover/read/traverse separation and restriction propagation | Hidden counterpart existence, counts, explanation and derived data do not leak through language results | L4 |
| L15 | U | Optional governance | Exact accepted-occurrence reads implemented; proposal, policy-version and acceptance effects remain | Personal direct acceptance and governed approval both work; policy mutation cannot grant caller authority | L6 |
| L16 | U | Offline branching on mobile/personal devices | Host-independent semantic core, WASM and native SDK surfaces | The same scenario runs disconnected with missing dependencies marked, then synchronizes and integrates by explicit policy | L6 |
| L17 | U | Event bus and reactive adapters | Typed event patterns, watch/subscription and declared effect plans | Commit triggers a reactive plan through actual engine bus; duplicates/replay do not duplicate logical language outputs | L4 |
| L18 | P | Context-aware composition | Context schema, applicability/broadcast and world checking | Omitted dimension does not mean universal applicability; unauthorized or implicit world crossing fails | L2 |
| L19 | P | Explicit identity alignment | Versioned identity-map plans and distinct similarity relation | Ambiguous mapping remains candidate; identity-map revision appears in provenance and snapshots | L3 |
| L20 | P | Explainable graph results | Derivation graph, source spans, dependency identities and immutable lens revision | `explain` traces every emitted assertion to exact premises, plan and input revisions | L3 |
| L21 | P | Contradiction, uncertainty and open world | Four support states; explicit closed-world/completeness assumptions; typed assessments | Positive and negative claims coexist; absence is unknown; extraction score is not probability by coercion | L2 |
| L22 | P | Safe recursion | Range-restricted finite-domain rules, stratified explicit negation and resource budgets | Recursive reachability terminates; unsafe variables/cycles through negation rejected; fuel exhaustion reported distinctly | L3 |
| L23 | P | Scenario evaluation | Branch assumptions, masks and explicit comparison plans | Hypothesis does not mutate baseline; assumption metadata survives explanation; no causal-inference claim implied | L3 |
| L24 | P | Incremental live graphs | Explicit Query/Filter registration artifacts and current reads implemented; general watch plans, rolling windows and support deltas remain | Incremental output equals full recomputation across insertion, correction, deletion and expiry | L4 |
| L25 | P | Coverage is part of every result | Complete/partial/local-fragment/unknown manifest and structured unavailable dependencies | Empty complete, empty partial, denied and unsupported outputs stay distinguishable without revealing hidden data | L2 |
| L26 | P | Referenceable graph objects | View identities bound to plan, schemas, parameters and snapshot vector | Same canonical inputs give same view identity; changing any semantic dependency changes it | L3 |
| L27 | P | Safe input and explicit effects | Data/code boundary, source validation, capability declarations and effect checker | Ingested adversarial text cannot execute or authorize; pure lens rejects write/network/LLM effects | L1/L4 |
| L28 | P | Exact and approximate semantics remain distinct | Query mode and approximation metadata, sound pruning rules | Approximate clustering never prunes exact search without a proven bound; approximation is visible | L5 |
| L29 | P | AI-friendly development | Machine-readable AST/diagnostics, `describe`, formatter and optional LSP | Tool integration discovers a schema, repairs a type error and runs a valid plan without guessing fields | L7 |
| L30 | P | Stable public ecosystem | MIT source, public RFCs, versioned packages/docs, examples and migration notes | Fresh clone builds/tests offline after dependencies are cached; public CI, reproducible release and license checks pass | L7 |

## Shared engine contract proposal

The authority should be `weave-engine/docs/contract/v0.1` with a version/hash-pinned vendored copy in this repository, plus bidirectional fixture tests. A runtime-independent shared crate may later be published only when both repositories can independently build from its tagged source. During bootstrap, a JSON schema plus normative semantics avoids a circular dependency.

The protocol must define:

1. Opaque stable IDs for entity, manifestation, space, graph, edge and assertion; immutable revision IDs; no identity inference from string spelling or content hashes.
2. First-class nodes and directed edges; optional n-ary named-argument assertions; metadata values that include scalar, typed artifact or graph reference. Graph references include graph identity, pinned revision, optional scope and resolution state. Recursive graph values use references, never recursive inline expansion.
3. Assertions with polarity, valid interval, contextual scope, source, derivation and restriction metadata. Engine-owned system revision/time cannot be supplied authoritatively by a client.
4. A `SnapshotSet` pinning every input graph, schema, map and policy revision needed for replay. Pinning separate peers does not assert globally atomic observation.
5. Pure query plans and separate atomic transaction/effect commands. Plans declare capability requirements, exact/approximate mode and budgets; the engine independently validates authority at execution.
6. Results with graph data/reference, snapshots, assumptions, provenance, coverage, unresolved dependencies, diagnostics and budget outcome. Permission-denied output must not expose protected existence.
7. Versioned event envelopes with event ID, causation/correlation, commit/revision, subject, type, payload and restriction context. Subscriptions bind a cursor and filter. At-least-once delivery implies explicit idempotency, not unsupported exactly-once claims.
8. Revisioned cluster memberships, geometry-space descriptors, typed bridges, governed proposals and portable capsule manifests. Unsupported operations fail explicitly rather than silently degrading.
9. Canonical byte encoding, number/time/interval constraints, deterministic ordering, unknown-field/version handling, maximum sizes and public conformance fixtures.

The engine decides commit atomicity, capability revocation and offline acceptance. The language checks statically known constraints and emits repairable diagnostics; it cannot override runtime checks. Wire IDs and hashes are data, never execution authority.

## Staged gates and completion criteria

Each gate ends with a tagged, reviewable artifact and evidence in `docs/STATUS.md`. A stage cannot be marked done from code existence alone.

| Gate | Prerequisites | Work and exit test |
|---|---|---|
| L0 Source and contract | Original source recovery, owner/engine review | Reconcile both papers; ratify semantic glossary, protocol v1, EBNF direction and requirement coverage. Record open questions and paper hashes. |
| L1 Front end | L0 | Parse/type-check a graph with entities, first-class edges and graph metadata; reject effect/typing violations with JSON diagnostics. Formatter round trip and hostile-input limits pass. |
| L2 Deterministic semantic kernel | L1 + engine snapshot model | Graph-valued lens composition, temporal/contextual joins, four-valued support, coverage and snapshot reproducibility pass oracle/property tests. |
| L3 Full knowledge semantics | L2 + engine identity/reference support | Manifestations, metadata recursion, identity maps, provenance, safe rules, scenarios and view identities pass exact expected fixtures. |
| L4 Reactive secure integration | L3 + engine atomic bus/capabilities | Actual engine transactions, reactive watches, authorization and differential-maintenance tests pass; crash/replay and information-flow cases are covered. |
| L5 Spatial and multiscale language | L3 + engine geometry/clustering | Typed geometry/embedding/bridge and recursive clustering/semantic zoom pass budget, compatibility, provenance and exactness checks. |
| L6 Distributed offline governance | L4 + L5 + engine sync/governance/mobile interfaces | Two peers and disconnected native/WASM client edit edge evidence, reconnect, preserve history/conflict and complete acceptance with explicit coverage. |
| L7 Tooling and public release | L1–L6 | CLI/docs/examples/packages, platform CI, interoperability/security review and full requirements audit pass. No unresolved required capability is labeled complete. |

## Test and review strategy

Use golden syntax/diagnostic fixtures for the language contract, property tests for interval intersection/canonicalization, and differential tests against simple full recomputation. Avoid tests that merely duplicate an implementation. Cross-project fixtures are the primary interoperability contract and run from both repositories on every change. Tests include denial and partial-failure behavior, not only success cases.

The end-to-end acceptance story is the paper's offline phone editing an edge's evidence graph: inspect the edge and graph-valued metadata, fork while disconnected, append evidence with source and valid time, run a local lens with explicit limited coverage, trigger permitted local reactions, reconnect and transfer revisions, process governance, observe one logical accepted change, and explain the accepted derived result from pinned evidence. Repeat with duplicate delivery, conflicting evidence, unavailable external references and revoked authority.

Performance targets must be measured and ratified before optimization claims: representative graph sizes, recursion depth/fuel, memory ceilings, cancellation latency, native/WASM startup and increment-maintenance latency. No unmeasured scale promise is a release criterion disguised as a result.

## Research and decisions still open

- The complete source papers are recovered. Use their explicit conformance tests and SOURCE_RECONCILIATION.md; optional n-ary relations and paper research questions are not mandatory completion gates.
- Final surface grammar and the distinction between total pure lenses and general resource-bounded functions.
- Stable identity and canonical view encoding, typed timestamps and interval infinities; cryptographic integrity is engine-owned but byte semantics are shared.
- Provenance size/retention, shared/cyclic metadata reference traversal and denial-versus-unavailability diagnostics that avoid existence leakage.
- Exact recursive negation/aggregation semantics and which incremental forms can be maintained efficiently.
- Policy for federated consistency, unavailable identities, resource reservations and deterministic distributed results.
- Plugin/adaptor ABI stability, sandboxing boundary and whether mobile access uses WASM, a C ABI or both; actual iOS/Android build validation is required before portability claims.
- Clustering stability, adaptive zoom preservation of anomalies/conflicts and formally sound exact-search pruning.
- Research on feedback-strengthened traversal routes from the related AI discussion is not automatically folded into language semantics. If added, route usefulness must remain separate from evidential truth and gets its own approved requirement/gate.
