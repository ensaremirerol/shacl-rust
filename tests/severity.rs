//! SHACL §2.1.5: a result's severity is the sh:severity of the shape that
//! produced it (default sh:Violation), never inherited from a parent shape.

use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::{parse_shapes, validation};

/// (severity IRI, constraint component local name) per result, sorted.
fn severities(data_ttl: &str, shapes_ttl: &str) -> Vec<(String, String)> {
    let data_graph = read_graph_from_string(data_ttl, "turtle").expect("Failed to read data");
    let shapes_graph = read_graph_from_string(shapes_ttl, "turtle").expect("Failed to read shapes");
    let shapes = parse_shapes(&shapes_graph).expect("Failed to parse shapes");
    let dataset = validation::dataset::ValidationDataset::from_graphs(
        data_graph.clone(),
        shapes_graph.clone(),
    )
    .expect("Failed to create dataset");
    let report = validation::validate(&dataset, &shapes);
    let mut out: Vec<_> = report
        .get_results()
        .iter()
        .map(|r| {
            let component = r.source_constraint_component().expect("component");
            let local = component.as_str().rsplit('#').next().unwrap().to_string();
            (r.severity().as_str().to_string(), local)
        })
        .collect();
    out.sort();
    out
}

const VIOLATION: &str = "http://www.w3.org/ns/shacl#Violation";
const WARNING: &str = "http://www.w3.org/ns/shacl#Warning";
const INFO: &str = "http://www.w3.org/ns/shacl#Info";

#[test]
fn nested_property_shape_does_not_inherit_node_severity() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:CarePlanShape a sh:NodeShape ;
            sh:targetClass ex:CarePlan ;
            sh:severity sh:Warning ;
            sh:property [ sh:path ex:hasProcedure ; sh:minCount 1 ] .
    "#;
    let data = "@prefix ex: <http://example.org/> . ex:plan1 a ex:CarePlan .";
    assert_eq!(
        severities(data, shapes),
        vec![(VIOLATION.into(), "MinCountConstraintComponent".into())]
    );
}

#[test]
fn explicit_property_severity_and_node_level_severity_are_kept() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:CarePlanShape a sh:NodeShape ;
            sh:targetClass ex:CarePlan ;
            sh:severity sh:Info ;
            sh:class ex:Missing ;
            sh:property [ sh:path ex:hasProcedure ; sh:minCount 1 ; sh:severity sh:Warning ] ;
            sh:property [ sh:path ex:hasDate ; sh:minCount 1 ] .
    "#;
    let data = "@prefix ex: <http://example.org/> . ex:plan1 a ex:CarePlan .";
    let mut expected = vec![
        (INFO.to_string(), "ClassConstraintComponent".to_string()),
        (
            WARNING.to_string(),
            "MinCountConstraintComponent".to_string(),
        ),
        (
            VIOLATION.to_string(),
            "MinCountConstraintComponent".to_string(),
        ),
    ];
    expected.sort();
    assert_eq!(severities(data, shapes), expected);
}
