use shacl_rust::diagnostics::{from_report, DiagnosticSeverity};
use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::validation::dataset::ValidationDataset;
use shacl_rust::{parse_shapes, validation};

const SHAPES: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person ;
        sh:property [ sh:path ex:age ; sh:minInclusive 0 ; ] .
"#;
const DATA: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    ex:alice a ex:Person ; ex:age "-3"^^xsd:integer .
"#;

#[test]
fn min_inclusive_violation_derives_v0007() {
    let data = read_graph_from_string(DATA, "turtle").unwrap();
    let shapes_graph = read_graph_from_string(SHAPES, "turtle").unwrap();
    let shapes = parse_shapes(&shapes_graph).unwrap();
    let dataset = ValidationDataset::from_graphs(data, shapes_graph.clone()).unwrap();
    let report = validation::validate(&dataset, &shapes);

    let diags = from_report(&report, &dataset, &shapes);
    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert_eq!(d.code, "V0007");
    assert_eq!(d.severity, DiagnosticSeverity::Error);
    assert_eq!(d.snippets.len(), 2, "data + shapes snippets");
    assert!(d.snippets[0].turtle.contains("age"));
    assert!(d.snippets[0].highlight.contains("-3"));
    assert!(d.snippets[1].turtle.contains("minInclusive"));
    assert!(d.expected.as_deref().unwrap().contains(">="));
    assert!(d.actual.as_deref().unwrap().contains("-3"));
    assert!(d.notes.iter().any(|n| n.contains("targetClass")));
    assert!(d.focus_node.as_deref().unwrap().contains("alice"));
}

#[test]
fn custom_component_falls_back_to_v0000() {
    // sh:sparql with a component-less constraint yields SPARQLConstraintComponent -> V0029;
    // a synthetic result with a non-sh component is unit-covered in registry —
    // here assert the sparql path maps to V0029.
    let shapes = r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
            sh:sparql [ sh:select """SELECT $this WHERE { $this <http://example.org/age> ?a . FILTER(?a < 0) }""" ; ] .
    "#;
    let data = read_graph_from_string(DATA, "turtle").unwrap();
    let sg = read_graph_from_string(shapes, "turtle").unwrap();
    let parsed = parse_shapes(&sg).unwrap();
    let dataset = ValidationDataset::from_graphs(data, sg.clone()).unwrap();
    let report = validation::validate(&dataset, &parsed);
    let diags = from_report(&report, &dataset, &parsed);
    assert_eq!(diags[0].code, "V0029");
}

#[test]
fn datatype_and_range_violations_cross_reference_at_the_same_location() {
    let shapes = r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [
                sh:path ex:age ;
                sh:datatype xsd:integer ;
                sh:minInclusive 0 ;
                sh:maxInclusive 130 ;
            ] .
    "#;
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:alice a ex:Person ; ex:age "old" .
    "#;
    let dg = read_graph_from_string(data, "turtle").unwrap();
    let sg = read_graph_from_string(shapes, "turtle").unwrap();
    let parsed = parse_shapes(&sg).unwrap();
    let dataset = ValidationDataset::from_graphs(dg, sg.clone()).unwrap();
    let report = validation::validate(&dataset, &parsed);

    let diags = from_report(&report, &dataset, &parsed);
    assert_eq!(diags.len(), 3, "{diags:?}");

    let datatype = diags
        .iter()
        .find(|d| d.code == "V0002")
        .expect("datatype violation");
    assert!(
        !datatype
            .notes
            .iter()
            .any(|n| n.contains("also fails sh:datatype")),
        "the datatype diagnostic itself should not cross-reference itself: {:?}",
        datatype.notes
    );

    let comparisons: Vec<_> = diags
        .iter()
        .filter(|d| d.code == "V0007" || d.code == "V0009")
        .collect();
    assert_eq!(comparisons.len(), 2, "{diags:?}");
    for d in comparisons {
        assert!(
            d.notes.iter().any(|n| n.contains("also fails sh:datatype")),
            "{:?} should cross-reference the datatype violation at the same path: {:?}",
            d.code,
            d.notes
        );
    }
}

/// R-2 "Join" fixture: `source_shape` for a violation on an anonymous
/// (blank-node) property shape must be stable across independent parses of
/// the same shapes text - not the raw per-run blank-node label, which
/// differs every time `read_graph_from_string`/`parse_shapes` run, even on
/// byte-identical input. Regression test for the exact symptom reported:
/// `source_shape` rendering as `_:ec119a...` instead of a stable id.
#[test]
fn source_shape_on_anonymous_property_shape_is_stable_across_independent_parses() {
    fn diagnose() -> shacl_rust::diagnostics::Diagnostic {
        let data = read_graph_from_string(DATA, "turtle").unwrap();
        let shapes_graph = read_graph_from_string(SHAPES, "turtle").unwrap();
        let shapes = parse_shapes(&shapes_graph).unwrap();
        let dataset = ValidationDataset::from_graphs(data, shapes_graph.clone()).unwrap();
        let report = validation::validate(&dataset, &shapes);
        from_report(&report, &dataset, &shapes)
            .into_iter()
            .next()
            .unwrap()
    }

    let first = diagnose();
    let second = diagnose();

    let source_shape = first.source_shape.as_deref().unwrap();
    assert!(
        !source_shape.starts_with("_:"),
        "source_shape must not be a raw blank-node label: {source_shape}"
    );
    assert_eq!(
        first.source_shape, second.source_shape,
        "source_shape must be identical across independent parses of the same shapes text"
    );

    // And it must actually match decompose_shapes' own id for that same
    // property shape - the whole point of the "Join" fixture.
    let shapes_graph = read_graph_from_string(SHAPES, "turtle").unwrap();
    let shapes = parse_shapes(&shapes_graph).unwrap();
    let decomposed = shacl_rust::decompose_shapes(&shapes, None, shapes_graph.len());
    let known_ids: Vec<&str> = decomposed["shapes"][0]["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["owner_property_shape"].as_str())
        .collect();
    assert!(
        known_ids.contains(&source_shape),
        "diagnostic source_shape {source_shape:?} not found among decompose_shapes owner_property_shape ids {known_ids:?}"
    );
}

#[test]
fn range_violation_alone_does_not_cross_reference_a_datatype_note() {
    // Regression guard: min_inclusive_violation_derives_v0007 above already
    // covers a range violation with NO coexisting datatype violation (age is
    // correctly typed xsd:integer); assert here explicitly that no spurious
    // cross-reference note is added when there is nothing to reference.
    let data = read_graph_from_string(DATA, "turtle").unwrap();
    let shapes_graph = read_graph_from_string(SHAPES, "turtle").unwrap();
    let shapes = parse_shapes(&shapes_graph).unwrap();
    let dataset = ValidationDataset::from_graphs(data, shapes_graph.clone()).unwrap();
    let report = validation::validate(&dataset, &shapes);

    let diags = from_report(&report, &dataset, &shapes);
    assert_eq!(diags.len(), 1);
    assert!(!diags[0]
        .notes
        .iter()
        .any(|n| n.contains("also fails sh:datatype")));
}
