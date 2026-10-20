//! LB1 (weave-language#5): `weave sdk`, the subprocess compiler mode a host
//! (engine B4's MCP server) runs in a sandbox. One bounded request on stdin,
//! one response on stdout, stable exit codes, byte-identical to the SDK, and
//! no hang or panic on hostile input.
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use weave_language::sdk::{MAX_REQUEST_BYTES, PolicyFloor, compile_response_with_floor};

const LIMIT: Duration = Duration::from_secs(60);

struct Outcome {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Run `weave sdk ARGS` with `input` on stdin, killing it if it outlives the
/// limit (which fails the test as a hang). Stdin is written from a thread so
/// that an early exit cannot deadlock the test.
fn sdk(args: &[&str], input: &[u8]) -> Outcome {
    let mut child = Command::new(env!("CARGO_BIN_EXE_weave"))
        .arg("sdk")
        .args(args)
        .current_dir(std::env::temp_dir())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let data = input.to_vec();
    // The child may stop reading at the request bound and exit: a broken pipe
    // here is expected, not an error.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let mut stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        stdout.read_to_end(&mut out).unwrap();
        out
    });
    let mut stderr = child.stderr.take().unwrap();
    let errors = std::thread::spawn(move || {
        let mut out = Vec::new();
        stderr.read_to_end(&mut out).unwrap();
        out
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > LIMIT {
            child.kill().unwrap();
            panic!("weave sdk hung on {} input bytes", input.len());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    writer.join().unwrap();
    Outcome {
        code: status.code().expect("weave sdk was killed by a signal"),
        stdout: reader.join().unwrap(),
        stderr: errors.join().unwrap(),
    }
}

fn request(source: &str, policy: Option<Value>) -> Vec<u8> {
    let mut body = json!({"format": "weave-compiler-request/1", "entry_id": "agent",
        "source": source, "modules": []});
    if let Some(policy) = policy {
        body["policy"] = policy;
    }
    serde_json::to_vec(&body).unwrap()
}

/// One well-formed failure response, with the expected exit code and no
/// stderr noise.
fn failure(out: &Outcome) -> Value {
    assert_eq!(out.code, 1, "{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.stderr.is_empty());
    let value: Value = serde_json::from_slice(&out.stdout).expect("one JSON response");
    assert_eq!(value["format"], "weave-compiler-response/1");
    assert_eq!(value["ok"], false);
    value["error"].clone()
}

#[test]
fn responses_equal_the_sdk_byte_api() {
    let cases = [
        request(
            r#"write W graph "fleet" { assert node "n" entity "N" space "s" reader_groups "team"; }"#,
            None,
        ),
        request(
            r#"write W graph "fleet" { correct node "n" entity "N" space "s"; }"#,
            None,
        ),
        request(
            r#"use G graph "fleet"; lens L from G { }"#,
            Some(json!({"read_only": true, "untrusted": true})),
        ),
        b"{".to_vec(),
        Vec::new(),
    ];
    for input in cases {
        let out = sdk(&[], &input);
        let expected = compile_response_with_floor(&input, &PolicyFloor::default());
        assert_eq!(out.stdout, expected.bytes);
        assert_eq!(out.code, if expected.ok { 0 } else { 1 });
        assert!(out.stderr.is_empty());
    }
}

#[test]
fn diagnostics_keep_exact_spans() {
    let source = r#"write W graph "fleet" { correct node "n" entity "N" space "s"; }"#;
    let error = failure(&sdk(&[], &request(source, None)));
    assert_eq!(error["code"], "E_VISIBILITY_REQUIRED");
    assert_eq!(error["source_id"], "agent");
    let (start, end) = (
        error["start"].as_u64().unwrap() as usize,
        error["end"].as_u64().unwrap() as usize,
    );
    assert_eq!(
        &source[start..end],
        r#"correct node "n" entity "N" space "s";"#
    );
    // Non-ASCII source: spans are UTF-8 byte offsets.
    let source = "write W graph \"λ🚀\" { assert node \"n\" entity \"N\" space \"s\"; }";
    let error = failure(&sdk(
        &[],
        &request(source, Some(json!({"untrusted": true}))),
    ));
    let (start, end) = (
        error["start"].as_u64().unwrap() as usize,
        error["end"].as_u64().unwrap() as usize,
    );
    assert_eq!(
        &source[start..end],
        r#"assert node "n" entity "N" space "s";"#
    );
}

#[test]
fn request_policy_and_visible_manifest_apply() {
    let manifest = json!({"format": "weave-visible/1", "principal": "agent-1",
        "graphs": [{"graph_id": "fleet", "permission": "write", "branches": ["main"]}],
        "reader_groups": ["team"]});
    let write =
        r#"write W graph "fleet" { assert node "n" entity "N" space "s" reader_groups "team"; }"#;
    let ok = sdk(
        &[],
        &request(
            write,
            Some(json!({"untrusted": true, "visible": manifest.clone()})),
        ),
    );
    assert_eq!(ok.code, 0);
    let hidden = failure(&sdk(
        &[],
        &request(
            &write.replace("team", "finance"),
            Some(json!({"visible": manifest.clone()})),
        ),
    ));
    assert_eq!(hidden["code"], "E_UNAVAILABLE");
    let graph = failure(&sdk(
        &[],
        &request(
            r#"use P graph "payroll"; lens L from P { }"#,
            Some(json!({"visible": manifest})),
        ),
    ));
    assert_eq!(graph["code"], "E_UNAVAILABLE");
    assert_eq!(
        failure(&sdk(&[], &request(write, Some(json!({"read_only": true})))))["code"],
        "E_EFFECT_DENIED"
    );
}

#[test]
fn host_floor_cannot_be_lowered_by_the_request() {
    let write = r#"write W graph "fleet" { assert node "n" entity "N" space "s"; }"#;
    for policy in [
        None,
        Some(json!({})),
        Some(json!({"read_only": false, "untrusted": false})),
    ] {
        let input = request(write, policy);
        assert_eq!(sdk(&[], &input).code, 0);
        assert_eq!(
            failure(&sdk(&["--read-only"], &input))["code"],
            "E_EFFECT_DENIED"
        );
        assert_eq!(
            failure(&sdk(&["--untrusted"], &input))["code"],
            "E_VISIBILITY_REQUIRED"
        );
    }
    let read = request(r#"use G graph "fleet"; lens L from G { }"#, None);
    let both = sdk(&["--untrusted", "--read-only"], &read);
    assert_eq!(both.code, 0);
    assert_eq!(
        both.stdout,
        compile_response_with_floor(&read, &PolicyFloor::default()).bytes,
        "a passing request's response does not depend on the floor"
    );
}

#[test]
fn usage_errors_write_no_response() {
    for args in [
        &["--bogus"][..],
        &["--read-only", "--read-only"],
        &["file.weave"],
        &["--visible"],
    ] {
        let out = sdk(args, b"");
        assert_eq!(out.code, 2, "{args:?}");
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn hostile_stdin_never_hangs_or_panics() {
    let valid = request(r#"use G graph "fleet";"#, None);
    let mut trailing = valid.clone();
    trailing.extend_from_slice(&valid);
    let deep_json = |depth: usize| {
        let mut body = br#"{"format":"weave-compiler-request/1","entry_id":"x","source":"","modules":[],"policy":{"visible":"#.to_vec();
        body.extend(std::iter::repeat_n(b'[', depth));
        body.extend(std::iter::repeat_n(b']', depth));
        body.extend_from_slice(b"}}");
        body
    };
    let deep_source = |open: &str, close: &str, depth: usize| {
        request(
            &format!(
                "write W graph \"g\" {{ assert node \"n\" entity \"N\" space \"s\" property \"k\" {}1{}; }}",
                open.repeat(depth),
                close.repeat(depth)
            ),
            None,
        )
    };
    let mut cases: Vec<(&str, Vec<u8>, &str)> = vec![
        ("empty", Vec::new(), "E_SDK_REQUEST"),
        ("malformed", b"{\"format\":".to_vec(), "E_SDK_REQUEST"),
        ("not an object", b"[1, 2, 3]".to_vec(), "E_SDK_REQUEST"),
        ("two requests", trailing, "E_SDK_REQUEST"),
        ("invalid UTF-8", vec![0xff, 0xfe, b'{'], "E_SDK_UTF8"),
        ("NUL bytes", vec![0; 4096], "E_SDK_REQUEST"),
        ("unknown member", br#"{"format":"weave-compiler-request/1","entry_id":"x","source":"","modules":[],"shell":"rm -rf /"}"#.to_vec(), "E_SDK_REQUEST"),
        ("unknown policy member", request("", Some(json!({"trusted": true}))), "E_SDK_REQUEST"),
        ("bad manifest", request("", Some(json!({"visible": {"format": "weave-visible/9", "principal": "p", "graphs": []}}))), "E_MANIFEST"),
        ("deep JSON", deep_json(1_000_000), "E_SDK_REQUEST"),
        ("deep JSON at the decoder limit", deep_json(200), "E_SDK_REQUEST"),
        ("source token flood", deep_source("[", "]", 200_000), "E_BUDGET"),
        ("deep source arrays", deep_source("[", "]", 45_000), "E_BUDGET"),
        ("deep source objects", deep_source("{\"a\":", "}", 20_000), "E_BUDGET"),
        ("deep scalar calls", request(&format!("value X {}1{};", "integer_add(1, ".repeat(12_000), ")".repeat(12_000)), None), "E_SCALAR_BUDGET"),
        ("source over 1 MiB", request(&" ".repeat(1_048_577), None), "E_SDK_BUDGET"),
        ("over the request bound", vec![b' '; MAX_REQUEST_BYTES + 1], "E_SDK_BUDGET"),
        ("far over the request bound", vec![b'{'; 4 * MAX_REQUEST_BYTES], "E_SDK_BUDGET"),
    ];
    cases.push(("module flood", serde_json::to_vec(&json!({"format": "weave-compiler-request/1",
        "entry_id": "x", "source": "", "modules": (0..65).map(|i| json!({"id": format!("m{i}"), "revision": "1", "source": ""})).collect::<Vec<_>>()})).unwrap(), "E_SDK_BUDGET"));
    for (name, input, code) in cases {
        let start = Instant::now();
        let error = failure(&sdk(&[], &input));
        assert!(
            error["code"].as_str().unwrap().starts_with(code),
            "{name}: {error}"
        );
        assert_ne!(error["code"], "E_SDK_INTERNAL", "{name}: panicked");
        assert!(start.elapsed() < LIMIT, "{name}");
    }
    // Exactly at the request bound the input is read and decoded.
    let mut padded = valid.clone();
    padded.resize(MAX_REQUEST_BYTES, b' ');
    let out = sdk(&[], &padded);
    assert_eq!(out.code, 0, "{}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn reads_no_files_beyond_the_request() {
    // An import names a module that exists on disk next to the working
    // directory; `weave sdk` resolves only supplied units.
    let dir = std::env::temp_dir().join(format!("weave-sdk-mode-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("core.weave"), "module \"core\" revision \"1\";").unwrap();
    let source = format!(
        "import c module \"core\" revision \"1\" sha256 \"{}\";",
        weave_language::modules::content_digest("module \"core\" revision \"1\";")
    );
    let out = sdk(&[], &request(&source, None));
    assert_eq!(failure(&out)["code"], "E_MODULE_MISSING");
    let entries: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
    assert_eq!(entries.len(), 1, "weave sdk created no files");
    std::fs::remove_dir_all(&dir).unwrap();
}
