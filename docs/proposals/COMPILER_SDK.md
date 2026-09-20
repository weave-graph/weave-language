# Bounded native and WebAssembly compiler SDK

Accepted design, implemented by the [compiler SDK](../COMPILER_SDK.md).
Source-only; protocol remains 0.18. No new runtime authority or persistent browser
engine is included in this profile.

## One complete, pure operation

Reuse `modules::link(...).compile_artifacts()` and the existing complete-artifact
fingerprint. A safe Rust `sdk::compile_request(&[u8]) -> Vec<u8>` accepts UTF-8 JSON:

```json
{"format":"weave-compiler-request/1","entry_id":"entry","source":"...","modules":[{"id":"core","revision":"1","source":"..."}]}
```

All fields are required; unknown/duplicate fields reject. Module units contain exact
source bytes; import statements supply the already-supported content pins. No file
paths, callbacks, resolver imports, network access, clock, principal or HostContext
are accepted. Missing modules fail explicitly. The entry ID is diagnostic context,
not source authority. Always return complete artifacts, including scalar values,
view templates and handler templates; no legacy artifact-dropping path is exposed.

Success JSON: `{format:"weave-compiler-response/1",ok:true,artifact_fingerprint,
artifacts:{program,values,view_templates,handler_templates?}}`.
Failure JSON: `{format:"weave-compiler-response/1",ok:false,error:{code,message,
source_id,start,end,import_trace,application_trace?}}`. Ordinary source diagnostics
retain original UTF-8 spans. Transport failures use bounded `E_SDK_*` diagnostics.
No timestamps, request IDs or platform-specific text enter successful identities.

## Fixed, pointer-free ABI

Add a small `weave-compiler-sdk` rlib/cdylib crate using the same Rust byte API.
The native C ABI and `wasm32-unknown-unknown` exports use only 32-bit integers and
owned opaque handles. No export accepts or returns a host pointer or memory offset.
This deliberately avoids unchecked pointer validity/aliasing assumptions and keeps
one ABI across native pointer widths and WASM. Supply a C header and a small JS
adapter; no npm dependency or install is needed.

| Export | Contract |
| --- | --- |
| `weave_compiler_abi_version() -> i32` | Returns 1 |
| `weave_compiler_input_new(length: u32) -> i32` | Positive input handle or negative ABI status |
| `weave_compiler_input_write(handle: u32, word: u32, count: u32) -> i32` | Append 1 or 2 low-order little-endian bytes; word must fit 16 bits; 0 success |
| `weave_compiler_compile(input: u32) -> i32` | Consume a fully written input; positive output handle, including source-error output |
| `weave_compiler_output_len(handle: u32) -> i32` | Exact byte length or negative ABI status |
| `weave_compiler_output_kind(handle: u32) -> i32` | 0 compiled artifacts, 1 diagnostic, negative ABI status |
| `weave_compiler_output_read(handle: u32, offset: u32) -> i32` | Read up to two little-endian bytes, 0..65535; final odd byte has zero high byte; negative status on invalid range |
| `weave_compiler_drop(handle: u32) -> i32` | Release input/output exactly once; repeated/stale handles reject |

Handles are monotonically allocated 1..i32::MAX and never reused within an instance;
exhaustion refuses allocation. Unknown handles, wrong handle kinds, unwritten input,
overflow/range errors and double release never dereference caller memory or panic.
ABI status constants are fixed negative values: INVALID_HANDLE=-1, BOUNDS=-2,
BUDGET=-3, INCOMPLETE=-4, BUSY=-5, INTERNAL=-6, HANDLE_EXHAUSTED=-7.
Compilation of malformed UTF-8/JSON/source returns an owned diagnostic output, not
an ABI error. Input handles remain owned on preflight ABI failure. Once compilation
is accepted, input ownership is consumed on both success and diagnostic failure.

