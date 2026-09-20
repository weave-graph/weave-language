mod module_files;
use std::{env, fs, io::Read, process};
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if !(args.len() == 2 || (args.len() == 4 && args[2] == "--modules"))
        || !["check", "plan", "ast", "describe", "fingerprint", "values"]
            .contains(&args[0].as_str())
    {
        eprintln!(
            "Usage: weave <check|plan|ast|describe|fingerprint|values> FILE.weave [--modules MAP.json]\nplan emits Weave Engine protocol JSON; host authorization belongs to the engine."
        );
        process::exit(2);
    }
    let source = read_source(&args[1]).unwrap_or_else(|e| {
        eprintln!(
            "{}",
            serde_json::json!({"code":"E_IO","message":e.to_string()})
        );
        process::exit(1)
    });
    if args.len() == 4 {
        let units = module_files::load(std::path::Path::new(&args[3]), source.len())
            .unwrap_or_else(|e| {
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
            "values" => {
                let output = linked.specialize().unwrap_or_else(module_fail);
                println!(
                    "{}",
                    serde_json::json!({"values":output.values,"specialization_fingerprint":output.fingerprint().unwrap()})
                );
            }
            "ast" => println!("{}", serde_json::to_string_pretty(linked.ast()).unwrap()),
            "describe" => println!(
                "{}",
                serde_json::to_string_pretty(&linked.schemas()).unwrap()
            ),
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
    if args[0] == "values" {
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
