# Weave protocol v0.4.0 integration notes

Version 0.4 adds an explicit schema profile and the engine's named metadata/local snapshot-manifest operations. The previous protocol versions retain their declared feature gates. The portable contract, including `schema.rs`, is copied from the engine and verified by the vendor manifest.

## Typed graph profile

`GraphData.schema` optionally embeds a `GraphSchema { id, revision, nodes, edges }`. Nodes and edges independently carry optional `type_id` values. A typed graph requires every object to have a declared type. Type IDs without a graph schema are invalid; they are not guessed from predicates or entity names.

`NodeSchema` defines scalar property fields, an optional exact space ID, and whether extra properties are allowed. `EdgeSchema` defines required source/target node types, scalar property fields, explicit cross-space permission, and whether extra properties are allowed. Properties are closed by default. `PropertySchema` declares `string`, signed 64-bit `integer`, or `boolean`, with independent required and nullable flags. Decimal, vector, graph-valued field schemas, schema variance and migrations are not implemented by this narrow scalar profile.

The same `validate_schema_graph` function runs in source compilation and at the engine boundary. It has no I/O and grants no authority. It checks descriptors, required/nullable fields, unknown properties, node spaces, edge endpoint types and explicit cross-space permission. Runtime authorization, historical structural identity and immutable schema-revision registration remain separate runtime obligations.

Absent optional schema/type fields are omitted during serialization to preserve historical untyped revision bytes. Schema identity is separate from object and space identity. Query/filter results retain schema meaning. A join of two typed inputs synthesizes collision-safe namespaced output node types and explicit derived edge types. A mixed typed/untyped join is rejected rather than silently dropping meaning. The engine registers immutable schema descriptors by ID/revision and rejects incompatible reuse.

## Named metadata and local transaction manifest boundary

The shared runtime contract also declares addressable `MetadataAttachment` records: host kind and identity, field key, value, valid time, origin, readers, required flag and schema revision. Values can be literal, pinned graph, explicit live graph, object or artifact references. These records are separate from the earlier anonymous reference list.

`CommitBatch` contains a local batch ID and multiple snapshot commits. Graph references can name logical revisions resolved through that committed snapshot manifest, separating logical revision identity from the manifest's integrity digests. This is the engine paper §2.1 design for finite serialization of cyclic knowledge references. It does not create cross-peer atomicity.

These protocol types do not imply every operation is yet exposed by source syntax. Consult [verified status](../../STATUS.md) and the engine's conformance tests for the exact runtime implementation and language coverage at each release.

## Native metadata graph values

`GraphExpression::Metadata { input, host, key }` selects one named attachment from an already-materialized value. It reuses the original query's resolved metadata snapshots and performs no hidden live reads. The result retains schema, pinned input revisions, complete/partial coverage and the attachment traversal path. `attachment_origins` and `edge_origins` preserve the path through nested selections so a downstream join retains the attachment as a premise.

The source frontend exposes initial typed snapshots, explicit local transaction batches and named pinned attachments. It does not yet expose live metadata handles, arbitrary attachment origin/policy fields or source-level expected-head updates.

No-op writes have explicit `Unchanged`/`BatchUnchanged` results; no successful change event is fabricated. `CommitReceipt.event_id` is optional because unchanged aliases have no new occurrence.
