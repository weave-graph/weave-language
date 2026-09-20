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
        let errors:Vec<_>=["value N integer_div(3,2);","function F revision \"1\" (integer input) returns integer {return integer_add(param input,integer_div(1,0));}","value X time 5; function F revision \"1\" (integer input) returns integer {return param input;} apply A from F {integer input value X;}"]
            .iter().map(|s|weave_language::specialize(s).unwrap_err()).collect();
        let artifact_source = "value Answer integer_add(20,22); live_handle Head graph \"Fleet\" branch \"main\"; view_template Active revision \"1\" from Head clock tick {match relation \"connected\"; at 0;}";
        let artifacts = weave_language::compile_artifacts(artifact_source).expect("fixed artifact fixture");
        let artifact_error = weave_language::compile(artifact_source).unwrap_err();
        let handler_source = include_str!("handlers.weave");
        let handlers = weave_language::compile_artifacts(handler_source).expect("fixed handler fixture");
        let handler_error = weave_language::compile(handler_source).unwrap_err();
        serde_json::to_vec(&serde_json::json!({"profile":"weave-source-scalar-artifact-parity-v3","cases":cases,"errors":errors,"artifacts":artifacts,"artifact_fingerprint":artifacts.fingerprint().unwrap(),"artifact_error":artifact_error,"handlers":handlers,"handler_fingerprint":handlers.fingerprint().unwrap(),"handler_error":handler_error})).unwrap()
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
