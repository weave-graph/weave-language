// Byte-oriented adapter for ABI 1. It never parses artifact JSON or reads WASM memory.
export class CompilerAbiError extends Error {
  constructor(operation, status) {
    super(`Weave compiler ${operation} failed (${status})`);
    this.name = 'CompilerAbiError'; this.operation = operation; this.status = status;
  }
}
const MAX_REQUEST = 8 * 1024 * 1024;
const MAX_RESPONSE = 16 * 1024 * 1024 + 4096;
export function createCompiler(exports) {
  const api = {};
  for (const name of ['abi_version', 'input_new', 'input_write', 'compile', 'output_len', 'output_kind', 'output_read', 'drop']) {
    const fn = exports[`weave_compiler_${name}`];
    if (typeof fn !== 'function') throw new TypeError(`Missing compiler export ${name}`);
    api[name] = fn;
  }
  if (api.abi_version() !== 1) throw new Error('Unsupported compiler ABI');
  function checked(name, ...args) {
    const status = api[name](...args);
    if (!Number.isInteger(status) || status < 0) throw new CompilerAbiError(name, status);
    return status;
  }
  function compileBytes(input) {
    if (!(input instanceof Uint8Array)) throw new TypeError('Expected UTF-8 request bytes');
    if (input.length > MAX_REQUEST) throw new CompilerAbiError('input_new', -3);
    let handle = 0, output = 0;
    try {
      handle = checked('input_new', input.length);
      for (let offset = 0; offset < input.length; offset += 2) {
        checked('input_write', handle, input[offset] | ((input[offset + 1] ?? 0) << 8), Math.min(2, input.length - offset));
      }
      output = checked('compile', handle); handle = 0;
      const length = checked('output_len', output), kind = checked('output_kind', output);
      if (length > MAX_RESPONSE || kind > 1) throw new Error('Invalid compiler response');
      const bytes = new Uint8Array(length);
      for (let offset = 0; offset < length; offset += 2) {
        const word = checked('output_read', output, offset);
        if (word > 65535) throw new Error('Invalid compiler byte word');
        bytes[offset] = word & 255;
        if (offset + 1 < length) bytes[offset + 1] = word >>> 8;
      }
      return { ok: kind === 0, bytes, text: new TextDecoder('utf-8', { fatal: true }).decode(bytes) };
    } finally {
      // A trap requires discarding the instance; ordinary failures release all handles.
      if (output) api.drop(output);
      if (handle) api.drop(handle);
    }
  }
  function compileSource({ entryId, source, modules = [] }) {
    if (typeof entryId !== 'string' || typeof source !== 'string' || !Array.isArray(modules) || modules.length > 64) {
      throw new TypeError('Expected source strings and at most 64 exact module units');
    }
    const encoder = new TextEncoder();
    let total = 0;
    function boundedString(value, limit, sourceUnit = false) {
      // UTF-16 length is a cheap lower bound on encoded bytes: reject huge strings
      // before encoding or JSON escaping; then enforce exact UTF-8 byte limits.
      if (value.length > limit) throw new CompilerAbiError('request_budget', -3);
      const length = encoder.encode(value).length;
      if (length > limit) throw new CompilerAbiError('request_budget', -3);
      if (sourceUnit) {
        total += length;
        if (total > 4 * 1024 * 1024) throw new CompilerAbiError('request_budget', -3);
      }
    }
    boundedString(entryId, 512); boundedString(source, 1024 * 1024, true);
    const units = modules.map(({ id, revision, source }) => {
      if ([id, revision, source].some(value => typeof value !== 'string')) throw new TypeError('Module fields must be strings');
      boundedString(id, 128); boundedString(revision, 128); boundedString(source, 1024 * 1024, true);
      return { id, revision, source };
    });
    return compileBytes(new TextEncoder().encode(JSON.stringify({ format: 'weave-compiler-request/1', entry_id: entryId, source, modules: units })));
  }
  return Object.freeze({ compileBytes, compileSource });
}
