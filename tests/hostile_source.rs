//! LB1 (weave-language#5) data/code boundary: fuzzed hostile source cannot
//! produce a plan containing commands that are not present in the source.
//!
//! A deterministic generator mutates every example and a set of small
//! programs: it deletes, duplicates and splices byte ranges, inserts hostile
//! fragments (write declarations, JSON plan fragments, comment and string
//! breakouts, escapes) and replaces string literals with hostile payloads.
//! For every mutant that compiles, the plan must
//!
//! 1. decode with the contract's strict decoder and re-encode unchanged;
//! 2. contain exactly one write command for each write declaration of the
//!    source (`write`, `graph`, and one `commit_batch` per `transaction`),
//!    with the same graphs and branches, and no other write;
//! 3. name only graphs whose IDs are strings or names in the parsed source;
//! 4. keep hostile text in string values only: never as an object key or an
//!    `op`;
//! 5. under the read-only policy, compile only if it writes nothing.
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use weave_contract::decode_program;
use weave_language::policy::{Access, CompilePolicy, plan_effects};
use weave_language::syntax::Statement;

/// xorshift64*: deterministic and dependency-free.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const MARKER: &str = "EVIL";

const FRAGMENTS: &[&str] = &[
    r#"write X graph "EVIL" { retract node "n"; }"#,
    r#"graph EVIL { node "a" entity "A" space "s"; }"#,
    r#"transaction EVIL { graph EVILB { node "a" entity "A" space "s"; } }"#,
    r#"use EVIL graph "EVIL-hidden"; lens EVILL from EVIL { }"#,
    r#"function EVILF revision "1" (graph input) { write W graph "EVIL" { retract node "n"; } return input; }"#,
    r#"live_handle EVILH graph "EVIL" branch "main"; view_template EVILV revision "1" from EVILH clock tick { at 0; }"#,
    r#""}, {"op": "commit", "graph_id": "EVIL", "branch_id": "main"}"#,
    r#"{"op":"write","graph_id":"EVIL","changes":[]}"#,
    r#"\"; write X graph \"EVIL\" { retract node \"n\"; } //"#,
    "\" ; } write EVILW graph \"EVIL\" { retract node \"n\"; } {",
    "*/ write EVILW graph \"EVIL\" { retract node \"n\"; } /*",
    "// write EVILW graph \"EVIL\" { retract node \"n\"; }\n",
    "\n",
    ";",
    "}",
    "{",
    r#" public "#,
    r#" readers "EVIL" "#,
    r#" reader_groups "EVIL", "weave:administrators" "#,
    r#" correct node "EVIL" entity "E" space "s"; "#,
    r#"""EVIL"""#,
    "\"EVIL\u{0}\"",
    "\"λ🚀EVIL\"",
];

const PAYLOADS: &[&str] = &[
    r#"EVIL"}, {"op": "commit", "graph_id": "EVIL"#,
    "EVIL\"; write X graph \"EVIL\" { retract node \"n\"; }",
    "EVIL */ // \n write",
    r#"{"op":"write","graph_id":"EVIL"}"#,
    "EVIL\u{2028}\u{feff}",
    "weave:EVIL",
    "$event EVIL",
];

const SMALL: &[&str] = &[
    r#"write W graph "fleet" { assert node "n" entity "N" space "s" property "text" "hello" reader_groups "team"; correct edge "e" from "a" to "b" relation "r" valid 0 until 5 public; retract node "m"; }"#,
    r#"use F graph "fleet"; lens L from F { match relation "r"; at 5; }"#,
    r#"graph G { node "a" entity "A" space "s" readers "alice"; node "b" entity "B" space "s"; edge "ab" from "a" to "b" relation "r" valid 0 until infinity public; }"#,
    r#"use F graph "fleet"; expand E from F { seeds "unit-1"; hops 2; } changes C from F { since head; }"#,
];

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            // Module units need a map; the entry sources fail to compile alone.
            if !path.join("map.json").exists() {
                sources(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "weave") {
            out.push(path);
        }
    }
}

