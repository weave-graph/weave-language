//! Bounded, I/O-free compiler requests. Source and output remain exact UTF-8 bytes.
use crate::{CompiledArtifacts, Diagnostic, modules};
use serde::{Deserialize, Serialize};
use std::io::Write;

pub const MAX_REQUEST_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024 + 4096;
pub const REQUEST_FORMAT: &str = "weave-compiler-request/1";
pub const RESPONSE_FORMAT: &str = "weave-compiler-response/1";

/// Complete serialized response; `ok` lets byte-oriented adapters avoid parsing numbers.
pub struct Response {
    pub ok: bool,
    pub bytes: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Unit {
    id: String,
    revision: String,
    source: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    format: String,
    #[serde(rename = "entry_id")]
    entry: String,
    source: String,
    #[serde(rename = "modules", deserialize_with = "bounded_units")]
    units: Vec<Unit>,
}

/// Exact JSON response bytes, including ordinary compiler diagnostics.
pub fn compile_request(input: &[u8]) -> Vec<u8> {
    compile_response(input).bytes
}

/// Equivalent byte API with a separate status bit for native/WASM adapters.
/// Native unwinding panics become a fixed diagnostic. Allocator aborts and WASM traps
/// are outside the recoverable contract; hosts must discard the failed instance.
pub fn compile_response(input: &[u8]) -> Response {
    std::panic::catch_unwind(|| compile_inner(input))
        .unwrap_or_else(|_| transport("E_SDK_INTERNAL", "Compiler failed internally"))
}
fn compile_inner(input: &[u8]) -> Response {
    let request = match decode_request(input) {
        Ok(r) => r,
        Err(e) => return transport(e.0, e.1),
    };
    let parsed = match crate::parse(&request.source) {
        Ok(parsed) => parsed,
        Err(e) => return failure(local_error(&request.entry, e)),
    };
    // Select by authored imports, not by whether the host supplied any units:
    // missing imports need linker traces; unused units must not normalize plain-source identity.
    let has_imports = parsed
        .statements
        .iter()
        .any(|s| matches!(s, crate::syntax::Statement::Import { .. }));
    let compiled = if !has_imports {
        crate::compile_artifacts_parsed(parsed).map_err(|e| local_error(&request.entry, e))
    } else {
        let units: Vec<_> = request
            .units
            .iter()
            .map(|u| modules::SourceModule {
                id: &u.id,
                revision: &u.revision,
                source: &u.source,
            })
            .collect();
        modules::link(&request.entry, &request.source, &units).and_then(|p| p.compile_artifacts())
    };
    match compiled {
        Err(error) => failure(error),
        Ok(artifacts) => match artifacts.fingerprint() {
            Err(e) => failure(local_error(&request.entry, e)),
            Ok(fingerprint) => {
                #[derive(Serialize)]
                struct Success<'a> {
                    format: &'static str,
                    ok: bool,
                    artifact_fingerprint: String,
                    artifacts: &'a CompiledArtifacts,
                }
                encode(
                    true,
                    &Success {
                        format: RESPONSE_FORMAT,
                        ok: true,
                        artifact_fingerprint: fingerprint,
                        artifacts: &artifacts,
                    },
                )
            }
        },
    }
}
fn local_error(source: &str, e: Diagnostic) -> modules::ModuleDiagnostic {
    modules::ModuleDiagnostic {
        source_id: source.into(),
        code: e.code,
        message: e.message,
        start: e.start,
        end: e.end,
        import_trace: vec![],
        application_trace: Box::new(
            e.trace
                .into_iter()
                .map(|(start, end)| modules::ApplicationLocation {
                    source_id: source.into(),
                    start,
                    end,
                })
                .collect(),
        ),
    }
}
fn transport(code: &str, message: &str) -> Response {
    failure(modules::ModuleDiagnostic {
        source_id: String::new(),
        code: code.into(),
        message: message.into(),
        start: 0,
        end: 0,
        import_trace: vec![],
        application_trace: Box::default(),
    })
}
fn failure(error: modules::ModuleDiagnostic) -> Response {
    #[derive(Serialize)]
    struct Failure {
        format: &'static str,
        ok: bool,
        error: modules::ModuleDiagnostic,
    }
    encode(
        false,
        &Failure {
            format: RESPONSE_FORMAT,
            ok: false,
            error,
        },
    )
}
struct Output(Vec<u8>);
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_RESPONSE_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("output budget"));
        }
        let needed = self.0.len() + bytes.len();
        if needed > self.0.capacity() {
            let target = needed
                .max(self.0.capacity().saturating_mul(2))
                .clamp(1024, MAX_RESPONSE_BYTES);
            self.0
                .try_reserve_exact(target - self.0.len())
                .map_err(std::io::Error::other)?;
            if self.0.capacity() > MAX_RESPONSE_BYTES {
                return Err(std::io::Error::other("output capacity budget"));
            }
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn encode(ok: bool, value: &impl Serialize) -> Response {
    let mut output = Output(Vec::new());
    if serde_json::to_writer(&mut output, value).is_err() {
        // Fixed bytes avoid recursive encoding or platform-specific allocator messages.
        return Response { ok: false, bytes: br#"{"format":"weave-compiler-response/1","ok":false,"error":{"source_id":"","code":"E_SDK_BUDGET","message":"Response exceeds allocation budget","start":0,"end":0,"import_trace":[]}}"#.to_vec() };
    }
    Response {
        ok,
        bytes: output.0,
    }
}

type DecodeError = (&'static str, &'static str);
const INVALID: DecodeError = (
    "E_SDK_REQUEST",
    "Expected strict weave-compiler-request/1 JSON",
);
const BUDGET: DecodeError = (
    "E_SDK_BUDGET",
    "Request source or transport budget exceeded",
);
fn decode_request(input: &[u8]) -> Result<Request, DecodeError> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(BUDGET);
    }
    std::str::from_utf8(input).map_err(|_| ("E_SDK_UTF8", "Request must be UTF-8"))?;
    // Decoded strings never exceed the bounded raw JSON envelope. Serde may allocate
    // scratch space for escapes; exact decoded source bounds apply before compilation.
    let r: Request = serde_json::from_slice(input).map_err(|e| {
        if e.to_string()
            .starts_with("E_SDK_BUDGET: at most 64 module units")
        {
            BUDGET
        } else {
            INVALID
        }
    })?;
    if r.format != REQUEST_FORMAT || r.entry.is_empty() {
        return Err(INVALID);
    }
    if r.entry.len() > 512 || r.source.len() > modules::MAX_UNIT_BYTES {
        return Err(BUDGET);
    }
    let mut total = r.source.len();
    for unit in &r.units {
        total = total.checked_add(unit.source.len()).ok_or(BUDGET)?;
        if unit.id.len() > 128
            || unit.revision.len() > 128
            || unit.source.len() > modules::MAX_UNIT_BYTES
            || total > modules::MAX_TOTAL_BYTES
        {
            return Err(BUDGET);
        }
    }
    Ok(r)
}
fn bounded_units<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Unit>, D::Error> {
    struct Units;
    struct Excess;
    impl<'de> serde::de::DeserializeSeed<'de> for Excess {
        type Value = ();
        fn deserialize<D: serde::Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
            Err(serde::de::Error::custom(
                "E_SDK_BUDGET: at most 64 module units",
            ))
        }
    }
    impl<'de> serde::de::Visitor<'de> for Units {
        type Value = Vec<Unit>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("at most 64 module units")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut units = Vec::new();
            loop {
                if units.len() == modules::MAX_UNITS {
                    seq.next_element_seed(Excess)?;
                    return Ok(units);
                }
                let Some(unit) = seq.next_element()? else {
                    return Ok(units);
                };
                units.push(unit);
            }
        }
    }
    d.deserialize_seq(Units)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capped_output_growth_returns_fixed_budget_diagnostic() {
        let mut out = Output(Vec::new());
        let block = vec![b'x'; 1024];
        for _ in 0..MAX_RESPONSE_BYTES / 1024 {
            out.write_all(&block).unwrap();
        }
        assert_eq!(out.0.len(), MAX_RESPONSE_BYTES);
        assert!(out.0.capacity() <= MAX_RESPONSE_BYTES);
        assert!(out.write_all(b"x").is_err());
        let response = encode(true, &"x".repeat(MAX_RESPONSE_BYTES));
        assert!(!response.ok);
        let value: serde_json::Value = serde_json::from_slice(&response.bytes).unwrap();
        assert_eq!(value["error"]["code"], "E_SDK_BUDGET");
        assert!(
            compile_response(
                br#"{"format":"weave-compiler-request/1","entry_id":"a","source":"","modules":[]}"#
            )
            .ok
        );
    }
}