The JS adapter packs/unpacks two bytes per call, returning `{ok,bytes,text}` from
output-kind and UTF-8 decoding. It never JSON-parses/reserializes artifact data.
Thus i64 values beyond 2^53, exact Decimal strings and source/module UTF-8 bytes
survive intact. Caller inspection must use a lossless decoder; runtime forwarding
uses the original bytes or a native exact JSON parser. JS request construction only
serializes source/unit strings and the fixed format, not authored numeric values.
The bounded two-byte copying cost will be measured on representative fixtures;
there is no claimed zero-copy path or unsafe pointer fast path in this profile.

## Bounds, ownership and failure cleanup

- Raw request at most 8 MiB, checked before decoding; serde scratch allocation is
  bounded by that envelope, and exact decoded limits apply before compilation. Existing source limits remain
  1 MiB/unit, 4 MiB total, 64 supplied units, depth 16 and existing AST/work limits.
  Explicit bounded unit-sequence decoding rejects an excess unit before retaining
  it; entry ID is bounded to 512 bytes. Heavily escaped transport may hit 8 MiB first.
- Response at most 16 MiB + 4096 bytes. Serialize through a bounded writer before
  retaining excess bytes; serialization overflow returns a small budget diagnostic.
- At most 16 live handles, 64 MiB total retained/reserved buffer bytes per ABI instance,
  and one in-flight compilation. Reserve worst-case output capacity before accepting
  a compile. Count the still-live request until compilation releases it; release all
  reservations on every completion/error. Failed preflight leaves the input usable.
- A per-instance synchronized arena makes individual calls safe; no borrowed buffer
  pointer survives a call. Compilation runs outside the arena lock. Concurrent compile
  attempts return BUSY; dropping a consumed input returns INVALID_HANDLE. These are
  allocation limits, not a claim about total process/compiler transient peak memory.
- Use checked arithmetic and fallible reservation; charge actual buffer capacity. Native unwind panics are caught
  outside the lock and converted to INTERNAL/owned fixed diagnostics after cleanup.
  WASM panic-abort, allocator abort and external process death cannot be promised as
  recoverable JSON errors: the host discards/recreates that instance. Tests require
  malformed bounded input and ABI misuse to take ordinary errors, never that path.

Rust callers need no arena: their input is borrowed for the synchronous call and
returned response bytes are ordinarily owned. Native C/WASM callers own every
allocated handle until consumed/dropped; the adapter releases outputs in `finally`.
No ambient global last-error buffer, leaked pointer, hidden host effects or caller
constructed authority object is introduced.

## Files and acceptance

Language owns `src/sdk.rs`, `crates/weave-compiler-sdk/{Cargo.toml,src/lib.rs}`, header,
JS adapter, tests/docs and CI. Add the SDK as a workspace member; reuse existing
Cargo target/cache and vendored crates. Engine changes are unnecessary. Existing
CLI/library APIs and output identities stay compatible.

Required checks before freeze:

1. Arbitrary requests supplied at execution time: native safe API, native cdylib via
   ctypes, and actual Node WebAssembly execution have identical response bytes.
   The WASM module must have zero imports; it is not the fixed parity fixture.
2. Exact pinned/diamond modules, missing/hash/cycle failures, original module spans,
   pure functions/scalars/vectors and complete view/handler artifacts. Compare
   successful artifacts and fingerprint with the existing library/CLI oracle.
3. i64 min/max and 9007199254740993, canonical Decimal/Quantity, Unicode/comments,
   negative zero and raw module pins; JS round-trip never uses Number for plan data.
4. Invalid UTF-8/JSON, duplicate/unknown fields, budget/length/offset arithmetic,
   excess nested module element, stale/wrong-kind/double-drop handles, incomplete
   writes, output quotas and reservation cleanup. Fresh valid request succeeds after
   each ordinary failure; isolate panic cleanup tests on native where unwind exists.
5. Clean source archive builds rlib/native cdylib/WASM using locked dependencies and
   documented commands; packaged header/adapter and exact vendor verifiers pass.
   Existing source suite, fixed parity and historical artifacts remain unchanged.

Bytes/object-reference semantics and source attach/fork/integrate/governance artifacts
remain separate mandatory gates; this SDK does not implement them or a browser store.
