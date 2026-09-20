# Temporal attachment proof handoff

Design only, accompanying experimental vendor `temporal.rs`. No version or store
marker is changed. Supersedes the generated match-attachment representation in
TEMPORAL_GRAPH_QUERIES.md: generated match details instead live in each occurrence's
`Derivation.parameters`, including relation/window/input intervals/exact origin
pins/role. Explain already exposes provenance. No new domain or convex-hull fact,
no schema property additions, and no artificial match metadata attachment.

## Existing metadata must remain navigable

Real input attachments remain graph data, not opaque provenance substitutes.
A Sequence occurrence from `L` with partner `R` requires both members; its copied
node, edge, and attached metadata cannot lose that restriction when detached.
Current `MetadataAttachment.derived_from`, `derived_nodes`, `derived_snapshots`
are declared global AND carriers. `influence::input_influence` deliberately
promotes them to the value carrier before a semantic operator removes records.
This behavior must remain unchanged for existing fields.

Suppose L has alternatives A or B and R has premise C. Pair-conditioned occurrence
alternatives are `(A AND C) OR (B AND C)`. Copying their existing metadata M into
attachments with flat gates produces `{A,C,M}` and `{B,C,M}`. A subsequent Filter
or Union then promotes both attachments' gates, accidentally requiring A AND B.
A new record-local `MetadataAttachment.derivations: Vec<Derivation>` would express
these alternatives, separately from existing explicit global carriers. Empty means
legacy behavior and omits on serialization. Flat `derived_from` must **not** become
a descriptive union of the new groups; `attachment_origins` is the separate
runtime descriptive index. New groups would not be promoted by input_influence,
just as ordinary edge alternative groups are not promoted today.

Storage authorization must require the existing global gates AND at least one
complete new derivation group when nonempty. It must authorize original attachment
premises and pair premises under the same shared recursion/depth/work context,
filter unavailable alternatives, and retain exact authenticated bytes for stored
integrity/export comparison. Dependency collection, provenance validation, capsule
closure, origin materialization and version rejection all need owner wiring.

## Empty graph-valued metadata shows a second required carrier

Attachment groups alone do not finish the semantics. Current native
`metadata_value` in `crates/weave-engine/src/metadata.rs` promotes its selected
attachment's `attachment_origins` into `GraphInfluence.assertions` before target
resolution (around lines 77–88). It subsequently copies those origins into output
nodes and edge proofs. This is deliberate protection for an empty/unavailable
target, not a redundant flattening that can simply be removed.

Counterexample: a Sequence-selected graph attachment M is justified by A OR B,
and its target is empty. Follow M, persist the empty result, then revoke A while B
remains readable. Correct reusable behavior retains B, while withholding both
must withhold the result. A flat AND carrier denies after revoking A; no carrier
releases an observation when both disappear. A pure operator cannot fabricate an
AssertionRef for its unstored generated attachment. Requiring save/read first
would regress the approved direct-composition profile.

The minimum candidate extension to assess is therefore:

- `MetadataAttachment.derivations` for record-local alternatives as above.
- `GraphInfluence.derivations` for a whole-value OR-of-AND path requirement, ANDed
  with its existing flat assertion/node/snapshot fields. Merging two values takes
  the bounded product of their alternative groups; empty groups mean no additional
  alternative requirement, not denial. An individual empty proof group is invalid.
- `Node.derivations` for generated isolated/scalar nodes retaining that alternative
  path when copied without the envelope. Existing Edge/Assertion derivations and
  the new attachment groups can compose the same path requirement. Node flat
  fields remain global AND. Ordinary node identity never comes from these proofs.

All three are restriction-only data, not authority. Use existing Derivation leaf
AssertionRefs/NodeRefs, preserving parent operator parameters and exact pins.
Existing flat snapshot carriers retain their declared whole-value AND semantics.
Add `Derivation.snapshot_premises: Vec<GraphRef>` (bounded, default and omitted when
empty) if a particular alternative needs a whole-snapshot restriction. This is
explicit authorization data, independently checked under the same whole-snapshot
visibility/guard semantics as flat snapshot influence. Source object references
also recursively enforce their own protected-source guards. Never infer these
authorization gates from descriptive `Derivation.input_snapshots`.

## Exact carrier algebra

For flat references F and groups G, visibility means `all(F) AND any(all(g) for g
in G)`. An omitted/empty G means no additional requirement, so F with empty G is
legacy behavior. An individual group with no assertion, node or explicit snapshot
premise is invalid even if it has descriptive pins/parameters. There is no encoded
false/deny-all carrier: unavailable proof alternatives yield a withheld result,
not a newly serialized empty group list that would turn denial into truth.

