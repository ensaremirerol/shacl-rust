use shacl_rust::diagnostics::shape_target_nodes;
use shacl_rust::parse_shapes;
use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::validation::dataset::ValidationDataset;

const SHAPES: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person .
    ex:OrphanShape a sh:NodeShape .
"#;

const DATA: &str = "@prefix ex: <http://example.org/> .
    ex:alice a ex:Person .
    ex:bob a ex:Person .";

fn run(shapes_ttl: &str, data_ttl: &str) -> Vec<(String, Vec<String>)> {
    let dg = read_graph_from_string(data_ttl, "turtle").unwrap();
    let sg = read_graph_from_string(shapes_ttl, "turtle").unwrap();
    let dataset = ValidationDataset::from_graphs(dg, sg).unwrap();
    let shapes = parse_shapes(dataset.shapes_graph()).unwrap();
    shape_target_nodes(&dataset, &shapes)
}

#[test]
fn lists_all_resolved_targets_conforming_and_violating_alike() {
    let result = run(SHAPES, DATA);

    assert_eq!(
        result.len(),
        1,
        "OrphanShape has no targets and must be omitted: {result:?}"
    );
    let (shape_node, nodes) = &result[0];
    assert_eq!(shape_node, "<http://example.org/PersonShape>");
    assert_eq!(
        nodes,
        &vec![
            "<http://example.org/alice>".to_string(),
            "<http://example.org/bob>".to_string(),
        ]
    );
}

#[test]
fn shape_with_no_targets_is_omitted_even_when_it_is_the_only_shape() {
    let shapes = "@prefix ex: <http://example.org/> . @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:OrphanShape a sh:NodeShape .";
    let result = run(shapes, "");
    assert!(result.is_empty(), "{result:?}");
}
