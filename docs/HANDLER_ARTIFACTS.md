# Compiled event handlers

The compiler implements inert protocol 0.18 handler artifacts. Actual compiler-to-native install/prepare/complete acceptance has passed locally; paired public CI and final release evidence are tracked in [status](STATUS.md).

A handler names a pure graph function and an explicit event subscription. Compilation produces a sealed recipe and does not subscribe, read a store, install an adapter, grant permissions, or run an external effect.

```weave
function Keep revision "1" (graph input) {
  return input;
}
handler Copy revision "1" using Keep {
  input event graph "Installation" branch "main" metadata depth 4;
  on "graph.accepted", "graph.committed";
  output slot "copy";
  replay pinned;
}
```

`weave artifacts file.weave` returns the Program, typed scalar values, view templates, handler templates and their complete fingerprint. `weave handler-plan file.weave --handler Copy` explicitly selects one handler artifact. `weave check` reports the artifact counts and host registration requirement. The library exposes `compile_artifacts` and `LinkedProgram::compile_artifacts`. Legacy plan, values, specialization, execution-oriented describe and fingerprint APIs reject unreturned templates with `E_HOST_ARTIFACT_REQUIRED`. Infallible linked AST/schema access remains inspection.

The named function may be partially applied to concrete scalar values or pure callbacks. It must have exactly one remaining unconstrained graph parameter and a graph result. Graph captures, including nested callback captures, are rejected. Unknown event schema cannot satisfy a schema-constrained parameter; the compiler reports `E_HANDLER_SCHEMA`. No implicit schema erasure or speculative runtime validation is inserted.

Recipe specialization starts with the internal `$event` graph binding. Authored identifiers cannot spell it. Every compiled binding references earlier bindings or `$event`; there is no synthetic Query. Filtering, joins, algebra, exact context selection, preloaded metadata navigation, finite rules, explanations, declared counterpart selection and geometry remain subject to their existing pure semantics. Typed-context loading, accepted/current views, identity resolution, clustering and other store reads cannot run inside the recipe. Unused function bodies still receive normal static validation. Recursive shared recipe validation provides an independent check at the native boundary.

Both authentic event types are required. `MetaGraphRebound` is not an alias. The metadata depth is an integer from zero through eight. The native host loads the exact event revision and declared metadata closure; a recipe cannot request an extra traversal through an unbound live head. Pinned replay means historical input, not a guarantee of the latest projection.

## Identity and bounds

The shared sealing helper binds the protocol, handler name/revision, complete pure recipe, input graph/branch/depth, canonical event set, output slot and canonical source manifests. Entry comments, whitespace and module aliases do not affect normalized identity. Imported module bytes remain exactly pinned: changing even a comment requires a new pin and changes aggregate identity. Graph, branch, slot, predicate and property strings that resemble `alias::member` remain literal strings.

No-handler complete artifacts omit the handler map and retain the existing complete-artifact fingerprint profile. Artifacts containing handlers use `weave-compiled-artifacts-v2`; callers must fingerprint the complete output. Function and Program identity algorithms are unchanged, apart from the emitted protocol version.

At most sixteen total view/handler artifacts and four MiB of estimated retained artifact data are accepted per compilation. Each handler is bounded to one MiB, 256 bindings, 1,000 total expression nodes and expression depth 32. All handlers share the existing specialization statement, byte and scalar-work budgets. Source/module bounds continue to apply. Unsupported reads, duplicate/event aliases, invalid signatures, unknown schemas and over-budget expansion produce explicit diagnostics; linked errors retain the original module locations.

## Trusted host boundary

The host installs a sealed template with an immutable adapter manifest and maps the output slot to a specific writable graph and branch. A slot string supplies no authority. This stage provides no source syntax for principals, grants, clocks, leases, network destinations or effects.

Preparation checks the actual leased event and current complete authorization of the exact event/metadata closure. Empty and isolated-node inputs retain exact snapshot influence without fabricated assertions. The native bridge evaluates the validated recipe and durably stores one exact Commit, its captured output CAS, template/source identity and private dependency closure. Completion accepts only the opaque preparation identity. Renewed leases and duplicate receipt recovery do not rerun changed source or rebase stale output CAS. Current authority is rechecked before completion and receipt reuse.

Generated output receives installed-principal restrictions and all required assertion, node and snapshot influences, including protected attribution metadata. That informational attachment records the template digest, source manifests, computed coverage and bounded diagnostics. Ordinary later Query coverage describes current visibility of the stored snapshot; reading an arbitrary attachment does not reconstruct the earlier computation's completeness, authenticate its source manifests, or grant authority. Diagnostics carry the same privacy restrictions as their inputs.

The bridge does not imply external-effect atomicity, sandbox execution, automatic release/declassification, cancellation, automatic CAS rebasing, or complete reactive-language conformance. Existing effect-ledger APIs remain separate.

## Acceptance

`examples/handlers.weave` supplies the actual Metadata → Reason recipe. `scripts/check_handlers.py` uses the compiled artifact with the trusted `compiled_handler_fixture` native example, rather than a handwritten substitute. Compiler tests cover seeded purity, artifact completeness, callbacks, captured graph rejection, schema obligations, genuine event types, module identity, original error locations, formatting and bounds. The actual fixture also verifies original object provenance after owned-ID remapping, four before/after preparation/completion process-death boundaries, immutable lease renewal, private detached-record reads and stale-CAS rollback. Its JSON report distinguishes fixture files, compiler/native processes and named verified behaviors. Native migration and paired publication remain separately reviewed.
