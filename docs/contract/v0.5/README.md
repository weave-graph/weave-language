# Weave compiler/runtime contract 0.5.0

This version adds pure Union, Diff, Project and Support graph expressions, pinned node origins, and persistent alternative derivation groups. It retains the 0.4 schema and metadata model. Earlier 0.1–0.4 plans remain accepted by the engine under their original feature gates; new expressions require 0.5.

`GraphExpression::Union {left,right}` combines exact immutable origin members. Different graph-local IDs never imply identical objects. Repeated composition retains stable origin identity. Revision-distinct historical records are not automatically independent corroboration. Typed inputs retain nominal schema meaning through synthesized type namespaces; mixed typed/untyped inputs require an explicit mapping and currently fail.

`Diff {before,after}` returns the union annotated with named `weave:diff:membership` literal attachments on added/removed nodes and edges. Unchanged members remain available without status attachments. This is immutable-origin snapshot membership: changing a revision yields removed/added records, not an inferred mutation correspondence. Removed assertions retain their original polarity. Attachment-only changes remain in the output but are not classified as membership deltas in this profile.

`Project {input,node_ids,edge_ids}` selects exact current-value IDs. Selected edges retain both endpoints and relevant named attachments. Requested absent members fail. Schema and derivation premises are retained, including premises outside the projected display graph. Snapshot/execution provenance remains available.

`Support {input,predicate,from,to,valid_at}` matches predicate and exact endpoint entity/space identities at one required Unix-millisecond instant. Its fixed typed status graph reports `unknown`, `supported`, `refuted` or `conflicted`. Positive and negative premises with disjoint validity do not conflict. Unknown never means false; partial coverage remains partial. No closed-world absence inference is implemented.

`Edge.derivations` is a list of alternatives (OR); each `Derivation.premises` list is joint support (AND). Groups retain operator parameters, their own premise snapshots, and parent derivation traces. The flattened `derived_from` and result `edge_origins` indexes are compatibility indexes, never Boolean support expressions. The engine authorizes groups separately and prunes invisible alternatives. Joining alternatives forms a bounded cross product; support preserves positive/negative alternatives separately, pairing them for a conflict explanation.

`QueryResult.node_origins` records engine-derived pinned source provenance, not cryptographic authentication. Executable plans cannot supply authoritative result envelopes. `AlgebraContext` is host-only, with principal/object/byte limits. Pure operator results are conservatively scoped to that principal. Object limits, cumulative serialized bytes and at most 128 derivation groups bound construction. These operations provide no effects, hidden commits, permission widening, identity inference, or complete-scope certificates.

The compiler vendors the portable reference implementation from the engine repository. The exact source commit and all file hashes are recorded in `vendor/manifest.json` when the paired checkpoint freezes.