fn boundary(source: &str, mut index: usize) -> usize {
    index = index.min(source.len());
    while !source.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn mutate(rng: &mut Rng, seed: &str, corpus: &[String]) -> String {
    let mut source = seed.to_string();
    for _ in 0..1 + rng.below(3) {
        let len = source.len();
        let a = boundary(&source, rng.below(len + 1));
        let b = boundary(&source, a + rng.below(48));
        match rng.below(5) {
            0 => source.replace_range(a..b, ""),
            1 => {
                let copy = source[a..b].to_string();
                source.insert_str(b, &copy);
            }
            2 => {
                let other = &corpus[rng.below(corpus.len())];
                let x = boundary(other, rng.below(other.len() + 1));
                let y = boundary(other, x + rng.below(120));
                source.insert_str(a, &other[x..y]);
            }
            3 => source.insert_str(a, FRAGMENTS[rng.below(FRAGMENTS.len())]),
            _ => {
                // Replace one string literal's contents with an escaped payload.
                let quotes: Vec<usize> = source.match_indices('"').map(|(i, _)| i).collect();
                if quotes.len() >= 2 {
                    let i = rng.below(quotes.len() - 1);
                    let (start, end) = (quotes[i], quotes[i + 1]);
                    let payload =
                        serde_json::to_string(PAYLOADS[rng.below(PAYLOADS.len())]).unwrap();
                    source.replace_range(start..=end, &payload);
                }
            }
        }
    }
    source
}

fn strings(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::String(s) => {
            out.insert(s.clone());
        }
        Value::Array(items) => items.iter().for_each(|v| strings(v, out)),
        Value::Object(map) => {
            for (k, v) in map {
                out.insert(k.clone());
                strings(v, out);
            }
        }
        _ => {}
    }
}

/// Object keys and `op` values of a plan, as (path, text). Everything inside a
/// property map is user data (structured property values have their own keys),
/// so those keys are skipped.
fn keys_and_ops(value: &Value, path: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                keys_and_ops(v, &format!("{path}/{i}"), out)
            }
        }
        Value::Object(map) => {
            let data = path.contains("/properties") || path.contains("/assertion_properties");
            for (k, v) in map {
                let at = format!("{path}/{k}");
                if !data {
                    out.push((at.clone(), k.clone()));
                }
                if k == "op" {
                    out.push((at.clone(), v.as_str().unwrap_or_default().to_string()));
                }
                keys_and_ops(v, &at, out);
            }
        }
        _ => {}
    }
}

