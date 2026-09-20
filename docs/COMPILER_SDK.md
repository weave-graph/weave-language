# Native and WebAssembly compiler SDK

The SDK compiles arbitrary bounded source into complete artifacts without I/O.
It uses the same language compiler, exact module linker and protocol 0.18 as the
CLI. It never executes a Program, installs a view/handler, accesses a store, fetches
a module, reads a clock or supplies runtime authority. Browser persistence and
mobile runtime integration remain separate requirements.

## Rust byte API

```rust
let response_bytes = weave_language::sdk::compile_request(request_bytes);
// Or compile_response(request_bytes) for { ok, bytes } without decoding output.
```

All request fields are required; duplicate/unknown fields reject:

```json
{"format":"weave-compiler-request/1","entry_id":"editor","source":"value Count 9007199254740993;","modules":[]}
```

Each optional supplied module entry has exactly `id`, `revision`, `source` string
fields. The `modules` array itself is required. Use the existing source `import`
syntax with an exact SHA-256 content pin. Source strings retain decoded UTF-8 bytes,
including comments/line endings; JSON escape spelling does not change the decoded
source. Missing, changed or cyclic dependencies fail with the original linker
code, source ID, byte span and import/application trace. No file paths or resolver
callbacks are accepted. Unused units do not add manifests or alter plain-source
identity. The diagnostic entry ID is descriptive and grants no authority.

Successful responses contain `format: "weave-compiler-response/1"`, `ok: true`,
`artifact_fingerprint` and the complete `artifacts` object: Program, scalar values,
view templates and handler templates when present. This fingerprint is the CLI
`artifacts` identity, not a plan-only or legacy linked fingerprint. Failure has
`ok: false` and `error` with code, message, original source ID/spans and traces.
SDK transport errors use `E_SDK_UTF8`, `E_SDK_REQUEST`, `E_SDK_BUDGET` or the fixed
`E_SDK_INTERNAL` diagnostic. Empty input is malformed JSON, not an empty Program.

## Native C and WASM

Build with the existing locked workspace dependencies:

```sh
cargo build --locked -p weave-compiler-sdk
cargo build --locked -p weave-compiler-sdk --target wasm32-unknown-unknown
```

The first produces a native cdylib (`.dylib`, `.so` or `.dll`); the second produces
`target/wasm32-unknown-unknown/debug/weave_compiler_sdk.wasm`. The WASM module has no
host imports. The [C header](../sdk/weave-compiler.h) describes the identical ABI 1
exports on both targets. All inputs/outputs are 32-bit integers and owned handles;
no caller pointer or WASM memory offset is accepted or returned.

Call `input_new(byte_length)`, then append each one/two-byte little-endian word with
`input_write(handle, word, count)`. `compile(handle)` consumes a fully written
input and returns a new output handle. `output_kind` returns 0 for artifacts or 1
for diagnostics. `output_len` is the exact JSON byte length; `output_read` returns
up to two bytes at each offset. Read the final odd byte according to that length.
`drop` releases an input/output exactly once. Handles are positive and never reused
within the library/WASM instance; exhaustion fails explicitly.

Negative ABI statuses: −1 invalid/stale/wrong-kind handle, −2 invalid range/word/count,
−3 buffer quota/allocation, −4 incomplete input, −5 concurrent compilation, −6 internal
failure, −7 handle exhaustion. Failed preflight leaves input owned and writable.
Accepted compilation consumes it even if compilation produces a diagnostic; the
returned diagnostic handle is still owned. Inputs and outputs not consumed must be
dropped. Dropping a consumed or already dropped handle fails. A native library has
one synchronized process-local arena; each WASM instance has its own. Calls on
other handles may proceed while one compilation runs; another compile gets BUSY.

## JavaScript without numeric corruption

```js
import { createCompiler } from './weave-compiler.mjs';
const module = await WebAssembly.compile(wasmBytes);
const instance = await WebAssembly.instantiate(module, {});
const compiler = createCompiler(instance.exports);
const result = compiler.compileSource({
  entryId: 'editor', source: 'value Count 9007199254740993;', modules: []
});
// result.ok, result.bytes (Uint8Array), result.text (exact JSON string).
// Forward original bytes; do not JSON.parse/stringify artifact data.
```

The dependency-free [adapter](../sdk/weave-compiler.mjs) prechecks string lengths,
then exact UTF-8 per-unit/aggregate limits before serializing convenience requests.
Heavily escaped valid source may still exceed the 8 MiB transport limit. It also accepts exact request
bytes via `compileBytes`. It handles release in `finally` and uses `output_kind`
without parsing numeric JSON. Source construction serializes only strings and
module lists; i64/Decimal/Quantity values never pass through JavaScript Number.
For artifact inspection use a lossless JSON decoder. Native forwarding can use an
exact JSON parser, or preserve response bytes verbatim. This profile copies two
bytes per ABI call; it is bounded and pointer-free, with no zero-copy promise.

## Limits and failure model

- Raw UTF-8 request: 8 MiB. Serde's temporary escape-decoding allocation is bounded
  by this envelope. Strict fields and a sequence visitor reject a 65th module
  without visiting its value. Decoded limits are checked before parsing/linking:
  1 MiB per source, 4 MiB aggregate, 64 supplied units, 512-byte entry ID and existing
  128-byte module labels. Existing linker depth/AST and specialization work budgets
  remain active. Heavily escaped transport can hit the raw limit first.
- Serialized response: 16 MiB + 4096 bytes. A capped amortized writer refuses excess
  output and returns a fixed small budget diagnostic; it never truncates artifacts.
- ABI arena: 16 live/reserved handles, 64 MiB retained/reserved buffer capacity,
  one in-flight compile. Account actual Vec capacity, including a consumed input
  until finish. Reserve the maximum output capacity before accepting compilation.
  Allocation/range arithmetic is checked; reservations are released on completion.
- These are retained-buffer/work bounds, not an OS memory or total transient-heap
  ceiling. The compiler may hold source/AST/artifacts while serializing. Rust's
  allocator abort, process death and WASM panic traps cannot be recovered as JSON.
  Native unwinding failures in compilation produce a fixed diagnostic and release
  reservations. Ordinary bounded malformed requests and ABI misuse return errors.
  After a trap or poisoned arena, discard the instance/process; do not reuse it.

A caller needing a hard execution deadline runs compilation in a worker/process
and terminates that worker; this SDK does not add cooperative cancellation. No
browser, Android or iOS runtime-persistence claim follows from compiler portability.

## Conformance and packaging

`scripts/check_compiler_sdk.py` supplies requests at execution time to safe Rust,
ctypes/native and optional Node/WASM paths and compares exact response bytes. It
also checks actual CLI artifact/fingerprint equivalence, pinned diamond imports,
module errors, i64 limits, exact numeric values, Unicode, view/handler completeness,
failure cleanup, and near-budget transfer overhead. This differs from the existing
fixed specialization fixture. Native/WASM builds and this harness run in CI.

Use the Git source archive, including `crates/weave-compiler-sdk`, `sdk/`, the
lockfile and exact vendors. No sibling engine, code generator, wasm-bindgen,
JavaScript package install or custom toolchain is required. Registry publication
remains disabled. Local measurements and executed checks are recorded in
[STATUS](STATUS.md); configured hosted checks are not reported as already run.
