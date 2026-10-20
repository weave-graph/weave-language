mod module_files;
use std::{env, fs, io::Read, process};
fn main() {
    let mut args: Vec<_> = env::args().skip(1).collect();
    // `weave sdk` takes its own flags and reads only stdin.
    if args.first().is_some_and(|arg| arg == "sdk") {
        sdk_command(&args[1..]);
    }
    // `describe --contract` adds the core contract v1 classification.
    let contract = args.first().is_some_and(|arg| arg == "describe")
        && args.iter().filter(|arg| *arg == "--contract").count() == 1;
    if contract {
        args.retain(|arg| arg != "--contract");
    }
    // LB1 compile policy: `--untrusted`, `--read-only` and `--visible MANIFEST.json`.
    let mut policy = weave_language::policy::CompilePolicy::default();
    for (flag, field) in [
        ("--untrusted", &mut policy.untrusted),
        ("--read-only", &mut policy.read_only),
    ] {
        let count = args.iter().filter(|arg| *arg == flag).count();
        if count > 1 {
            args.clear();
        }
        *field = count == 1;
        args.retain(|arg| arg != flag);
    }
    let usage = || {
        eprintln!(
            "Usage: weave <check|plan|ast|describe|fingerprint|values|artifacts|view-plan|handler-plan|effects> FILE.weave [--modules MAP.json] [--template NAME|--handler NAME] [--contract] [--untrusted] [--read-only] [--visible MANIFEST.json]\nview-plan requires --template; handler-plan requires --handler; --contract is for describe only; compilation never installs host artifacts.\n--untrusted, --read-only and --visible add compile-time policy checks; the engine still authorizes everything.\nweave fmt FILE.weave [--check|--write] formats one source file.\nweave sdk [--untrusted] [--read-only] compiles one weave-compiler-request/1 from stdin to stdout."
        );
        process::exit(2);
    };
    if args.first().is_some_and(|arg| arg == "fmt") {
        format_command(&args);
        return;
    }
    if args.len() < 2
        || ![
            "check",
            "plan",
            "ast",
            "describe",
            "fingerprint",
            "values",
            "artifacts",
            "view-plan",
            "handler-plan",
            "effects",
        ]
        .contains(&args[0].as_str())
    {
        usage();
    }
    let mut modules = None;
    let mut template = None;
    let mut visible = None;
    for option in args[2..].chunks(2) {
        if option.len() != 2 {
            usage();
        }
        match option[0].as_str() {
            "--modules" if modules.is_none() => modules = Some(&option[1]),
            "--visible" if visible.is_none() => visible = Some(&option[1]),
            "--template" if template.is_none() && args[0] == "view-plan" => {
                template = Some(&option[1])
            }
            "--handler" if template.is_none() && args[0] == "handler-plan" => {
                template = Some(&option[1])
            }
            _ => usage(),
        }
    }
    if (["view-plan", "handler-plan"].contains(&args[0].as_str())) != template.is_some() {
        usage();
    }
    let source = read_source(&args[1]).unwrap_or_else(|e| {
        eprintln!(
            "{}",
            serde_json::json!({"code":"E_IO","message":e.to_string()})
        );
        process::exit(1)
    });
    if let Some(path) = visible {
        let bytes = fs::File::open(path)
            .and_then(|file| {
                let mut bytes = Vec::new();
                file.take(weave_language::policy::MAX_MANIFEST_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map(|_| bytes)
            })
            .unwrap_or_else(|e| {
                eprintln!(
                    "{}",
                    serde_json::json!({"code":"E_IO","message":e.to_string()})
                );
                process::exit(1)
            });
        policy.visible = Some(
            weave_language::policy::VisibleManifest::decode(&bytes).unwrap_or_else(|e| fail(e)),
        );
    }
    let effects = args[0] == "effects";
    if let Some(map) = modules {
        let units =
            module_files::load(std::path::Path::new(map), source.len()).unwrap_or_else(|e| {
                eprintln!(
                    "{}",
                    serde_json::json!({"code":"E_MODULE_IO", "message":e.to_string()})
                );
                process::exit(1)
            });
        let supplied: Vec<_> = units
            .iter()
            .map(|u| weave_language::modules::SourceModule {
                id: &u.id,
                revision: &u.revision,
                source: &u.source,
            })
            .collect();
        let linked =
            weave_language::modules::link("entry", &source, &supplied).unwrap_or_else(module_fail);
        if !policy.is_default() || effects {
            let artifacts = linked.compile_artifacts().unwrap_or_else(module_fail);
            let parsed = weave_language::parse(&source).unwrap_or_else(|e| fail(e));
            weave_language::policy::check(&source, &parsed, &artifacts, &policy)
                .unwrap_or_else(|e| fail(e));
            if effects {
                print_effects(&artifacts.program);
                return;
            }
        }
        match args[0].as_str() {
            "artifacts" | "view-plan" | "handler-plan" | "check" => {
                let output = linked.compile_artifacts().unwrap_or_else(module_fail);
                show_artifacts(&args[0], template.map(String::as_str), output);
            }
            "values" => {
                let output = linked.specialize().unwrap_or_else(module_fail);
                println!(
                    "{}",
                    serde_json::json!({"values":output.values,"specialization_fingerprint":output.fingerprint().unwrap()})
                );
            }
            "ast" => println!("{}", serde_json::to_string_pretty(linked.ast()).unwrap()),
            "describe" if contract => {
                let description = linked.describe_contract().unwrap_or_else(module_fail);
                println!("{}", serde_json::to_string_pretty(&description).unwrap());
            }
            "describe" => {
                linked.require_no_artifacts().unwrap_or_else(module_fail);
                linked.compile().unwrap_or_else(module_fail);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&linked.schemas()).unwrap()
                );
            }
            "fingerprint" => println!(
                "{}",
                serde_json::json!({"plan_fingerprint":linked.fingerprint().unwrap_or_else(module_fail)})
            ),
            _ => {
                let plan = linked.compile().unwrap_or_else(module_fail);
                if args[0] == "plan" {
                    println!("{}", serde_json::to_string_pretty(&plan).unwrap());
                } else {
                    println!(
                        "{}",
                        serde_json::json!({"status":"valid","version":plan.version,"commands":plan.commands.len()})
                    );
                }
            }
        }
        return;
    }
    if !policy.is_default() || effects {
        let artifacts = weave_language::compile_artifacts_with_policy(&source, &policy)
            .unwrap_or_else(|e| fail(e));
        if effects {
            print_effects(&artifacts.program);
            return;
        }
    }
    if ["check", "artifacts", "view-plan", "handler-plan"].contains(&args[0].as_str()) {
        let output = weave_language::compile_artifacts(&source).unwrap_or_else(|e| fail(e));
        show_artifacts(&args[0], template.map(String::as_str), output);
    } else if args[0] == "values" {
        let output = weave_language::specialize(&source).unwrap_or_else(|e| fail(e));
        println!(
            "{}",
            serde_json::json!({"values":output.values,"specialization_fingerprint":output.fingerprint().unwrap()})
        );
    } else if args[0] == "fingerprint" {
        match weave_language::fingerprint(&source) {
            Ok(id) => println!("{}", serde_json::json!({"plan_fingerprint":id})),
            Err(e) => fail(e),
        }
    } else if args[0] == "describe" && contract {
        match weave_language::describe_contract(&source) {
            Ok(description) => println!("{}", serde_json::to_string_pretty(&description).unwrap()),
            Err(e) => fail(e),
        }
    } else if args[0] == "describe" {
        match weave_language::describe(&source) {
            Ok(schemas) => println!("{}", serde_json::to_string_pretty(&schemas).unwrap()),
            Err(e) => fail(e),
        }
    } else if args[0] == "ast" {
        match weave_language::parse(&source) {
            Ok(ast) => println!("{}", serde_json::to_string_pretty(&ast).unwrap()),
            Err(e) => fail(e),
        }
    } else {
        match weave_language::compile(&source) {
            Ok(plan) => {
                if args[0] == "plan" {
                    println!("{}", serde_json::to_string_pretty(&plan).unwrap())
                } else {
                    println!(
                        "{}",
                        serde_json::json!({"status":"valid","version":plan.version,"commands":plan.commands.len()})
                    )
                }
            }
            Err(e) => fail(e),
        }
    }
}
fn show_artifacts(
    command: &str,
    template: Option<&str>,
    output: weave_language::CompiledArtifacts,
) {
    match command {
        "view-plan" => {
            let name = template.expect("argument checked");
            let artifact = output.view_templates.get(name).unwrap_or_else(|| {
                fail(weave_language::Diagnostic {
                    code: "E_VIEW_TEMPLATE".into(),
                    message: "Selected template is not declared".into(),
                    start: 0,
                    end: 0,
                    trace: vec![],
                })
            });
            println!("{}", serde_json::to_string_pretty(artifact).unwrap());
        }
        "handler-plan" => {
            let name = template.expect("argument checked");
            let artifact = output.handler_templates.get(name).unwrap_or_else(|| {
                fail(weave_language::Diagnostic {
                    code: "E_HANDLER_TEMPLATE".into(),
                    message: "Selected handler is not declared".into(),
                    start: 0,
                    end: 0,
                    trace: vec![],
                })
            });
            println!("{}", serde_json::to_string_pretty(artifact).unwrap());
        }
        "check" => println!(
            "{}",
            serde_json::json!({"status":"valid","version":output.program.version,"commands":output.program.commands.len(),"view_templates":output.view_templates.len(),"handler_templates":output.handler_templates.len(),"host_registration_required":!output.view_templates.is_empty() || !output.handler_templates.is_empty()})
        ),
        _ => {
            // Serialize directly (not through `serde_json::Value`), so binary32
            // embedding components print as in `plan` and the SDK.
            #[derive(serde::Serialize)]
            struct Output<'a> {
                artifacts: &'a weave_language::CompiledArtifacts,
                artifact_fingerprint: String,
            }
            println!(
                "{}",
                serde_json::to_string(&Output {
                    artifacts: &output,
                    artifact_fingerprint: output.fingerprint().unwrap(),
                })
                .unwrap()
            )
        }
    }
}
/// `weave effects`: every graph the plan reads, references or writes (LB1).
fn print_effects(program: &weave_contract::Program) {
    let effects = weave_language::policy::plan_effects(program);
    let writes = effects
        .iter()
        .filter(|e| e.access == weave_language::policy::Access::Write)
        .count();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "version": program.version,
            "commands": program.commands.len(),
            "writes": writes,
            "effects": effects,
        }))
        .unwrap()
    );
}
fn module_fail<T>(error: weave_language::modules::ModuleDiagnostic) -> T {
    eprintln!("{}", serde_json::to_string(&error).unwrap());
    process::exit(1)
}
fn fail(error: weave_language::Diagnostic) -> ! {
    eprintln!("{}", serde_json::to_string(&error).unwrap());
    process::exit(1)
}

