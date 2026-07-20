use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::validation::dataset::ValidationDataset;
use shacl_rust::{parse_shapes, validation};

fn validate(data_ttl: &str, shapes_ttl: &str) -> (bool, usize) {
    let data_graph = read_graph_from_string(data_ttl, "turtle").expect("data parse");
    let shapes_graph = read_graph_from_string(shapes_ttl, "turtle").expect("shapes parse");
    let shapes = parse_shapes(&shapes_graph).expect("parse shapes");
    let dataset =
        ValidationDataset::from_graphs(data_graph, shapes_graph.clone()).expect("dataset");
    let report = validation::validate(&dataset, &shapes);
    (*report.get_conforms(), report.get_results().len())
}

const SHAPES: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person ;
        sh:closed true ;
        sh:ignoredProperties (rdf:type ex:comment) ;
        sh:property [ sh:path ex:name ] .
"#;

#[test]
fn ignored_properties_are_allowed_on_closed_shape() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:alice a ex:Person ; ex:name "Alice" ; ex:comment "ok" .
    "#;
    let (conforms, violations) = validate(data, SHAPES);
    assert!(
        conforms,
        "rdf:type and ex:comment are in sh:ignoredProperties, got {violations} violations"
    );
}

#[test]
fn unlisted_property_still_violates_closed_shape() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:alice a ex:Person ; ex:name "Alice" ; ex:hobby "golf" .
    "#;
    let (conforms, violations) = validate(data, SHAPES);
    assert!(!conforms, "ex:hobby is not allowed on the closed shape");
    assert_eq!(violations, 1);
}

#[test]
fn closed_without_ignored_properties_flags_type() {
    let shapes = r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:closed true ;
            sh:property [ sh:path ex:name ] .
    "#;
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:alice a ex:Person ; ex:name "Alice" .
    "#;
    let (conforms, violations) = validate(data, shapes);
    assert!(!conforms, "rdf:type is not ignored and not a property path");
    assert_eq!(violations, 1);
}