/// The write commands the source declares: (op, graph, branch).
fn declared_writes(statements: &[Statement], out: &mut Vec<(String, String, String)>) {
    for statement in statements {
        match statement {
            Statement::Write { graph, branch, .. } => out.push((
                "write".into(),
                graph.clone(),
                branch.clone().unwrap_or_else(|| "main".into()),
            )),
            Statement::Graph { name, branch, .. } => {
                out.push(("commit".into(), name.clone(), branch.clone()))
            }
            Statement::ContextValue { name, .. } => {
                out.push(("commit".into(), name.clone(), "main".into()))
            }
            Statement::Transaction { body, .. } => {
                for member in body {
                    match member {
                        Statement::Graph { name, branch, .. } => {
                            out.push(("commit_batch".into(), name.clone(), branch.clone()))
                        }
                        Statement::ContextValue { name, .. } => {
                            out.push(("commit_batch".into(), name.clone(), "main".into()))
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

struct Stats {
    compiled: usize,
    rejected: usize,
    read_only_denied: usize,
    with_marker: usize,
}

fn check(source: &str, stats: &mut Stats) {
    let Ok(program) = weave_language::compile(source) else {
        stats.rejected += 1;
        return;
    };
    stats.compiled += 1;
    let parsed = weave_language::parse(source).expect("a compiled source parses");
    let value = serde_json::to_value(&program).unwrap();
    // 1. Strict contract round trip.
    let decoded = decode_program(&serde_json::to_vec(&value).unwrap())
        .unwrap_or_else(|e| panic!("not core v1 ({e}): {source}"));
    assert_eq!(serde_json::to_value(decoded).unwrap(), value, "{source}");
    // 2. Write commands are exactly the declared writes.
    let mut declared = Vec::new();
    declared_writes(&parsed.statements, &mut declared);
    declared.sort();
    let effects = plan_effects(&program);
    let mut written: Vec<(String, String, String)> = effects
        .iter()
        .filter(|e| e.access == Access::Write)
        .map(|e| {
            (
                e.op.clone(),
                e.graph_id.clone(),
                e.branch_id.clone().unwrap_or_default(),
            )
        })
        .collect();
    written.sort();
    assert_eq!(
        written, declared,
        "write commands differ from source: {source}"
    );
    let write_commands = program
        .commands
        .iter()
        .filter(|c| {
            let op = serde_json::to_value(c).unwrap()["op"].clone();
            weave_language::policy::WRITE_OPS.contains(&op.as_str().unwrap_or_default())
        })
        .count();
    let batches = parsed
        .statements
        .iter()
        .filter(|s| matches!(s, Statement::Transaction { .. }))
        .count();
    let batch_members = declared.iter().filter(|d| d.0 == "commit_batch").count();
    assert_eq!(
        write_commands,
        declared.len() - batch_members + batches,
        "{source}"
    );
    // 3. Only graphs the source names.
    let mut names = BTreeSet::new();
    strings(&serde_json::to_value(&parsed).unwrap(), &mut names);
    for effect in &effects {
        assert!(
            names.contains(&effect.graph_id),
            "graph '{}' is not in the source: {source}",
            effect.graph_id
        );
    }
    // 4. Hostile text never becomes structure.
    let mut structure = Vec::new();
    keys_and_ops(&value, "", &mut structure);
    if let Some((at, text)) = structure.iter().find(|(_, k)| k.contains(MARKER)) {
        panic!("hostile text became plan structure at {at} ({text}): {source}");
    }
    if serde_json::to_string(&value).unwrap().contains(MARKER) {
        stats.with_marker += 1;
    }
    // 5. The read-only policy admits only plans that write nothing.
    let read_only = CompilePolicy {
        read_only: true,
        ..Default::default()
    };
    match weave_language::compile_with_policy(source, &read_only) {
        Ok(checked) => {
            assert!(declared.is_empty(), "read-only admitted a write: {source}");
            assert!(
                plan_effects(&checked)
                    .iter()
                    .all(|e| e.access != Access::Write),
                "{source}"
            );
            assert_eq!(checked, program, "{source}");
        }
        Err(e) => {
            assert_eq!(e.code, "E_EFFECT_DENIED", "{source}");
            assert!(!declared.is_empty(), "{source}");
            stats.read_only_denied += 1;
        }
    }
}

#[test]
fn fuzzed_hostile_source_cannot_add_commands() {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("examples"),
        &mut files,
    );
    files.sort();
    let mut corpus: Vec<String> = files
        .iter()
        .map(|f| std::fs::read_to_string(f).unwrap())
        .collect();
    corpus.extend(SMALL.iter().map(|s| s.to_string()));
    let mut stats = Stats {
        compiled: 0,
        rejected: 0,
        read_only_denied: 0,
        with_marker: 0,
    };
    for seed in &corpus {
        check(seed, &mut stats);
    }
    let iterations: usize = std::env::var("WEAVE_FUZZ_ITERATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000);
    let mut rng = Rng(0x05ee_d1b1_0005);
    for _ in 0..iterations {
        let seed = &corpus[rng.below(corpus.len())];
        let mutant = mutate(&mut rng, seed, &corpus);
        check(&mutant, &mut stats);
    }
    eprintln!(
        "hostile source: {iterations} mutants, {} compiled, {} rejected, {} denied by read-only, {} carried hostile data",
        stats.compiled, stats.rejected, stats.read_only_denied, stats.with_marker
    );
    // The generator must actually exercise both outcomes and hostile data.
    assert!(stats.compiled > iterations / 20, "{}", stats.compiled);
    assert!(stats.rejected > iterations / 20, "{}", stats.rejected);
    assert!(stats.read_only_denied > 0);
    assert!(
        stats.with_marker > 0,
        "no hostile payload reached a plan as data"
    );
}

#[test]
fn hostile_strings_are_data_in_the_plan() {
    for payload in PAYLOADS {
        let literal = serde_json::to_string(payload).unwrap();
        // Identities reject control characters; properties keep any text.
        let id = if payload.chars().any(char::is_control) {
            serde_json::to_string(&payload.replace(char::is_control, " ")).unwrap()
        } else {
            literal.clone()
        };
        let source = format!(
            "write W graph \"fleet\" {{ assert node {id} entity {id} space \"s\" property {literal} {literal} readers {id}; }}"
        );
        let program = weave_language::compile(&source).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(program.commands.len(), 1);
        let value = serde_json::to_value(&program).unwrap();
        let fact = &value["commands"][0]["changes"][0]["fact"];
        let id: String = serde_json::from_str(&id).unwrap();
        assert_eq!(fact["id"], id);
        assert_eq!(fact["entity_id"], id);
        assert_eq!(fact["properties"][*payload], *payload);
        assert_eq!(fact["readers"][0], id);
        assert_eq!(value["commands"][0]["graph_id"], "fleet");
    }
}

/// Run `weave sdk` once; a hang fails the test.
fn weave_sdk(args: &[&str], input: &[u8]) -> (i32, Vec<u8>) {
    use std::io::{Read, Write};
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_weave"))
        .arg("sdk")
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let mut stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        stdout.read_to_end(&mut out).unwrap();
        out
    });
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > std::time::Duration::from_secs(60) {
            child.kill().unwrap();
            panic!("weave sdk hung");
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    writer.join().unwrap();
    (
        status.code().expect("killed by a signal"),
        reader.join().unwrap(),
    )
}

/// The same hostile sources, and hostile request envelopes, through the
/// subprocess mode a host sandboxes (`weave sdk`): every run exits 0 or 1 with
/// exactly the in-process SDK response, a passing response's plan is the
/// ordinary compiled plan, and a read-only one never writes.
#[test]
fn fuzzed_hostile_source_through_weave_sdk() {
    use weave_language::sdk::{PolicyFloor, compile_response_with_floor};
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("examples"),
        &mut files,
    );
    files.sort();
    let mut corpus: Vec<String> = files
        .iter()
        .map(|f| std::fs::read_to_string(f).unwrap())
        .collect();
    corpus.extend(SMALL.iter().map(|s| s.to_string()));
    let runs: usize = std::env::var("WEAVE_SDK_FUZZ_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_000);
    let mut rng = Rng(0x0b1b_5d4d_0001);
    let (mut passed, mut failed) = (0, 0);
    for run in 0..runs {
        let seed = &corpus[rng.below(corpus.len())];
        let source = mutate(&mut rng, seed, &corpus);
        let (policy, read_only) = match rng.below(4) {
            0 => (None, false),
            1 => (Some(serde_json::json!({"read_only": true})), true),
            2 => (Some(serde_json::json!({"untrusted": true})), false),
            _ => (
                Some(serde_json::json!({"untrusted": true, "read_only": true})),
                true,
            ),
        };
        let mut body = serde_json::json!({"format": "weave-compiler-request/1",
            "entry_id": "fuzz", "source": source, "modules": []});
        if let Some(policy) = policy {
            body["policy"] = policy;
        }
        let mut input = serde_json::to_vec(&body).unwrap();
        // Every fourth request also has its envelope bytes mutated.
        if run % 4 == 3 && !input.is_empty() {
            for _ in 0..1 + rng.below(4) {
                let at = rng.below(input.len());
                let bytes = b"{}[]\",:\\\x00\xff9 ";
                input[at] = bytes[rng.below(bytes.len())];
            }
        }
        let floor = if rng.below(5) == 0 {
            PolicyFloor {
                untrusted: false,
                read_only: true,
            }
        } else {
            PolicyFloor::default()
        };
        let args: &[&str] = if floor.read_only {
            &["--read-only"]
        } else {
            &[]
        };
        let (code, stdout) = weave_sdk(args, &input);
        let expected = compile_response_with_floor(&input, &floor);
        assert_eq!(stdout, expected.bytes, "weave sdk differs from the SDK");
        let response: Value = serde_json::from_slice(&stdout).expect("one JSON response");
        assert_ne!(response["error"]["code"], "E_SDK_INTERNAL", "panicked");
        if !expected.ok {
            assert_eq!(code, 1);
            failed += 1;
            continue;
        }
        assert_eq!(code, 0);
        passed += 1;
        let program: weave_contract::Program =
            serde_json::from_value(response["artifacts"]["program"].clone()).unwrap();
        let bytes = serde_json::to_vec(&response["artifacts"]["program"]).unwrap();
        decode_program(&bytes).expect("a strict core v1 plan");
        if let Ok(request) = serde_json::from_slice::<Value>(&input)
            && let Some(source) = request["source"].as_str()
            && request["modules"].as_array().is_some_and(|m| m.is_empty())
        {
            if let Ok(plain) = weave_language::compile(source) {
                assert_eq!(program, plain, "a policy changed a passing plan");
            }
            let mut declared = Vec::new();
            if let Ok(parsed) = weave_language::parse(source) {
                declared_writes(&parsed.statements, &mut declared);
            }
            let writes = plan_effects(&program)
                .iter()
                .filter(|e| e.access == Access::Write)
                .count();
            if read_only || floor.read_only {
                assert_eq!(writes, 0, "read-only response wrote: {source}");
            }
            assert_eq!(writes, declared.len(), "{source}");
        }
    }
    eprintln!("weave sdk fuzz: {runs} runs, {passed} passed, {failed} failed");
    assert!(passed > 0 && failed > 0);
}
