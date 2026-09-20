use weave_contract::{AssertionRef, GraphRef, NodeRef, StructuralRef};
use weave_language::{
    bytes::{Bytes, BytesError, MAX_BYTES},
    compile_artifacts,
    references::*,
    scalars::ScalarValue,
};
#[test]
fn bytes_private_bounds_canonical_hex_and_all_octets() {
    let bytes = Bytes::new((0..=255).collect()).unwrap();
    assert_eq!(Bytes::from_hex(&bytes.to_hex()).unwrap(), bytes);
    assert_eq!(
        serde_json::from_str::<Bytes>(&serde_json::to_string(&bytes).unwrap()).unwrap(),
        bytes
    );
    assert_eq!(Bytes::from_hex("00fF").unwrap().as_slice(), [0, 255]);
    assert!(Bytes::from_hex("").unwrap().is_empty());
    for bad in ["f", "0g", "00 ff", "é", "0x00"] {
        assert_eq!(Bytes::from_hex(bad), Err(BytesError::Literal));
    }
    let max = Bytes::new(vec![255; MAX_BYTES]).unwrap();
    assert_eq!(max.len(), MAX_BYTES);
    assert_eq!(
        max.concat(&Bytes::from_hex("01").unwrap()),
        Err(BytesError::Budget)
    );
    assert_eq!(Bytes::new(vec![0; MAX_BYTES + 1]), Err(BytesError::Budget));
    assert_eq!(
        Bytes::from_hex(&"00".repeat(MAX_BYTES + 1)),
        Err(BytesError::Budget)
    );
    assert!(serde_json::from_str::<Bytes>("[0,255]").is_err());
}
#[test]
fn checked_reference_payloads_preserve_kind_pins_and_strict_serde() {
    let raw = NodeRef {
        graph_id: "G".into(),
        revision: "r".into(),
        node_id: "n".into(),
    };
    let n = NodeReference::new(raw.clone()).unwrap();
    assert_eq!(n.reference(), &raw);
    let e = EdgeReference::new(StructuralRef {
        graph_id: "G".into(),
        revision: "r".into(),
        edge_id: "n".into(),
    })
    .unwrap();
    assert_ne!(ObjectReference::Node(n.clone()), ObjectReference::Edge(e));
    let a = AssertionReference::new(AssertionRef {
        graph_id: "G".into(),
        revision: "r".into(),
        assertion_id: "n".into(),
    })
    .unwrap();
    let s = SnapshotReference::new(GraphRef {
        graph_id: "G".into(),
        revision: "r".into(),
    })
    .unwrap();
    for object in [
        ObjectReference::Node(n),
        ObjectReference::Assertion(a),
        ObjectReference::Snapshot(s),
    ] {
        assert_eq!(
            serde_json::from_str::<ObjectReference>(&serde_json::to_string(&object).unwrap())
                .unwrap(),
            object
        );
    }
    for field in ["graph_id", "revision", "node_id"] {
        for bad in ["".to_string(), "λ".repeat(257)] {
            let mut value = serde_json::to_value(&raw).unwrap();
            value[field] = bad.into();
            assert!(serde_json::from_value::<NodeReference>(value).is_err());
        }
    }
    let mut boundary = raw.clone();
    boundary.node_id = "λ".repeat(256);
    assert!(NodeReference::new(boundary).is_ok());
    for bad in [
        r#"{"graph_id":"G","revision":"r","node_id":"n","node_id":"x"}"#,
        r#"{"graph_id":"G","revision":"r","node_id":"n","extra":0}"#,
        r#"{"graph_id":"G","node_id":"n"}"#,
    ] {
        assert!(serde_json::from_str::<NodeReference>(bad).is_err());
        for wrapped in [
            format!(r#"{{"kind":"node","reference":{bad}}}"#),
            format!(r#"{{"reference":{bad},"kind":"node"}}"#),
        ] {
            assert!(
                serde_json::from_str::<ObjectReference>(&wrapped).is_err(),
                "{wrapped}"
            );
        }
    }
    for bad in [
        r#"{"kind":"node","kind":"edge","reference":{"graph_id":"G","revision":"r","node_id":"n"}}"#,
        r#"{"kind":"node","reference":{"graph_id":"G","revision":"r","node_id":"n"},"extra":1}"#,
    ] {
        assert!(serde_json::from_str::<ObjectReference>(bad).is_err());
    }
}
#[test]
fn source_values_partial_callbacks_and_literal_lowering_have_no_reads() {
    let source = include_str!("../examples/bytes_references.weave");
    let out = compile_artifacts(source).unwrap();
    assert_eq!(
        out.values["Payload"],
        ScalarValue::Bytes(Bytes::from_hex("0041ff").unwrap())
    );
    assert_eq!(out.values["Length"], ScalarValue::Integer(3));
    assert_eq!(out.values["Same"], ScalarValue::Boolean(true));
    assert_eq!(out.values["KindsDiffer"], ScalarValue::Boolean(false));
    let weave_contract::Command::Commit { data, .. } = &out.program.commands[0] else {
        panic!()
    };
    assert_eq!(data.attachments.len(), 6);
    for attachment in &data.attachments {
        assert!(matches!(
            attachment.value,
            weave_contract::MetadataValue::Literal { .. }
        ));
        assert!(
            attachment.derived_from.is_empty()
                && attachment.derived_nodes.is_empty()
                && attachment.derived_snapshots.is_empty()
        );
    }
    let source = r#"function Keep revision "1" (node_ref input) returns node_ref {return param input;}
function Through revision "1" (node_ref input,function f(node_ref input) returns node_ref) returns node_ref {apply R from f {node_ref input param input;} return value R;}
apply With from Through {function f Keep;} apply N from With {node_ref input node_ref("G","r","n");}"#;
    let out = compile_artifacts(source).unwrap();
    assert!(out.program.commands.is_empty());
    assert_eq!(out.values["N"].json()["reference"]["revision"], "r");
    let formatted = weave_language::format_source(source).unwrap();
    assert_eq!(
        weave_language::format_source(&formatted).unwrap(),
        formatted
    );
    assert_eq!(
        compile_artifacts(&formatted)
            .unwrap()
            .fingerprint()
            .unwrap(),
        out.fingerprint().unwrap()
    );
    assert_eq!(
        compile_artifacts("value B bytes \"00FF\";")
            .unwrap()
            .fingerprint()
            .unwrap(),
        compile_artifacts("value B bytes \"00ff\";")
            .unwrap()
            .fingerprint()
            .unwrap()
    );
}
#[test]
fn nominal_mismatches_closed_errors_and_schema_erasure_reject() {
    for source in [
        "value X reference_equal(node_ref(\"G\",\"r\",\"x\"),edge_ref(\"G\",\"r\",\"x\"));",
        "function F revision \"1\" (node_ref input) returns node_ref {return param input;} apply X from F {object_ref input object_ref(node_ref(\"G\",\"r\",\"n\"));}",
        "value B bytes \"ff\"; schema S revision \"1\" {node N space \"s\" {property \"v\" string required;}} graph G schema S {node \"n\" type N entity \"n\" space \"s\" property \"v\" value B;}",
    ] {
        let code = if source.starts_with("function F") {
            "E_PARAMETER_TYPE"
        } else {
            "E_SCALAR_TYPE"
        };
        assert_eq!(
            compile_artifacts(source).unwrap_err().code,
            code,
            "{source}"
        );
    }
    assert_eq!(
        compile_artifacts(
            "function Bad revision \"1\" () returns node_ref {return node_ref(\"G\",\"\",\"n\");}"
        )
        .unwrap_err()
        .code,
        "E_REFERENCE_VALUE"
    );
    assert!(compile_artifacts("function Symbolic revision \"1\" (string revision) returns node_ref {return node_ref(\"G\",param revision,\"n\");}").is_ok());
    assert_eq!(
        compile_artifacts("function Bad revision \"1\" () returns bytes {return bytes \"xy\";}")
            .unwrap_err()
            .code,
        "E_BYTES_LITERAL"
    );
}
#[test]
fn pinned_module_references_are_data_and_original_error_spans_survive() {
    use weave_language::modules::{SourceModule, content_digest, link};
    let module = "module \"refs\" revision \"1\"; function Make revision \"1\" (string revision) returns node_ref {return node_ref(\"a::Graph\",param revision,\"a::Node\");}";
    let entry = format!(
        "import a module \"refs\" revision \"1\" sha256 {:?}; apply R from a::Make {{string revision \"a::revision\";}}",
        content_digest(module)
    );
    let modules = [SourceModule {
        id: "refs",
        revision: "1",
        source: module,
    }];
    let linked = link("entry", &entry, &modules)
        .unwrap()
        .compile_artifacts()
        .unwrap();
    assert_eq!(
        linked.values["R"].json()["reference"],
        serde_json::json!({"graph_id":"a::Graph","revision":"a::revision","node_id":"a::Node"})
    );
    let other = entry
        .replace("import a ", "import alias ")
        .replace("from a::Make", "from alias::Make");
    assert_eq!(
        linked.fingerprint().unwrap(),
        link("entry", &other, &modules)
            .unwrap()
            .compile_artifacts()
            .unwrap()
            .fingerprint()
            .unwrap()
    );
}

#[test]
fn byte_work_and_materialization_are_bounded_and_constructor_errors_keep_spans() {
    let too_large = format!("value B bytes {:?};", "00".repeat(MAX_BYTES + 1));
    assert_eq!(
        compile_artifacts(&too_large).unwrap_err().code,
        "E_BYTES_BUDGET"
    );
    let base = format!("value B bytes {:?};", "00".repeat(8192));
    let mut repeated = base;
    for i in 0..150 {
        repeated.push_str(&format!("value Equal{i} bytes_equal(value B,value B);"));
    }
    assert_eq!(
        compile_artifacts(&repeated).unwrap_err().code,
        "E_SCALAR_BUDGET"
    );
    let source = "value Wrong node_ref(\"G\",\"\",\"n\");";
    let error = compile_artifacts(source).unwrap_err();
    assert_eq!(
        &source[error.start..error.end],
        "node_ref(\"G\",\"\",\"n\")"
    );
}

#[test]
fn transitive_reference_functions_and_unused_bad_module_keep_source_attribution() {
    use weave_language::modules::{SourceModule, content_digest, link};
    let core = "module \"core\" revision \"1\"; function Pin revision \"1\" (string revision) returns node_ref {return node_ref(\"G\",param revision,\"n\");}";
    let outer = format!(
        "module \"outer\" revision \"1\"; import c module \"core\" revision \"1\" sha256 {:?}; function F revision \"1\" (string revision) returns node_ref {{apply P from c::Pin {{string revision param revision;}} return value P;}}",
        content_digest(core)
    );
    let entry = format!(
        "import o module \"outer\" revision \"1\" sha256 {:?}; apply N from o::F {{string revision \"r\";}}",
        content_digest(&outer)
    );
    let result = link(
        "entry",
        &entry,
        &[
            SourceModule {
                id: "core",
                revision: "1",
                source: core,
            },
            SourceModule {
                id: "outer",
                revision: "1",
                source: &outer,
            },
        ],
    )
    .unwrap()
    .compile_artifacts()
    .unwrap();
    assert!(result.program.commands.is_empty());
    assert_eq!(result.values["N"].json()["reference"]["revision"], "r");
    let bad = "module \"bad\" revision \"1\"; function Bad revision \"1\" () returns node_ref {return node_ref(\"G\",\"\",\"n\");}";
    let entry = format!(
        "import b module \"bad\" revision \"1\" sha256 {:?};",
        content_digest(bad)
    );
    let error = link(
        "entry",
        &entry,
        &[SourceModule {
            id: "bad",
            revision: "1",
            source: bad,
        }],
    )
    .unwrap()
    .compile_artifacts()
    .unwrap_err();
    assert_ne!(error.source_id, "entry");
    assert_eq!(error.code, "E_REFERENCE_VALUE");
    assert_eq!(&bad[error.start..error.end], "node_ref(\"G\",\"\",\"n\")");
}

#[test]
fn exact_reference_identity_and_higher_order_byte_arguments_never_coerce() {
    let source = r#"value N node_ref("G","r","n");
value OtherRevision reference_equal(value N,node_ref("G","r2","n"));
value OtherGraph reference_equal(value N,node_ref("G2","r","n"));
value OtherObject reference_equal(value N,node_ref("G","r","n2"));
value OtherKind reference_equal(object_ref(value N),object_ref(assertion_ref("G","r","n")));
function Keep revision "1" (bytes input) returns bytes {return param input;}
function Through revision "1" (bytes input,function callback(bytes input) returns bytes) returns bytes {apply R from callback {bytes input param input;} return value R;}
apply F from Through {function callback Keep;} apply R from F {bytes input bytes "00ff";}
"#;
    let out = compile_artifacts(source).unwrap();
    assert!(out.program.commands.is_empty());
    for key in ["OtherRevision", "OtherGraph", "OtherObject", "OtherKind"] {
        assert_eq!(out.values[key], ScalarValue::Boolean(false));
    }
    assert_eq!(
        out.values["R"],
        ScalarValue::Bytes(Bytes::from_hex("00ff").unwrap())
    );
    let bad = source.replace(
        "function callback(bytes input) returns bytes",
        "function callback(node_ref input) returns node_ref",
    );
    assert!(compile_artifacts(&bad).is_err());
}

#[test]
fn empty_byte_source_literal_is_a_value_not_an_identifier() {
    let out=compile_artifacts(r#"value Empty bytes ""; value Length bytes_len(value Empty); value Equal bytes_equal(value Empty,bytes ""); value Joined bytes_concat(bytes "",bytes "ff");"#).unwrap();
    assert_eq!(
        out.values["Empty"],
        ScalarValue::Bytes(Bytes::new(vec![]).unwrap())
    );
    assert_eq!(out.values["Length"], ScalarValue::Integer(0));
    assert_eq!(out.values["Equal"], ScalarValue::Boolean(true));
    assert_eq!(
        out.values["Joined"],
        ScalarValue::Bytes(Bytes::from_hex("ff").unwrap())
    );
    assert_eq!(
        compile_artifacts(r#"graph G {node "" entity "e" space "s";}"#)
            .unwrap_err()
            .code,
        "E_ID"
    );
}
