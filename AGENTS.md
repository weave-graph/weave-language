# Working in weave-language

Instructions for AI agents and human contributors. `CLAUDE.md` imports this file.

## Read first

1. The [Weave vision](https://github.com/weave-graph/weave-engine/blob/main/VISION.md)
   and [engine roadmap](https://github.com/weave-graph/weave-engine/blob/main/docs/ROADMAP.md).
   The engine is the product; this language is an optional front end, and its
   priorities follow the engine's phases.
2. [docs/ROADMAP_PROPOSAL.md](docs/ROADMAP_PROPOSAL.md) for the proposed
   language epics, and [docs/STATUS.md](docs/STATUS.md) for what works today.
3. The GitHub issue for the epic you are working on. Its acceptance criteria
   define the work.

`docs/IMPLEMENTATION_PLAN.md`, `docs/WORKFLOW.md` and
`docs/SOURCE_RECONCILIATION.md` record the original white-paper plan. They are
background, not the work queue.

## Architecture overview

One Rust package (`weave-language`, binary `weave`) plus one workspace member.

| Path | Role |
|---|---|
| `src/syntax.rs` | Hand-written bounded lexer and parser, AST, spans |
| `src/lib.rs` | Validation and lowering to engine Programs; diagnostics |
| `src/main.rs` | CLI: `check`, `plan`, `ast`, `describe`, `fingerprint`, `values`, `artifacts`, `view-plan`, `handler-plan`, `effects`, `fmt`, `sdk` (one SDK request on stdin); `--modules MAP.json`; policy flags `--untrusted`, `--read-only`, `--visible MANIFEST.json` |
| `src/policy.rs` | Compile policies for untrusted source, visible-manifest diagnostics and plan effects (LB1) |
| `src/functions.rs`, `scalars.rs`, `decimal.rs`, `interval.rs`, `vectors.rs`, `vector_types.rs`, `bytes.rs`, `references.rs`, `context_axes.rs`, `graph_types.rs` | Pure compile-time semantics and value types |
| `src/modules.rs`, `module_files.rs`, `modules/rewrite.rs` | Content-pinned source modules |
| `src/format.rs` | Token-preserving formatter |
| `src/sdk.rs`, `crates/weave-compiler-sdk`, `sdk/` | Byte-oriented compiler SDK: Rust, C ABI header, zero-import WASM with a JS adapter |
| `vendor/weave-contract`, `vendor/weave-spaces` | Exact copies of the engine crates; hashes are in `vendor/manifest.json` and `vendor/spaces-manifest.json` |
| `examples/` | `.weave` programs, also used by tests and scripts |
| `scripts/check_*.py` | End-to-end compiler-to-engine acceptance (most need `--engine PATH`); `verify_*.py` check the vendored files |

The compiler is pure. It never runs plans, touches storage or grants authority.
The engine validates and authorizes every emitted plan.

## Build and test

Requires stable Rust (tested with 1.94+), Python 3 and a C toolchain.

```sh
python3 scripts/verify_contract.py      # vendored contract matches manifest
python3 scripts/verify_spaces.py
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --workspace --locked         # about 249 tests
cargo check --locked --lib --target wasm32-unknown-unknown   # portable core
```

For cross-project checks, build the sibling engine and pass its binary, for
example:

```sh
python3 scripts/check_composed.py --engine ../weave-engine/target/debug/weave-engine
```

**CI** (`.github/workflows/ci.yml`):

- The `native` job runs the contract checks, fmt, Clippy, tests and the SDK and
  interval oracles on Linux, macOS and Windows.
- The `wasm` job checks the wasm32 build and native/WASM parity.
- `docs-check.yml` enforces the docs rule below.

If linking fails with `cc: unknown command`, a non-compiler `cc` is first on
`PATH`. Use `CC=gcc CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=gcc`.

## Conventions

- **Ownership.**
  - This repo owns grammar, types, diagnostics, pure evaluation, lowering and
    tooling.
  - The engine owns storage, recorded time, permissions, events, embeddings
    search and every service.
  - Never supply authoritative recorded time from the compiler.
  - Never silently weaken coverage or permission labels, or substitute
    similarity for identity.
- **Contract.**
  - Never hand-edit `vendor/`. Re-import the complete engine crate, update
    the manifest and run `verify_contract.py`.
  - Contract changes are coordinated with the engine and its golden fixtures.
- **Evaluation** stays pure and bounded (fuel, size and depth limits with
  explicit diagnostics). Diagnostics are JSON with a stable `code` and exact
  UTF-8 byte spans.
- **Tests** cover observable behavior, including failure cases. Keep examples
  synthetic.
- **Do not commit** credentials, personal paths, build outputs or private
  conversation material.

## Documentation rule: docs ship with code

**Every PR that changes behavior updates the affected docs in the same PR**:

- the feature reference and `docs/SYNTAX.md` for syntax;
- `docs/STATUS.md` when supported behavior or limits change;
- `docs/README.md` when a doc is added;
- the README when the quickstart or purpose changes.

A PR without doc changes must say `[no-docs]` with a reason. CI enforces this.

Write reference docs for the current state, and mark the version that
introduced a feature. Put dated verification evidence in `docs/STATUS_HISTORY.md`
or a `VERIFICATION_*.md` file, not in reference docs or issue bodies.

## Definition of done

A change is done when:

1. Its claimed acceptance criteria have linked tests.
2. The full CI set above passes locally for what you touched, and contract
   verification always passes.
3. Docs are updated per the rule above.
4. Limits are documented.
5. Contract changes are coordinated with the engine.

An epic closes only when every criterion in its issue is met; subsets are
recorded as issue comments.

## Process

- Branch from `main`. Make focused PRs that reference the epic issue. Do not
  merge without owner approval.
- Do not close, retitle or create GitHub issues unless the owner asked for it.
- Report the commands you ran and their actual results. Skipped or unrun checks
  are not passes.
