use std::{env, fs, io::Read, process};
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 2 || !["check", "plan", "ast", "describe"].contains(&args[0].as_str()) {
        eprintln!(
            "Usage: weave <check|plan|ast|describe> FILE.weave\nplan emits Weave Engine protocol JSON; host authorization belongs to the engine."
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
    if args[0] == "describe" {
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
