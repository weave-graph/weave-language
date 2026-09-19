# Source reconciliation: architecture papers v0.1

The user supplied the original paired papers on 19 September 2026. The [language paper](source/Weave_Language_White_Paper_v0.1.md) is preserved byte-for-byte alongside the original [PDF](source/Weave_Language_White_Paper_v0.1.pdf) and [DOCX](source/Weave_Language_White_Paper_v0.1.docx), with SHA256 provenance in [the source manifest](source/provenance.json). Both papers are dated 11 September 2026 and describe proposed conformance requirements, implementation choices and research questions. Their code is illustrative syntax, not a formal grammar to claim we already implement.

The language and companion engine papers have been read against the current code and all 30 plan rows. Source retrieval is complete. Full semantic conformance is not. References below use language-paper sections prefixed **L** and companion engine-paper sections prefixed **E**.

## Corrections to the earlier reconstructed plan

1. **N-ary relations are optional (L §2).** Binary typed edges with their own identities are mandatory; optional named n-ary generalization must not block full required-scope acceptance.
2. **Graph metadata has named attachment identity and context (L §§3.1–3.2; E §2).** A vector of pinned graph references is a useful partial implementation but does not implement keyed, independently versioned, attributable attachments with valid time and visibility. Inline graph construction/rebinding must be atomic with its local host transaction.
3. **Metadata cycles have an explicit storage design (L §3.3; E §2.1).** Logical object references resolve through a snapshot manifest, so knowledge cycles do not require cyclic hashes. The current protocol's external revision-only references cannot express that whole design. The missing implementation is no longer categorized as an unanswered paper question. Canonical byte encoding and exact protocol remain engineering decisions.
4. **Structural edge identity is immutable (L §2; E §2).** Retargeting endpoints or changing predicate creates a new edge ID. Current snapshot replacement must be reviewed/enforced against history by the engine; source validation alone is insufficient.
5. **Time is replica-relative (L §5; E §9).** Valid time, local receipt time, causal checkpoint and optional accepted-view time differ. A Unix timestamp alone cannot claim universal system-time ordering.
6. **Composition must retain joint versus alternative support (L §§4.2,5.2).** Current leaf origins preserve dependencies of an executed path; the general explanation model must distinguish one conjunction from several alternative derivations, and include rule/schema/attachment revisions and parameters.
7. **Metadata permission is path-sensitive (L §9; E §10.1).** Host, attachment and target restrictions all apply to traversal. A public target independently readable by another route must not be globally relabeled private by one private host.
8. **Initial reasoning is finite-domain/range-restricted, with stratified negation over complete scopes (L §5.2).** General unbounded evaluation, arbitrary arithmetic recursion and implicit closed-world negation are excluded from the guaranteed-termination profile.
9. **Adapter effects need a ledger and explicit uncertain outcomes (L §10; E §4).** Local deduplication is a partial foundation, not proof of exactly-once external actions or full lifecycle/isolation/replay semantics.
10. **Research and optional delivery stay separate.** L §12/E §13.2 explicitly leave variance, identity-split propagation, cryptographic replication, provenance compression and formal assurance open. Reward-based traversal is optional (E §13.2). Editor LSP, n-ary extension, crates.io packages and one particular algorithm are not added mandatory user deliverables.

## Current executable profile and next implementation work

Protocol v0.3 implements public nodes/edges and scalar/pinned-reference metadata, valid intervals, trusted-host runtime authorization, immutable named graph results, typed relation/time parameters, partial application, temporal path joins and preserved leaf provenance. Native integration and source distribution have been checked. This is a coherent **partial** profile; calling it full compliance with L §12 would be incorrect because typed schemas, explicit space bridges, full capability semantics and reactive language operations are missing.

The next language milestone implements versioned graph schemas, distinct node/edge types, scalar metadata shape checks, typed endpoints and space constraints, plus machine-readable schema discovery. The same portable schema validator must run at the engine boundary so direct JSON clients cannot bypass the language checker. Its protocol additions are coordinated with the engine's metadata/snapshot-manifest work before release.

Subsequent work follows the existing DAG: named metadata queries and local atomic graph construction; richer graph algebra and complete-scope reasoning; explicit spaces/bridges; reactive effects and incremental views; offline language operations/governance; geometric/multiscale service calls. Each scope needs real tests and cannot be completed by labeling all missing operations as external adapters.

## Paper conformance checklist

| L §12 test | Current evidence | Remaining acceptance |
|---|---|---|
| Edge evidence graph can be queried and joined natively | Engine resolves pinned metadata into results | Language must name the attachment, use its graph as a first-class source and join it while retaining attachment provenance. |
| Metadata graph refers to host | Reference representation is finite; unavailable refs report partial | Implement logical within-snapshot manifests; construct and traverse a real cycle with explicit budget outcome. |
| Shared metadata advances, pins stay unchanged | Historical revision queries exist | Named attachments, explicit rebind/live distinction and a dedicated historical attachment test. |
| Concurrent peers edit one attachment | Local engine branch/capsule groundwork | Keep both attachment versions and policy-selected interpretation through language-visible conflict queries. |
| Shared identity, independent local manifestation state | Distinct entity/space values supported | Protected identity-link set, counterpart coverage and explicit propagation semantics. |
| Disjoint valid-time premises emit nothing | Tested actual cross-project join | Preserve in general joins/rules and query-window intersections. |
| Missing offline reference is not false | Partial metadata coverage, no closed-world language operator | Complete-scope enforcement when negation/rules are added. |
| Duplicate event yields one idempotent local effect | Engine receipt tests | Source reactor declaration, scoped subscription and end-to-end duplicate delivery test. |
| Private input cannot emit public derivative | Engine authorized query/join tests | Full metadata-path, explanation, event, cluster, geometry and release-policy tests. |

The complete [acceptance gap matrix](ACCEPTANCE_GAPS.md) now refers to recovered sections rather than requesting unavailable source. The [status](STATUS.md) and workflow distinguish recovered source from implementation and public-release verification.
