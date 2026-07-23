use shacl_rust::diagnostics::*;

fn sample() -> Diagnostic {
    Diagnostic {
        code: "V0007",
        severity: DiagnosticSeverity::Error,
        title: "value violates sh:minInclusive".into(),
        constraint_component: Some(
            "http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent".into(),
        ),
        snippets: vec![
            Snippet {
                origin: SnippetOrigin::DataGraph,
                turtle: "<http://example.org/alice> <http://example.org/age> \"-3\"^^<http://www.w3.org/2001/XMLSchema#integer> .".into(),
                highlight: "\"-3\"^^<http://www.w3.org/2001/XMLSchema#integer>".into(),
                annotation: "this value is less than the required minimum".into(),
            },
            Snippet {
                origin: SnippetOrigin::ShapesGraph,
                turtle: "[] sh:path <http://example.org/age> ;\n   sh:minInclusive \"0\"^^<http://www.w3.org/2001/XMLSchema#integer> .".into(),
                highlight: "sh:minInclusive \"0\"^^<http://www.w3.org/2001/XMLSchema#integer> .".into(),
                annotation: "constraint declared here".into(),
            },
        ],
        expected: Some("a literal >= \"0\"^^<http://www.w3.org/2001/XMLSchema#integer>".into()),
        actual: Some("\"-3\"^^<http://www.w3.org/2001/XMLSchema#integer>".into()),
        notes: vec!["focus node selected by sh:targetClass <http://example.org/Person>".into()],
        help: Some("change the value to satisfy the bound, or relax sh:minInclusive on the shape".into()),
        focus_node: Some("<http://example.org/alice>".into()),
        source_shape: Some("<http://example.org/PersonShape>".into()),
        path: Some("<http://example.org/age>".into()),
        verdict: None,
    }
}

#[test]
fn text_rendering_matches_golden_file() {
    let rendered = render_text(&[sample()], false);
    let expected = include_str!("fixtures/diagnostics/text_basic.expected");
    assert_eq!(rendered, expected, "\n--- rendered ---\n{rendered}");
}

#[test]
fn ndjson_schema_is_stable() {
    let line = render_ndjson(&[sample()]);
    let v: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
    assert_eq!(v["code"], "V0007");
    assert_eq!(v["severity"], "error");
    assert_eq!(v["snippets"][0]["origin"], "data");
    assert_eq!(v["snippets"][1]["origin"], "shapes");
    assert_eq!(v["verdict"], serde_json::Value::Null);
    assert!(v["constraint_component"]
        .as_str()
        .unwrap()
        .ends_with("MinInclusiveConstraintComponent"));
    assert_eq!(line.matches('\n').count(), 1);
}
