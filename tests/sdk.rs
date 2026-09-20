use serde_json::{Value, json};
use weave_language::{
    compile_artifacts,
    modules::{SourceModule, content_digest, link},
    sdk::*,
};
fn request(source: &str) -> Vec<u8> {
    serde_json::to_vec(
        &json!({"format":REQUEST_FORMAT,"entry_id":"entry.weave","source":source,"modules":[]}),
    )
    .unwrap()
}
fn response(bytes: &[u8]) -> Value {
    serde_json::from_slice(&compile_request(bytes)).unwrap()
}
fn assert_error(bytes: &[u8], code: &str) {
    let r = response(bytes);
    assert_eq!(r["ok"], false, "{r}");
    assert_eq!(r["error"]["code"], code, "{r}");
}
#[test]
fn no_import_artifacts_and_fingerprints_are_exact() {
    for source in [
        include_str!("../examples/vectors.weave"),
        include_str!("../examples/handlers.weave"),
        include_str!("../examples/intervals.weave"),
        "value I 9007199254740993; value Max 9223372036854775807; value Min -9223372036854775808; value D decimal \"0.000000000000000001\"; value U \"λ🚀\";",
        "value N 42; live_handle H graph \"G\" branch \"main\"; view_template V revision \"1\" from H clock fixed {}",
    ] {
        let r = response(&request(source));
        assert_eq!(r["ok"], true, "{r}");
        let expected = compile_artifacts(source).unwrap();
        assert_eq!(r["artifacts"], serde_json::to_value(&expected).unwrap());
        assert_eq!(r["artifact_fingerprint"], expected.fingerprint().unwrap());
    }
}
#[test]
fn original_diagnostics_and_linked_identity() {
    let module = "module \"core\" revision \"1\"; function Keep revision \"1\" (integer input) returns integer {return param input;}";
    let entry = format!(
        "import c module \"core\" revision \"1\" sha256 {:?}; apply V from c::Keep {{integer input 9007199254740993;}}",
        content_digest(module)
    );
    let req = json!({"format":REQUEST_FORMAT,"entry_id":"entry.weave","source":entry,"modules":[{"id":"core","revision":"1","source":module}]});
    let result = response(&serde_json::to_vec(&req).unwrap());
    let expected = link(
        "entry.weave",
        &entry,
        &[SourceModule {
            id: "core",
            revision: "1",
            source: module,
        }],
    )
    .unwrap()
    .compile_artifacts()
    .unwrap();
    assert_eq!(
        result["artifacts"],
        serde_json::to_value(&expected).unwrap()
    );
    assert_eq!(
        result["artifact_fingerprint"],
        expected.fingerprint().unwrap()
    );
    let bad = "module \"core\" revision \"1\"; function Bad revision \"1\" () returns integer {return integer_div(1,0);}";
    let source = format!(
        "import c module \"core\" revision \"1\" sha256 {:?};",
        content_digest(bad)
    );
    let req = json!({"format":REQUEST_FORMAT,"entry_id":"entry.weave","source":source,"modules":[{"id":"core","revision":"1","source":bad}]});
    let r = response(&serde_json::to_vec(&req).unwrap());
    let error = link(
        "entry.weave",
        &source,
        &[SourceModule {
            id: "core",
            revision: "1",
            source: bad,
        }],
    )
    .unwrap()
    .compile_artifacts()
    .unwrap_err();
    assert_eq!(r["error"], serde_json::to_value(error).unwrap());
    let plain = "value X integer_div(1,0);";
    let error = compile_artifacts(plain).unwrap_err();
    let r = response(&request(plain));
    assert_eq!(r["error"]["code"], error.code);
    assert_eq!(r["error"]["start"], error.start);
    assert_eq!(r["error"]["end"], error.end);
}
#[test]
fn strict_transport_and_decoded_bounds() {
    assert_error(&[255], "E_SDK_UTF8");
    for raw in [
        r#"{"format":"weave-compiler-request/1","entry_id":"a","source":"","source":"","modules":[]}"#,
        r#"{"format":"weave-compiler-request/1","entry_id":"a","source":"","modules":[],"extra":0}"#,
        r#"{"format":"weave-compiler-request/1","entry_id":"a","source":"","modules":[{"id":"x","id":"y","revision":"1","source":""}]}"#,
        r#"{"format":"weave-compiler-request/1","entry_id":"a","source":"\ud800","modules":[]}"#,
        "null",
        "{}",
    ] {
        assert_error(raw.as_bytes(), "E_SDK_REQUEST");
    }
    assert_error(&vec![b' '; MAX_REQUEST_BYTES + 1], "E_SDK_BUDGET");
    assert_error(
        &request(&" ".repeat(weave_language::modules::MAX_UNIT_BYTES + 1)),
        "E_SDK_BUDGET",
    );
    let units:Vec<_>=(0..5).map(|i|json!({"id":format!("m{i}"),"revision":"1","source":" ".repeat(weave_language::modules::MAX_UNIT_BYTES)})).collect();
    assert_error(
        &serde_json::to_vec(
            &json!({"format":REQUEST_FORMAT,"entry_id":"a","source":"","modules":units}),
        )
        .unwrap(),
        "E_SDK_BUDGET",
    );
    // The 65th element is refused without visiting its malformed/deep body.
    let unit = r#"{"id":"m","revision":"1","source":""}"#;
    let raw = format!(
        r#"{{"format":"weave-compiler-request/1","entry_id":"a","source":"","modules":[{},{}]}}"#,
        vec![unit; 64].join(","),
        "[".repeat(1024)
    );
    assert_error(raw.as_bytes(), "E_SDK_BUDGET");
    // Serde's escaped scratch buffer is bounded by raw transport, decoded units separately.
    let raw = format!(
        r#"{{"format":"weave-compiler-request/1","entry_id":"a","source":"{}","modules":[]}}"#,
        "\\u0020".repeat(weave_language::modules::MAX_UNIT_BYTES + 1)
    );
    assert_error(raw.as_bytes(), "E_SDK_BUDGET");
    assert_eq!(response(&request("value Fine 1;"))["ok"], true);
}

#[test]
fn unused_supplied_units_do_not_change_plain_source_and_missing_import_uses_linker() {
    let source = "function Z revision \"1\" (integer x) returns integer {return param x;} function A revision \"1\" (integer x) returns integer {return param x;} value V 1;";
    let request = json!({"format":REQUEST_FORMAT,"entry_id":"main","source":source,"modules":[{"id":"unused","revision":"1","source":"module \"unused\" revision \"1\";"}]});
    let r = response(&serde_json::to_vec(&request).unwrap());
    let ordinary = compile_artifacts(source).unwrap();
    assert_eq!(r["artifacts"], serde_json::to_value(&ordinary).unwrap());
    assert_eq!(r["artifact_fingerprint"], ordinary.fingerprint().unwrap());
    let missing = format!(
        "import x module \"missing\" revision \"1\" sha256 {:?};",
        "0".repeat(64)
    );
    let r = response(
        &serde_json::to_vec(
            &json!({"format":REQUEST_FORMAT,"entry_id":"main","source":missing,"modules":[]}),
        )
        .unwrap(),
    );
    assert_eq!(
        r["error"],
        serde_json::to_value(link("main", &missing, &[]).err().unwrap()).unwrap()
    );
}

#[test]
fn caller_field_names_cannot_reclassify_transport_diagnostics() {
    assert_error(br#"{"format":"weave-compiler-request/1","entry_id":"a","source":"","modules":[],"E_SDK_BUDGET":0}"#, "E_SDK_REQUEST");
}
