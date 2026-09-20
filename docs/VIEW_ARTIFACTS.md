# Accepted graphs and explicit view artifacts (protocol 0.16)

Accepted reads and current-view reads produce immutable graph values. View templates produce separate host registration artifacts. Compilation performs none of these host operations and never supplies authority or the authorization clock.

```weave
accepted Approved view "fleet-review" decision "exact-decision-occurrence";
lens Connections from Approved { match relation "connected"; at 5; }

view_current Current view "fleet-active"
  definition "sha256:0000000000000000000000000000000000000000000000000000000000000000"
  tick 5;
```

The example digest is a placeholder; use the digest emitted from the exact compiled template installed by the host. `accepted` requires an exact occurrence belonging to the named view. It preserves the native decision/source influence, even for an empty source. A newer acceptance does not retarget a pinned occurrence. Current authorization, policy expiry and proof checks still apply when reading historical occurrences or saved derived results.

`view_current` requires the immutable definition digest and either `fixed` or `tick TIME`. The engine uses RequireCurrent and refuses stale/partial materializations; it never refreshes as a side effect of reading. `tick value Instant` accepts an already specialized Time value; an Integer value is not implicitly Time. Fact time never changes the engine's trusted authorization time. Each statement binds once at its ordered position per Program execution; subsequent pure functions consume the bound graph rather than rereading the service.

A service result's actual schema is dynamic. Unconstrained graph functions may consume it; exact schema-constrained functions reject an unknown schema until a separate checked dynamic schema facility exists. The source cannot certify a schema or current access from a view name. Missing/inaccessible service errors retain native privacy rules. Source reads cannot install policy, approve a proposal, install a view, enroll a cache or subscribe to events.

## Compile a registration artifact

```weave
live_handle FleetHead graph "Fleet" branch "main";
view_template Active revision "1" from FleetHead clock tick {
  match relation "connected";
  at 0;
}
```

`weave view-plan example.weave --template Active` emits a `weave-view-registration/1` artifact. Optional `--modules MAP.json` uses the existing explicit pinned local loader. This command selects one artifact for review and performs no registration or other Program operation.

The first template profile is a standalone query from one declared live handle with optional relation and valid-time filters. It rejects already materialized graph captures, metadata expansion, nested service reads and arbitrary operators. `clock fixed` retains the authored selectors. `clock tick` uses the native engine's explicit tick replacement semantics; this is not a rolling-window operator. Optional selector `value` references use exact String/Time types.

The artifact contains the compiled expression, clock policy, template name/revision, canonical source manifest and definition digest. Its own source identity hashes the compiled recipe in a distinct template namespace. Comments and whitespace in the entry recipe, and module alias renaming, do not change its normalized identity; source revisions, predicate, time, clock and external selectors do. Imported modules use exact content pins: changing their bytes, even comments or whitespace, changes the required module manifest and therefore the aggregate definition identity. All linked module dependencies remain in the manifest. Graph, branch, view and decision strings are literal external IDs; the linker never rewrites them as source symbols.

A trusted host explicitly calls `register_compiled_view(instance_id, template, tick, host)`. The runtime checks the artifact, persists its source binding atomically, and retains those manifests on initial computation, full fallback and incremental refresh. Legacy source-less registration does not satisfy a source-aware definition digest. Instance identity and principal come from the host call, not a serialized grant. Enrollment, scheduling, ticks, refresh and transition consumption remain explicit native operations.

## Complete compiler output and compatibility

`compile_artifacts(source)` and `LinkedProgram::compile_artifacts()` return `CompiledArtifacts { program, values, view_templates }`. The separate complete fingerprint uses profile `weave-compiled-artifacts-v1` and covers all three outputs. `weave artifacts FILE` emits these artifacts plus that fingerprint. Ordinary programs keep their existing Program semantics and source identities.

Legacy `compile`, `specialize`, `fingerprint`, `describe` and their executable CLI/linked equivalents reject templates with `E_HOST_ARTIFACT_REQUIRED`; they cannot silently discard an unreturned registration artifact. `weave check` validates complete output and reports the artifact count/host registration requirement. Its success certifies source validation, not installation. `parse`/`ast` and infallible `LinkedProgram::schemas()` remain explicit inspection APIs; the latter only projects AST schema declarations and does not claim to compile or execute a source file.

Pure function bodies reject accepted/current reads and templates, including unused bodies. Imported modules remain pure and cannot contain these statements. An entry unit can use imported pure functions after a service read, and can include those exact module manifests in a template. Exporting templates from modules and capturing arbitrary pure-function recipes are future extensions.

Limits: at most 16 templates, 1 MiB serialized artifact each, 4 MiB aggregate template budget, existing source/AST/expansion limits, and conservative precharges before retained source-manifest copies. Complete artifact fingerprinting uses the public 16 MiB identity limit. Native read/materialization/registration limits remain authoritative; source cannot raise them.

This implements bounded source access for L15/L24 and source-aware host registration for L26. Reactor/effect syntax, generalized rolling windows, accepted-head-following templates, stale typed receipts, arbitrary view dependencies and full incremental evaluation remain open. Actual compiler-to-native acceptance is in `scripts/check_view_services.py`; it uses explicit prebuilt compiler/host fixture paths and never starts a build.