fn read_source(path: &str) -> std::io::Result<String> {
    let mut source = String::new();
    fs::File::open(path)?
        .take(1_048_577)
        .read_to_string(&mut source)?;
    Ok(source)
}

fn format_command(args: &[String]) {
    if !(args.len() == 2 || (args.len() == 3 && matches!(args[2].as_str(), "--check" | "--write")))
    {
        eprintln!("Usage: weave fmt FILE.weave [--check|--write]");
        process::exit(2);
    }
    let io_fail = |error: std::io::Error| -> ! {
        eprintln!(
            "{}",
            serde_json::json!({"code":"E_IO","message":error.to_string()})
        );
        process::exit(1)
    };
    let source = read_source(&args[1]).unwrap_or_else(|e| io_fail(e));
    let formatted = weave_language::format_source(&source).unwrap_or_else(|e| fail(e));
    match args.get(2).map(String::as_str) {
        Some("--check") if formatted != source => {
            eprintln!(
                "{}",
                serde_json::json!({"code":"E_FORMAT","message":"Source needs formatting"})
            );
            process::exit(1);
        }
        Some("--write") => {
            write_formatted(std::path::Path::new(&args[1]), &source, &formatted)
                .unwrap_or_else(|e| io_fail(e));
        }
        Some("--check") => {}
        _ => print!("{formatted}"),
    }
}