- Conjunction combines flat fields with canonical set union. If one side has no
  groups, retain the other's groups. Otherwise take the checked Cartesian product,
  unioning each pair's assertion/node/snapshot premise sets and retaining both
  parent derivation traces. Reject overflow before allocating combinations.
- Disjunction is only for alternative proofs of the **same** record/value, never
  for merging different graph inputs. Distribute each side's flat requirements
  into that side's alternatives, then concatenate/deduplicate the bounded groups.
  A side with no groups but nonempty flats becomes a group of those flats; a side
  with neither flats nor groups is an unconditional proof and makes the alternative
  requirement unnecessary. This operation is internal to trusted computation;
  serialized references still cannot authenticate user claims.
- Union of two graph values conjuncts their whole-value carriers. It must not use
  proof disjunction to release one input merely because the other is public.
- Normalize newly constructed reference sets by exact graph/revision/object keys.
  Normalize generated groups using the complete canonical derivation, including
  operator/parameters/descriptive pins, not just its leaves. Remove exact duplicates
  only; do not drop distinct explanations or infer implication/subsumption.
  Preserve original stored/signed group bytes and use normal full payload equality
  for authenticated integrity. Authorization may prune a delivered copy, never
  rewrite a stored revision or make a partially visible snapshot fully exportable.

Bound raw group count at 128 before decoding/cloning. Bound aggregate reference
occurrences across flat fields and all groups at 1,000 (before dedup), plus existing
shared byte/depth/work limits. Products are checked before cloning; a composed
carrier too large fails explicitly, not by selecting one convenient proof or
flattening alternatives. Input recursion and authorization use one shared visiting
set/work budget; group branching cannot restart a fresh budget. A cyclic branch
cannot manufacture support, and exhausting the overall budget cannot become an
unrestricted empty carrier.

These fields are default/omit-empty additions, preserving old DTO bytes and old
view/handler identities when absent. Any nonempty new attachment/node/whole-value
group or explicit snapshot premise requires the next jointly reserved protocol.
Recursive old-version validation covers raw Commit/CommitBatch snapshots, nested
expressions, template recipes and incoming capsules before any durable mutation.
Persisted semantics require the engine-owner-selected new store marker, so an old
binary cannot load a record while ignoring its OR gate. Existing older capsules
must reject new restriction fields; the owner must choose and test the next capsule
profile rather than silently reinterpreting an older format. Signed transport
responses must bind the actual negotiated capsule format as they already do.

This is a proposed coherent delta, **not yet approved or implemented**. If the
engine can express the empty path with an existing authorized carrier while
retaining OR, it should replace this addition with an explicit proof/example.

## Dropped-value and scalar-consumption boundary

Current ordinary edge proofs are record-local: merely removing an edge does not
promote all its alternatives into graph-wide AND influence. New attachment groups
would follow that distinction. Existing explicit attachment carriers still promote
unchanged, including when every attachment is removed. There is no weakening of
the existing declared-carrier guarantee and no new universal noninterference claim
for arbitrary deletion of ordinary original records.

Selecting graph-valued metadata is an actual path consumption and therefore must
add the selected alternative requirement to the whole-value carrier even for an
empty target. Operations consuming that carrier retain it through projection,
filtering, union/diff, context selection, windows, rules, Explain and persistence.
Generated node-only conclusions must also retain it locally. Support's unknown
status and other scalar/summary outputs need explicit regressions here; merely
adding node-level groups without teaching these consumers to compose them is not
sufficient. A generic reduction that consumes several separate records needs their
AND of OR groups (bounded product), not a union of all proof leaves.

## Required executable counterexamples

1. Sequence with A OR B and C, then Filter and Union. Original event fields/schema
   stay exact; match parameters/roles survive; no new match attachments are emitted.
2. Attach a real Graph payload to a selected occurrence. Navigate before saving;
   revoke A, retaining B: both original attachment payload and target stay visible.
   Revoke both: withhold. Repeat with empty target and subsequent Support/Explain.
3. Strip result envelope/readers and copy each generated node, edge, attachment
   separately. Private pair C remains required; alternatives A/B remain alternatives.
4. Existing attachment flat gates still survive complete attachment removal, and
   old wire omission bytes remain exact. Reject new groups under old protocol before
   any writes; old binaries refuse the new store format when persisted semantics
   could be ignored. Whole-visibility/export comparison must not normalize away
   unavailable derivation alternatives or changed group payloads.
5. Reject empty groups, bad IDs, cycles, excessive depth, more than 128 alternatives,
   products above the allowed count, more than the agreed combined reference budget,
   and output bytes before clone/retention. Native and portable tests share fixtures.

Engine owns the canonical enum/version/store/auth boundary after facade A freeze.
Language's current vendor-only prototype must not be integrated as a finished
canonical contract until this composition seam is resolved and exercised natively.
