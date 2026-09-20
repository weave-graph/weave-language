mod module_files;
use std::{env, fs, io::Read, process};
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let usage = || {
        eprintln!(
            "Usage: weave <check|plan|ast|describe|fingerprint|values|artifacts|view-plan> FILE.weave [--modules MAP.json] [--template NAME]\nview-plan requires --template; compilation never registers a view.\nweave fmt FILE.weave [--check|--write] formats one source file."
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
        ]
        .contains(&args[0].as_str())
    {
        usage();
    }
    let mut modules = None;
    let mut template = None;
    for option in args[2..].chunks(2) {
        if option.len() != 2 {
            usage();
        }
        match option[0].as_str() {
            "--modules" if modules.is_none() => modules = Some(&option[1]),
            "--template" if template.is_none() && args[0] == "view-plan" => {
                template = Some(&option[1])
            }
            _ => usage(),
        }
    }
    if (args[0] == "view-plan") != template.is_some() {
        usage();
    }
    let source = read_source(&args[1]).unwrap_or_else(|e| {
        eprintln!(
            "{}",
            serde_json::json!({"code":"E_IO","message":e.to_string()})
        );
        process::exit(1)
    });
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
        match args[0].as_str() {
            "artifacts" | "view-plan" | "check" => {
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
    if ["check", "artifacts", "view-plan"].contains(&args[0].as_str()) {
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
        "check" => println!(
            "{}",
            serde_json::json!({"status":"valid","version":output.program.version,"commands":output.program.commands.len(),"view_templates":output.view_templates.len(),"host_registration_required":!output.view_templates.is_empty()})
        ),
        _ => println!(
            "{}",
            serde_json::json!({"artifacts":output,"artifact_fingerprint":output.fingerprint().unwrap()})
        ),
    }
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
