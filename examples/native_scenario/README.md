# Source inputs for native scenario B

These are executable compiler inputs for the engine's
`docs/proposals/NATIVE_SCENARIO_B.md`. They do not implement the trusted host
journal, install adapters, select principals or grant authority. Each variant's
manifest is **trusted test-controller configuration**, not a source capability.
The engine process harness consumes it as configuration and loads complete SDK
responses without embedding fixture names in runtime code.

`base` and `renamed` change graph IDs, module/rule revisions, the warning predicate,
attachment/edge selectors, handler names, adapter IDs and output mapping together.
Both retain the same semantic sequence:

1. `seed.weave` atomically creates positive baseline evidence and operational plus
   physical manifestations. A real graph-valued edge attachment pins its logical
   sibling. Properties include Unicode and the exact integer 9007199254740993.
2. `diagnostics.weave` is a pure content-pinned module: navigate edge metadata,
   derive a positive warning from an explicit **negative** quality assertion, then
   select warnings at valid-time 7. Absence alone does not satisfy that rule.
3. `handler.weave` retains two scalar values and two templates in the complete
   artifact. Only the selected handler is installed by the host. Both authentic
   event types are declared because the current sealed-handler profile requires
   `graph.accepted` and `graph.committed`; this scenario may exercise committed events.
4. `offline.weave.in` atomically replaces both source graphs using exact expected
   revisions and binds the installation to the new logical evidence sibling.
5. `cluster.weave.in` selects the exact warning event revision; it does not read a
   latest head or claim a source-level cluster reactor.

The module hash is already embedded in `handler.weave` and recorded in the
manifest. Read every file as exact UTF-8 bytes. Changes to module comments or
formatting require a new content pin and relinking.

Each `{{..._REVISION_JSON}}` placeholder is **outside quotes**. Replace it exactly
once with `json.dumps(exact_revision, ensure_ascii=False)` (or equivalent complete
JSON string encoding) before compiling. Never interpolate unescaped string content
or modify a compiled Program to insert pins. The controller obtains these revisions
from its actual committed snapshots/event; compilation does not attest authority.

`module-map.json` is a local CLI resolver map. For SDK compilation, provide exact
`{id, revision, source}` units from the manifest's `modules` list. Keep the raw SDK
request/response bytes and complete inventory, including the unselected template.

Run compiler-only validation without building or executing an engine:

```sh
python3 scripts/check_native_scenario_sources.py \
  --compiler /path/to/weave --sdk /path/to/libweave_compiler_sdk.dylib
```

This checks actual compiler/SDK identities, complete inventory, negative rule
polarity, exact CAS substitutions, graph-valued metadata and the pinned Cluster
selection in both variants. Native durable completion, privacy, crash recovery,
transfer, effects and mobile/browser scenario evidence belong to the separate
engine harness; fixture compilation alone proves none of those outcomes.
