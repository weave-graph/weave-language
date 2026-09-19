# Weave language contributor instructions

Read `docs/IMPLEMENTATION_PLAN.md`, `docs/WORKFLOW.md` and any recovered original papers before implementation. Follow the full requirement matrix; an initial subset is not completion. Keep exact source provenance separate from new design decisions.

This repository owns language semantics and tooling. The engine repository owns runtime/storage/network/authorization. Shared protocol changes require coordinated fixtures and versioning. Never supply authoritative system time from language clients, silently weaken coverage or permission labels, or substitute similarity for identity.

Keep evaluation pure and bounded. Use graph references for graph-valued metadata, including cycles and unavailable references. Preserve explicit contradictions, provenance and revision pins. Test observable semantics rather than private implementation shape.

Record actual checks and limits in `docs/STATUS.md`; do not claim unrun target builds or unimplemented plan rows. Do not commit credentials, private conversation material unrelated to the design, build outputs or personal machine paths. Public release and organization management are handled by the orchestrator.
