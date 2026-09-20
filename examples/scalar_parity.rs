//! Fixed compiler conformance fixtures, not an arbitrary source or host execution API.
use std::sync::OnceLock;
fn bytes() -> &'static [u8] {
    static OUTPUT: OnceLock<Vec<u8>> = OnceLock::new();
    OUTPUT.get_or_init(|| {
        let sources=[
            include_str!("../docs/proposals/scalar-functions/fixtures/arithmetic.weave"),
            include_str!("../docs/proposals/scalar-functions/fixtures/higher_order.weave"),
            include_str!("../docs/proposals/scalar-functions/fixtures/typed_graph.weave"),
            "value Less decimal_lt(decimal \"-999999999999999999\",decimal \"0.000000000000000001\"); value Text string_concat(\"λ\",\"✓\");",
        ];
        let cases:Vec<_>=sources.iter().map(|s|{let r=weave_language::specialize(s).expect("fixed accepted fixture");serde_json::json!({"program":r.program,"values":r.values,"fingerprint":r.fingerprint().unwrap()})}).collect();
        let vector_source = include_str!("vectors.weave");
        let vectors = weave_language::compile_artifacts(vector_source).expect("fixed vector fixture");
        let vector_declaration = r#"vector_type P space {"id":"physical","revision":"1","geometry":{"kind":"physical3d","frame":"A","unit":"metre"}} role position;"#;
        let vector_boundaries = weave_language::compile_artifacts(&format!("{vector_declaration} value V vector P [1.7976931348623157e308,5e-324,-0.0];")).unwrap();
        let vector_error = weave_language::compile_artifacts(&format!("{vector_declaration} value V vector P [0,0];")).unwrap_err();
        let interval_source = include_str!("intervals.weave");
        let intervals = weave_language::compile_artifacts(interval_source).expect("fixed interval fixture");
        let interval_errors:Vec<_>=["value I interval(time 1,time 1);","value I interval_end(interval_open(time 0));","value I interval_intersection(interval(time 0,time 1),interval(time 1,time 2));"].iter().map(|s|weave_language::compile_artifacts(s).unwrap_err()).collect();
        let errors:Vec<_>=["value N integer_div(3,2);","function F revision \"1\" (integer input) returns integer {return integer_add(param input,integer_div(1,0));}","value X time 5; function F revision \"1\" (integer input) returns integer {return param input;} apply A from F {integer input value X;}"]
            .iter().map(|s|weave_language::specialize(s).unwrap_err()).collect();
        let artifact_source = "value Answer integer_add(20,22); live_handle Head graph \"Fleet\" branch \"main\"; view_template Active revision \"1\" from Head clock tick {match relation \"connected\"; at 0;}";
        let artifacts = weave_language::compile_artifacts(artifact_source).expect("fixed artifact fixture");
        let artifact_error = weave_language::compile(artifact_source).unwrap_err();
        let handler_source = include_str!("handlers.weave");
        let handlers = weave_language::compile_artifacts(handler_source).expect("fixed handler fixture");
        let handler_error = weave_language::compile(handler_source).unwrap_err();
        serde_json::to_vec(&serde_json::json!({"profile":"weave-source-scalar-artifact-parity-v5","cases":cases,"vectors":vectors,"vector_boundaries":vector_boundaries,"vector_error":vector_error,"vector_fingerprint":vectors.fingerprint().unwrap(),"intervals":intervals,"interval_fingerprint":intervals.fingerprint().unwrap(),"interval_errors":interval_errors,"errors":errors,"artifacts":artifacts,"artifact_fingerprint":artifacts.fingerprint().unwrap(),"artifact_error":artifact_error,"handlers":handlers,"handler_fingerprint":handlers.fingerprint().unwrap(),"handler_error":handler_error})).unwrap()
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn scalar_fixture_ptr() -> *const u8 {
    bytes().as_ptr()
}
#[unsafe(no_mangle)]
pub extern "C" fn scalar_fixture_len() -> usize {
    bytes().len()
}
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    println!("{}", std::str::from_utf8(bytes()).unwrap());
}
