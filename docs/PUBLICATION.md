# Public project and acceptance tracking

The experimental source repository is public under the MIT license: [weave-graph/weave-language](https://github.com/weave-graph/weave-language). It is not a claim of full white-paper conformance.

- [Implementation board](https://github.com/orgs/weave-graph/projects/1)
- [Dependency graph](WORKFLOW.md)
- [Implementation plan](IMPLEMENTATION_PLAN.md)
- [Current implementation evidence and limits](STATUS.md)
- [Hosted CI](https://github.com/weave-graph/weave-language/actions)

Original v0.1 papers were supplied by the project owner on 2026-09-19. Preserved source artifacts and hashes are in `docs/source/`; the original papers describe an architecture proposal and distinguish requirements, recommendations, and open research. Private conversation exports are excluded from Git.

Each gate has a public issue. Partial implementation keeps its gate open. Closing a gate requires evidence covering its acceptance criteria, including exact commits, checks, supported hosts, and limitations.

| Gate | Public acceptance issue |
| --- | --- |
| L0 | [L0](https://github.com/weave-graph/weave-language/issues/1) |
| L1 | [L1](https://github.com/weave-graph/weave-language/issues/2) |
| L2 | [L2](https://github.com/weave-graph/weave-language/issues/3) |
| L3 | [L3](https://github.com/weave-graph/weave-language/issues/4) |
| L4 | [L4](https://github.com/weave-graph/weave-language/issues/5) |
| L5 | [L5](https://github.com/weave-graph/weave-language/issues/6) |
| L6 | [L6](https://github.com/weave-graph/weave-language/issues/7) |
| L7 | [L7](https://github.com/weave-graph/weave-language/issues/8) |

## Independent integration evidence

The orchestrator cloned both public repositories without credentials and ran three separate compiler-to-runtime acceptance suites on language `bbaa5f3` / engine `938cde7` (protocol 0.3). All passed: persistence and revision pins across processes, temporal joins and permission filtering, reusable graph composition and atomic rollback. The engine repository contains these executable suites as `scripts/root_integration.py`, `scripts/root_join.py`, and `scripts/root_values.py`; its Language integration workflow runs against an explicitly pinned compiler commit.

This evidence covers those behaviors only. It does not establish federation, full capability security, clustering, portable mobile execution, or complete language semantics.
