use weave_language::{
    compile_artifacts,
    scalars::ScalarValue,
    vectors::{CheckedVector, Geometry, Space, Unit, VectorDescriptor, VectorRole},
};
const PHYSICAL: &str = r#"vector_type P space {"id":"physical","revision":"1","geometry":{"kind":"physical3d","frame":"building","unit":"metre"}} role position;"#;
fn descriptor() -> VectorDescriptor {
    VectorDescriptor::new(
        Space {
            id: "physical".into(),
            revision: "1".into(),
            geometry: Geometry::Physical3d {
                frame: "building".into(),
                unit: Unit::Metre,
            },
        },
        VectorRole::Position,
    )
    .unwrap()
}
#[test]
fn checked_rust_and_serde_paths_preserve_eq_finiteness_and_strict_fields() {
    fn requires_eq<T: Eq>() {}
    requires_eq::<ScalarValue>();
    requires_eq::<CheckedVector>();
    assert!(CheckedVector::new(descriptor(), vec![f64::NAN, 0.0, 0.0]).is_err());
    assert!(CheckedVector::new(descriptor(), vec![f64::INFINITY, 0.0, 0.0]).is_err());
    assert!(CheckedVector::new(descriptor(), vec![0.0; 4]).is_err());
    let a = CheckedVector::new(descriptor(), vec![-0.0, 0.0, f64::MAX]).unwrap();
    let b = CheckedVector::new(descriptor(), vec![0.0, 0.0, f64::MAX]).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.values()[0].to_bits(), 0.0f64.to_bits());
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
    let raw = serde_json::to_string(&a).unwrap();
    assert_eq!(serde_json::from_str::<CheckedVector>(&raw).unwrap(), a);
    for bad in [
        raw.replace(
            "\"kind\":\"physical3d\"",
            "\"kind\":\"physical3d\",\"kind\":\"physical3d\"",
        ),
        raw.replace(
            "\"revision\":\"1\"",
            "\"revision\":\"1\",\"revision\":\"1\"",
        ),
        raw.replace(
            "\"role\":\"position\"",
            "\"role\":\"position\",\"role\":\"position\"",
        ),
        raw.replace("\"unit\":\"metre\"", "\"unit\":\"metre\",\"unknown\":true"),
        raw.replace("\"position\"", "\"embedding\""),
    ] {
        assert!(
            serde_json::from_str::<CheckedVector>(&bad).is_err(),
            "{bad}"
        );
        assert!(
            serde_json::from_str::<ScalarValue>(&format!(
                "{{\"type\":\"vector\",\"value\":{bad}}}"
            ))
            .is_err()
        );
    }
    let oversized = raw.replace(
        "[0.0,0.0,1.7976931348623157e+308]",
        &format!("[{}]", vec!["0"; 4097].join(",")),
    );
    assert_ne!(oversized, raw);
    assert!(
        serde_json::from_str::<CheckedVector>(&oversized)
            .unwrap_err()
            .to_string()
            .contains("E_VECTOR_BUDGET")
    );
}
#[test]
fn source_values_callbacks_and_aliases_keep_nominal_types() {
    let source = format!(
        r#"{PHYSICAL}
function Keep revision "1" (vector input type P) returns vector type P {{return param input;}}
function Through revision "1" (vector input type P,function callback(vector input type P) returns vector type P) returns vector type P {{apply R from callback {{vector input param input;}} return value R;}}
apply Via from Through {{function callback Keep;}}
apply Origin from Via {{vector input vector P [-0.0,0.0,0.0];}}
value Equal vector_equal(value Origin,vector P [0.0,0.0,0.0]);
graph G {{node "n" entity "n" space "physical" property "coordinates" value Origin; attachment "a" on graph key "coordinates" literal value Origin valid 0 until infinity;}}
"#
    );
    let result = compile_artifacts(&source).unwrap();
    assert_eq!(result.values["Equal"], ScalarValue::Boolean(true));
    assert_eq!(result.values["Origin"].json()["kind"], "coordinates");
    let renamed = source
        .replace("type P", "type Renamed")
        .replace("vector P", "vector Renamed");
    assert_eq!(
        result.fingerprint().unwrap(),
        compile_artifacts(&renamed).unwrap().fingerprint().unwrap()
    );
    let formatted = weave_language::format_source(&source).unwrap();
    assert_eq!(
        result.fingerprint().unwrap(),
        compile_artifacts(&formatted)
            .unwrap()
            .fingerprint()
            .unwrap()
    );
    assert_eq!(
        weave_language::format_source(&formatted).unwrap(),
        formatted
    );
}
#[test]
fn invalid_or_ambiguous_source_descriptors_and_unused_values_fail() {
    for source in [
        PHYSICAL.replace(
            "\"kind\":\"physical3d\"",
            "\"kind\":\"physical3d\",\"kind\":\"physical3d\"",
        ),
        PHYSICAL.replace(
            "\"revision\":\"1\"",
            "\"revision\":\"1\",\"revision\":\"1\"",
        ),
    ] {
        assert_eq!(compile_artifacts(&source).unwrap_err().code, "E_DUPLICATE");
    }
    assert_eq!(compile_artifacts(&format!("{PHYSICAL} function F revision \"1\" (vector input type P) returns vector type P {{return vector P [0.0,0.0];}}")).unwrap_err().code,"E_VECTOR_DIMENSION");
    let conflict = PHYSICAL
        .replace("vector_type P", "vector_type Other")
        .replace("building", "other");
    assert_eq!(
        compile_artifacts(&format!("{PHYSICAL}{conflict}"))
            .unwrap_err()
            .code,
        "E_VECTOR_DESCRIPTOR_CONFLICT"
    );
    assert!(compile_artifacts(&format!("{PHYSICAL} value V vector P [1e400,0.0,0.0];")).is_err());
    for scalar in ["string", "float", "integer"] {
        let source = format!(
            "{PHYSICAL} value V vector P [0,0,0]; schema S revision \"1\" {{node N space \"physical\" {{property \"v\" {scalar} required;}}}} graph G schema S {{node \"n\" type N entity \"n\" space \"physical\" property \"v\" value V;}}"
        );
        assert_eq!(
            compile_artifacts(&source).unwrap_err().code,
            "E_SCALAR_TYPE"
        );
    }
}