/// Same-directory atomic replacement; final-component symlinks are rejected.
/// Preserve the original permission bits. No imports or module pins are changed.
fn write_formatted(path: &std::path::Path, original: &str, formatted: &str) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind, Write};
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "--write requires a regular file; symlinks are rejected",
        ));
    }
    if original == formatted {
        return Ok(());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let mut temporary = None;
    for attempt in 0..100 {
        let candidate = parent.join(format!(".weave-format-{}-{attempt}.tmp", process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    let (temporary, mut file) = temporary.ok_or_else(|| {
        Error::new(
            ErrorKind::AlreadyExists,
            "Cannot allocate formatter temporary file",
        )
    })?;
    let result = (|| {
        file.set_permissions(metadata.permissions())?;
        file.write_all(formatted.as_bytes())?;
        file.sync_all()?;
        // Refuse a detected intervening edit or replacement with a symlink.
        if !fs::symlink_metadata(path)?.file_type().is_file()
            || read_source(
                path.to_str()
                    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Invalid source path"))?,
            )? != original
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Source changed while formatting",
            ));
        }
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Exit codes of `weave sdk`: the response was `ok: true`.
const SDK_EXIT_OK: i32 = 0;
/// The response is a failure (`ok: false`): a compiler diagnostic or an
/// `E_SDK_*` transport error, with the details in the response.
const SDK_EXIT_FAILURE: i32 = 1;
/// Invalid command line; no response is written.
const SDK_EXIT_USAGE: i32 = 2;
/// The response could not be written to stdout.
const SDK_EXIT_OUTPUT: i32 = 3;

/// `weave sdk`: one bounded `weave-compiler-request/1` from stdin, one
/// response to stdout (LB1). It opens no file and no socket: everything it
/// compiles is in the request, exactly as for the C ABI and WASM SDK, and the
/// response bytes are those of `weave_language::sdk::compile_response`.
/// `--untrusted` and `--read-only` set a host policy floor that the request's
/// own `policy` cannot lower.
fn sdk_command(flags: &[String]) -> ! {
    use std::io::Write;
    let mut floor = weave_language::sdk::PolicyFloor::default();
    for flag in flags {
        let field = match flag.as_str() {
            "--untrusted" => &mut floor.untrusted,
            "--read-only" => &mut floor.read_only,
            _ => {
                eprintln!("Usage: weave sdk [--untrusted] [--read-only] < REQUEST.json");
                process::exit(SDK_EXIT_USAGE);
            }
        };
        if *field {
            eprintln!("Usage: weave sdk [--untrusted] [--read-only] < REQUEST.json");
            process::exit(SDK_EXIT_USAGE);
        }
        *field = true;
    }
    // Read at most one byte past the request bound: a larger input is
    // rejected without being read to its end.
    let mut input = Vec::new();
    let limit = weave_language::sdk::MAX_REQUEST_BYTES as u64 + 1;
    let response = match std::io::stdin().lock().take(limit).read_to_end(&mut input) {
        Ok(_) => weave_language::sdk::compile_response_with_floor(&input, &floor),
        Err(_) => weave_language::sdk::transport_failure("E_SDK_IO", "Request could not be read"),
    };
    let mut stdout = std::io::stdout().lock();
    if stdout
        .write_all(&response.bytes)
        .and_then(|()| stdout.flush())
        .is_err()
    {
        process::exit(SDK_EXIT_OUTPUT);
    }
    process::exit(if response.ok {
        SDK_EXIT_OK
    } else {
        SDK_EXIT_FAILURE
    });
}
