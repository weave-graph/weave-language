# Contributing

Weave Language is an experimental implementation of a broader proposed design. Start with [verified status](docs/STATUS.md), [acceptance gaps](docs/ACCEPTANCE_GAPS.md), [syntax](docs/SYNTAX.md), and the [implementation workflow](docs/WORKFLOW.md). Original white-paper reconciliation is recorded in docs/SOURCE_RECONCILIATION.md; illustrative grammar and exploratory implementation choices are not ratified specifications.

Before a change, identify the requirement IDs, observable acceptance behavior and affected protocol version. Keep semantic changes separate from maintenance. Compiler changes that affect the runtime contract need coordinated engine review and fixtures in both projects. Do not hand-edit vendored protocol code: import the complete authoritative engine crate, preserve its MIT license, update `vendor/manifest.json`, and run its verifier.

## Build and checks

Use Rust 1.94.0 or a newer stable toolchain and Python 3. The current release was tested on Rust 1.94.0; an earlier minimum supported compiler has not been established. Build from the repository root:

```sh
python3 scripts/verify_contract.py
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo install --locked --path . --root ./install-check
```

For portable-library checking, install the `wasm32-unknown-unknown` Rust target and run `cargo check --locked --lib --target wasm32-unknown-unknown`. A target build alone does not verify browser or mobile integration.

The separately built engine can run the cross-project fixture:

```sh
python3 scripts/check_composed.py --engine /path/to/weave-engine
```

Add tests for changed observable behavior, including relevant failure cases. Document commands actually run and their outcomes. Preserve explicit coverage, authority, uncertainty, provenance and valid/system-time distinctions. A compiling stub or skipped unsupported case does not close a requirement.

## Pull requests and license

Describe the problem, resulting behavior, validation and remaining limits. Public examples and fixtures must be synthetic. Do not include credentials, private conversation exports or personal machine paths. Contributions are provided under the repository's MIT license; retain third-party notices. New dependencies need license and platform review, a lockfile update and a reason to enter the portable core.

See [SECURITY.md](SECURITY.md) for vulnerability reporting guidance. Optional editor tools and registry packages must not delay required language/runtime semantics or be misreported as original user requirements.
