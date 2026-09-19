//! Independent linker boundary checks. No resolver authority comes from internal names.
use weave_language::modules::{SourceModule, compile_with_modules, content_digest};

fn import(alias: &str, id: &str, source: &str) -> String {
    format!(
        "import {alias} module {id:?} revision \"1\" sha256 {:?};\n",
        content_digest(source)
    )
}

#[test]
fn root_cannot_address_transitive_exports_by_forging_internal_symbols() {
    let base = "module \"base\" revision \"1\"; function Keep revision \"1\" (graph input) {return input;}";
    let bridge = format!(
        "module \"bridge\" revision \"1\"; {}",
        import("hidden", "base", base)
    );
    let internal = format!("__weave_module_{}_Keep", content_digest("base"));
    let entry = format!(
        "{}graph G {{}} apply Result from {internal} {{graph input G;}}",
        import("b", "bridge", &bridge)
    );
    let modules = [
        SourceModule { id: "base", revision: "1", source: base },
        SourceModule { id: "bridge", revision: "1", source: &bridge },
    ];
    let error = compile_with_modules("entry", &entry, &modules).unwrap_err();
    assert_eq!(error.source_id, "entry");
    assert_eq!(&entry[error.start..error.end], internal);
}

#[test]
fn cached_diamond_subtrees_still_count_toward_longest_import_depth() {
    // Sorted direct imports discover the 15-unit A chain before the Z chain.
    // Z adds three units before the cached A subtree: actual depth is 18.
    let mut units: Vec<(String, String)> = Vec::new();
    let mut next: Option<(String, String)> = None;
    for i in (0..15).rev() {
        let id = format!("A{i:02}");
        let mut source = format!("module {id:?} revision \"1\";\n");
        if let Some((next_id, next_source)) = &next {
            source.push_str(&import("next", next_id, next_source));
        }
        next = Some((id.clone(), source.clone()));
        units.push((id, source));
    }
    let (a_id, a_source) = next.clone().unwrap();
    for i in (0..3).rev() {
        let id = format!("Z{i:02}");
        let (next_id, next_source) = next.as_ref().unwrap();
        let source = format!(
            "module {id:?} revision \"1\";\n{}",
            import("next", next_id, next_source)
        );
        next = Some((id.clone(), source.clone()));
        units.push((id, source));
    }
    let (z_id, z_source) = next.unwrap();
    let entry = import("early", &a_id, &a_source) + &import("late", &z_id, &z_source);
    let modules: Vec<_> = units.iter().map(|(id, source)| SourceModule {
        id, revision: "1", source,
    }).collect();
    assert_eq!(
        compile_with_modules("entry", &entry, &modules).unwrap_err().code,
        "E_MODULE_BUDGET"
    );
}
