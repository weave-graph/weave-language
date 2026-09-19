# Typed total contexts (protocol 0.14.0)

A context is an exact graph revision, distinct from a representation space or branch. Typed axes describe that context; matching labels or equal axis values do not equate independently pinned contexts or grant access. This is a bounded implementation of paper §§1.1, 4.2 and 5.1, not a complete compatibility/world model.

```weave
context_schema OperatingWorld revision "1" {
  axis "region" enum ["EE", "FI"];
  axis "load" decimal;
}
transaction worlds {
  context_value Estonia schema OperatingWorld source "scenario-author" {
    axis "region" "EE";
    axis "load" decimal "0.750";
  }
}
use Claims graph "Evidence" revision "exact-evidence-revision";
typed_context Selected from Claims
  graph "Estonia" revision "logical:worlds:Estonia" schema OperatingWorld;
```

`context_schema` is an immutable compile-time declaration. Axis types are `boolean`, `integer` (signed i64), `string`, `decimal`, `quantity dimension "length" unit "metre" revision "1"`, and an exact string `enum`. Each context value supplies every declared axis exactly once. Null, Float conversion, extra/omitted axes, defaults and wildcards are rejected. Quantities require the identical nominal dimension/unit/revision; validation never authorizes conversion. Decimal source literals normalize without binary floating point.

`context_value` emits an explicit snapshot commit and requires the host's ordinary write grant. Its canonical self-relation has predicate `weave:context:definition`, assertion ID `definition`, mandatory claimed source attribution, positive polarity, default context and unbounded valid time. The `weave.context` assertion property contains the complete schema and assignment. Inside a named transaction, this ordinary graph can share atomic logical pins with claims. It does not install policy or confer trust in an author's labels.

`typed_context` emits a pure read expression with the input graph, exact descriptor graph/revision and complete expected schema. The runtime validates the descriptor and current access to its assertion and anchor before selecting the exact context. Missing, denied, malformed or incompatible descriptors fail with `E_CONTEXT_UNAVAILABLE`; they do not become a complete empty result. These forms are top-level declarations: a pure function can receive and return the resulting graph but cannot hide a descriptor read in its body.

## Closure and persistence

Graph data carries an optional `context_typing` envelope with a selected exact context and retained witnesses. Each witness contains the complete schema, exact definition AssertionRef and actual anchor NodeRefs. It is descriptive data revalidated against current authority, not a caller-supplied authorization token. At most 32 witnesses and 16 anchors per witness are retained.

Union, projection, joins, rules, support, explanations, metadata and native services preserve the dependencies. A mixed union may clear its selected type while retaining all witnesses. Generated conclusions include descriptor premise/anchor gates, and empty results retain the graph-level carrier. Saving, reloading or transporting a graph therefore cannot erase a private descriptor by dropping edges or clearing readers. Denied saved carriers yield generic partial results without schema, pins or contents.

This first profile uses a conservative whole-value AND restriction across retained witnesses. A public fragment combined with a private typed empty value may remain private even after projection; no minimal-release policy is implied. Immutable source records remain unchanged, while newly generated records receive their own influence gates.

## Validation and limits

Schemas have 1–32 axes; enums have 1–128 unique members. Identifiers and enum members are at most 512 UTF-8 bytes, ordinary string values at most 4096 bytes, and each schema/definition at most 64 KiB canonical JSON. The strict schema/definition entrypoints reject duplicate and unknown fields, including escaped duplicate names. Embedded expected schemas use the same custom decoder and cumulative budget. Stored assertion properties are generic JSON values: duplicate keys in an original raw property object may already have been normalized by generic JSON decoding before descriptor interpretation. The runtime validates the resulting stored canonical descriptor; it does not claim to recover or reject discarded raw duplicate keys. Source context declarations separately reject duplicate axes at their exact tokens. Complete carriers are bounded to 3 MiB and remain subject to runtime output/read limits. Same schema label/revision with a different descriptor conflicts within a declaration/composition environment; reads do not mutate a global schema registry.

Old protocols reject the new operator and carrier before writes. The [shared protocol](contract/v0.14/README.md), [source example](../examples/typed_contexts.weave) and [actual CLI acceptance](../scripts/check_typed_contexts.py) record the executable boundary.

Reference-valued axes, explicit compatibility mappings/broadcasts, governed world crossing, axis access in richer function/rule signatures and hypothetical assumptions remain open. Exact Default is never a universal context.