#[test]
fn exact_descriptor_matrix_rejects_same_length_but_incompatible_values() {
    let declaration = PHYSICAL.replace("vector_type P", "vector_type Q");
    for (from, to, code) in [
        (
            "\"id\":\"physical\"",
            "\"id\":\"different\"",
            "E_PARAMETER_TYPE",
        ),
        (
            "\"revision\":\"1\"",
            "\"revision\":\"2\"",
            "E_PARAMETER_TYPE",
        ),
        ("building", "other", "E_VECTOR_DESCRIPTOR_CONFLICT"),
        ("metre", "centimetre", "E_VECTOR_DESCRIPTOR_CONFLICT"),
        ("role position", "role direction", "E_PARAMETER_TYPE"),
    ] {
        let source = format!(
            "{PHYSICAL}{} function F revision \"1\" (vector input type P) returns vector type P {{return param input;}} apply Bad from F {{vector input vector Q [0,0,0];}}",
            declaration.replace(from, to)
        );
        assert_eq!(compile_artifacts(&source).unwrap_err().code, code, "{from}");
    }
    let embedding = r#"vector_type E space {"id":"semantic","revision":"1","geometry":{"kind":"embedding","encoder":"v1","preprocessing":"tokens","dimensions":3,"metric":"euclidean"}} role embedding;"#;
    for (from, to) in [
        ("v1", "v2"),
        ("tokens", "other"),
        ("euclidean", "cosine_distance"),
    ] {
        let changed = embedding
            .replace("vector_type E", "vector_type Other")
            .replace(from, to);
        assert_eq!(
            compile_artifacts(&format!("{embedding}{changed}"))
                .unwrap_err()
                .code,
            "E_VECTOR_DESCRIPTOR_CONFLICT"
        );
    }
    let source =
        format!("{PHYSICAL}{embedding} value Bad vector_equal(vector P [0,0,0],vector E [0,0,0]);");
    assert_eq!(
        compile_artifacts(&source).unwrap_err().code,
        "E_SCALAR_TYPE"
    );
}

