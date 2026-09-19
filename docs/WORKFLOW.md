# Implementation workflow

This workflow covers the full [implementation plan](IMPLEMENTATION_PLAN.md). Its executable data representation is [workflow.json](workflow.json). A completed early gate never implies full product completion.

```mermaid
flowchart TD
  L0["L0 Recover papers · ratify semantics/protocol"] --> L1["L1 Grammar · types · diagnostics"]
  L1 --> L2["L2 Pure lenses · temporal joins · coverage"]
  L2 --> L3["L3 Metadata graphs · manifestations · provenance · rules"]
  L3 --> L4["L4 Secure reactive engine integration"]
  L3 --> L5["L5 Geometry · embeddings · semantic zoom"]
  L4 --> L6["L6 Offline peers · synchronization · governance"]
  L5 --> L6
  L6 --> L7["L7 Tooling · conformance audit · public release"]
  E1["Engine: snapshots and model"] -.-> L2
  E2["Engine: identity and references"] -.-> L3
  E3["Engine: capabilities and atomic event bus"] -.-> L4
  E4["Engine: geometry and clustering"] -.-> L5
  E5["Engine: sync, governance and device host"] -.-> L6
  L7 --> Done["Owner scope accepted; all required tests pass"]
```

## Work item lifecycle

```mermaid
stateDiagram-v2
  [*] --> Proposed
  Proposed --> Ready: requirements and dependencies reviewed
  Ready --> InProgress: one assigned owner
  InProgress --> Review: implementation and acceptance evidence
  InProgress --> Blocked: named external dependency
  Blocked --> Ready: dependency resolved
  Review --> InProgress: changes required
  Review --> Verified: independent checks pass
  Verified --> Released: public artifact and CI verified
  Released --> [*]
```

Each issue names requirement IDs, input/output artifacts, dependencies, one owner and an observable acceptance test. Changes to protocol semantics require review from both project owners and conformance fixtures in both repositories. Work proceeds in isolated branches; implementation, documentation and meaningful tests land together. Never mark `Verified` on a skipped, unsupported or placeholder path.

For each gate, maintain a status entry with commit SHA, exact commands, outcomes, CI/release URLs, known limitations and remaining requirements. Report code implemented, locally verified, integration verified and publicly released separately. Research uncertainty becomes a decision record or blocked item; it does not disappear from the dependency graph.

Security and performance reviews occur while implementing the relevant semantics and again at release. Public issues and documentation use synthetic examples and exclude personal source conversation content that is unrelated to the design.
