//! Native safe-byte API driver, used to compare arbitrary requests with both ABIs.
use std::io::{Read, Write};
fn main() {
    let mut input = Vec::new();
    std::io::stdin()
        .take((weave_language::sdk::MAX_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut input)
        .expect("read request");
    std::io::stdout()
        .write_all(&weave_language::sdk::compile_request(&input))
        .expect("write response");
}
