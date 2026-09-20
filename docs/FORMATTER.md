# Source formatter

`format_source(&str) -> Result<String, Diagnostic>` formats one syntax-valid source unit without linking imports, compiling a plan, reading graph state or registering host artifacts. It preserves the exact spelling and order of every code token and `//` comment, including string escapes, numeric spelling, Unicode and comment content. `//` is the only comment form currently supported by the language lexer.

```sh
weave fmt examples/fleet.weave
weave fmt examples/fleet.weave --check
weave fmt examples/fleet.weave --write
```

Default output goes to stdout and leaves the file untouched. `--check` emits `E_FORMAT` and exits 1 if formatting would change the file; already formatted input exits 0. `--write` explicitly replaces one regular file through a temporary file in the same directory, retaining its permission bits. Final-component symlinks are rejected, including when their target already has canonical formatting. No imported files or pins are automatically changed. Syntax/resource failures produce original JSON diagnostics and no partial formatted source or replacement. A detected intervening edit aborts the write; this is not a filesystem sandbox or a guarantee against arbitrary concurrent writers.

The bounded style uses two-space block indentation, statement newlines, punctuation spacing and a final newline. Existing token spelling is never decoded and regenerated. Arrays and function arguments remain compact; JSON object literals use the same brace layout. Comments keep their text and whether they followed a code token on its line. Blank lines outside tokens/comments normalize away. A CR in a CRLF comment is retained as part of that original comment spelling; other generated line separators are LF. There is no configurable line-width reflow or AST pretty-printer.

Input and output are each limited to 1 MiB; at most 100,000 combined code tokens and comments are retained. Existing parser depth and syntax limits still apply. A valid source whose expanded indentation exceeds the output budget is rejected instead of emitting a truncated file. Semantic errors or missing imported dependencies do not prevent syntax-only formatting; use `weave check` for compilation/link validation.

## Exact module pins change

Formatting preserves parsed meaning, not original file bytes. Content-pinned modules deliberately identify **all bytes**, including comments and whitespace. Formatting such a module changes its digest; an importer with the old pin must fail until its pin is explicitly updated and the complete module set is relinked. Transitive import edits may change more module pins. `--write` does not cascade those edits, increment module revisions, or approve publication.

For an entry unit without changed module bytes, plan, typed value and complete host-artifact semantics/identities remain stable under formatting. After formatting imported units and explicitly updating pins, semantic graph commands can remain equivalent while source manifests and aggregate artifact fingerprints must change. Those identities are never silently erased to manufacture an equality claim.

## Scope

This implements the formatter portion of the approved L29 tooling plan. Runtime schema discovery, richer repair/tooling workflows and broader language semantics remain separate requirements. Source formatting is not host execution or installation of a view/reactor.

Independent orchestrator verification of `3ea25e3` passed the actual CLI against Unicode and CRLF comments, escaped URL/string literals, scalar result and fingerprint equivalence, repeated formatting, nonmutating stdout/check, explicit writes preserving permission bits, symlink rejection, and unchanged files after malformed/oversized input. Temporary files were removed. The implementation owner also passed all 142 tests, strict lint, WASM library compilation, and exact vendor/source checks. Hosted publication is verified separately.
