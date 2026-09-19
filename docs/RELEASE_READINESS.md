# Experimental source release readiness

This checklist concerns distribution of the current experimental source, not full-design acceptance. The [acceptance gaps](ACCEPTANCE_GAPS.md) remain authoritative for incomplete capabilities.

## Verified locally

- Repository includes MIT license, contribution and security guidance, source, tests, examples, lockfile and a standalone vendored contract.
- Contract verifier checks the exact vendored file set, license, Cargo version, protocol constant and current documentation hash.
- Source grammar and public API are documented as provisional; current protocol is v0.3.0. Historical contract notes are labeled historical.
- `check`, `ast` and `plan` are the implemented CLI subcommands. No formatter, language runner, `describe`, LSP or mobile SDK is advertised as implemented.
- `publish = false` prevents accidental registry publication before the separate contract package has an approved distribution path. Git source distribution remains available.

## Distribution procedure

Use a clean commit, with no required untracked files:

```sh
git archive --format=tar --output=weave-language-source.tar HEAD
```

Extract into a new directory, verify the contract and build/test/install there. `cargo install --locked --path . --root ./install-check` installs the CLI without publishing anything. Dependencies are locked but still require access to crates.io unless already cached; an offline-capable runtime is not the same as a dependency-free offline build.

The source archive includes `vendor/weave-contract/{Cargo.toml,LICENSE,src/lib.rs}`. Cargo's registry package excludes nested package files and rewrites path dependencies; it is deliberately not the release artifact at this stage.

## CI configuration

Native jobs check contract integrity, formatting, strict Clippy, all-target tests and installation on Linux/macOS/Windows. A separate job checks the library's WASM target. Workflow permissions are limited to reading contents. These are configured jobs, not proof they have run. Rust and GitHub Action references currently follow their selected stable/major channels; action/toolchain pinning and dependency vulnerability review remain release-hardening work, not claims of bit-for-bit reproducibility.

## Orchestrator verification before public release

- Confirm intended public organization and repositories exist and verify public visibility.
- Verify no private source export, credentials, private paths or generated local databases enter the tree/history.
- Link the exact source commit and public CI results. A local passing run does not substitute for remote matrix outcomes.
- Confirm contribution and security contact/reporting statements reflect actual repository settings.
- Label the release experimental and link the complete acceptance gaps; do not claim original-paper conformance before source recovery.

Registry packages, binary signing, packaged WASM/browser execution and mobile distributions are later optional delivery artifacts or platform requirements with their own evidence. Their absence does not prevent a truthful public GitHub source release.