#[test]
fn pinned_modules_capture_types_transitively_and_keep_original_error_locations() {
    use weave_language::modules::{SourceModule, content_digest, link};
    let core = format!(
        "module \"core\" revision \"1\";{PHYSICAL} function Keep revision \"1\" (vector input type P) returns vector type P {{return param input;}}"
    );
    let middle = format!(
        "module \"middle\" revision \"1\"; import c module \"core\" revision \"1\" sha256 {:?}; function Wrap revision \"1\" (vector input type c::P) returns vector type c::P {{apply R from c::Keep {{vector input param input;}} return value R;}}",
        content_digest(&core)
    );
    let entry = format!(
        "import c module \"core\" revision \"1\" sha256 {:?}; import m module \"middle\" revision \"1\" sha256 {:?}; apply R from m::Wrap {{vector input vector c::P [1,2,3];}}",
        content_digest(&core),
        content_digest(&middle)
    );
    let units = [
        SourceModule {
            id: "core",
            revision: "1",
            source: &core,
        },
        SourceModule {
            id: "middle",
            revision: "1",
            source: &middle,
        },
    ];
    let a = link("entry", &entry, &units)
        .unwrap()
        .compile_artifacts()
        .unwrap();
    let b = link(
        "entry",
        &entry
            .replace("import c ", "import Renamed ")
            .replace("c::", "Renamed::"),
        &units,
    )
    .unwrap()
    .compile_artifacts()
    .unwrap();
    assert_eq!(a.fingerprint().unwrap(), b.fingerprint().unwrap());
    assert!(a.program.commands.is_empty());
    assert_eq!(
        a.program
            .source_revisions
            .iter()
            .filter(|r| r.name == "module:core")
            .count(),
        1
    );
    let bad = "module \"bad\" revision \"1\"; // λ\nfunction F revision \"1\" (vector input type Missing) returns vector type Missing {return param input;}";
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
    assert_eq!(error.source_id, "module:bad@1");
    assert_eq!(&bad[error.start..error.end], "Missing");
    assert_eq!(error.import_trace.len(), 1);
}

#[test]
fn large_literals_and_repeated_work_are_precharged() {
    let declaration = r#"vector_type E space {"id":"semantic","revision":"1","geometry":{"kind":"embedding","encoder":"v1","preprocessing":"tokens","dimensions":4096,"metric":"euclidean"}} role embedding;"#;
    let literal = format!("vector E [{}]", vec!["0"; 4096].join(","));
    let source = format!("{declaration} value V {literal};");
    let result = compile_artifacts(&source).unwrap();
    assert_eq!(
        result.values["V"].json()["values"]
            .as_array()
            .unwrap()
            .len(),
        4096
    );
    let too_many = format!(
        "{declaration} value V vector E [{}];",
        vec!["0"; 4097].join(",")
    );
    assert_eq!(
        compile_artifacts(&too_many).unwrap_err().code,
        "E_VECTOR_DIMENSION"
    );
    let repeated = format!(
        "{source}{}",
        (0..100)
            .map(|i| format!("value C{i} vector_equal(value V,value V);"))
            .collect::<String>()
    );
    assert_eq!(
        compile_artifacts(&repeated).unwrap_err().code,
        "E_SCALAR_BUDGET"
    );
    let capture = format!(
        "{source} function Same revision \"1\" (vector input type E,vector other type E) returns boolean {{return vector_equal(param input,param other);}} {}",
        (0..100)
            .map(|i| format!("apply Partial{i} from Same {{vector input value V;}}"))
            .collect::<String>()
    );
    assert_eq!(
        compile_artifacts(&capture).unwrap_err().code,
        "E_SCALAR_BUDGET"
    );
}

#[test]
fn overlong_vectors_reject_before_visiting_an_excess_nested_subtree() {
    let prefix =
        serde_json::to_string(&CheckedVector::new(descriptor(), vec![0.0; 3]).unwrap()).unwrap();
    let nested = format!("{}0{}", "[".repeat(1024), "]".repeat(1024));
    let bad = prefix.replace(
        "[0.0,0.0,0.0]",
        &format!("[{},{}]", vec!["0"; 4096].join(","), nested),
    );
    assert!(
        serde_json::from_str::<CheckedVector>(&bad)
            .unwrap_err()
            .to_string()
            .contains("E_VECTOR_BUDGET")
    );
    let source = format!(
        "{PHYSICAL} value V vector P [{},{}];",
        vec!["0"; 4096].join(","),
        nested
    );
    assert_eq!(
        compile_artifacts(&source).unwrap_err().code,
        "E_VECTOR_DIMENSION"
    );
}
