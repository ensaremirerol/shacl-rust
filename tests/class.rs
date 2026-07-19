use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::{parse_shapes, validation};

fn validate(data_ttl: &str, shapes_ttl: &str) -> (bool, usize) {
    let data_graph = read_graph_from_string(data_ttl, "turtle").expect("Failed to read data");
    let shapes_graph = read_graph_from_string(shapes_ttl, "turtle").expect("Failed to read shapes");
    let shapes = parse_shapes(&shapes_graph).expect("Failed to parse shapes");
    let dataset =
        validation::dataset::ValidationDataset::from_graphs(data_graph.clone(), shapes_graph.clone())
            .expect("Failed to create dataset");
    let report = validation::validate(&dataset, &shapes);
    (*report.get_conforms(), report.get_results().len())
}

const SHAPES: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .

    ex:OwnerShape a sh:NodeShape ;
        sh:targetClass ex:Owner ;
        sh:property [
            sh:path ex:pet ;
            sh:class ex:Animal ;
        ] .
"#;

#[test]
fn test_class_matches_direct_instance() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:alice a ex:Owner ; ex:pet ex:generic .
        ex:generic a ex:Animal .
    "#;
    let (conforms, violations) = validate(data, SHAPES);
    assert!(conforms, "direct instance of ex:Animal must conform");
    assert_eq!(violations, 0);
}

#[test]
fn test_class_matches_subclass_instance() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Dog rdfs:subClassOf ex:Animal .
        ex:alice a ex:Owner ; ex:pet ex:rex .
        ex:rex a ex:Dog .
    "#;
    let (conforms, violations) = validate(data, SHAPES);
    assert!(
        conforms,
        "instance of ex:Dog (rdfs:subClassOf ex:Animal) must conform to sh:class ex:Animal, got {violations} violations"
    );
}

#[test]
fn test_class_matches_transitive_subclass_instance() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Puppy rdfs:subClassOf ex:Dog .
        ex:Dog rdfs:subClassOf ex:Animal .
        ex:alice a ex:Owner ; ex:pet ex:spot .
        ex:spot a ex:Puppy .
    "#;
    let (conforms, violations) = validate(data, SHAPES);
    assert!(
        conforms,
        "instance of ex:Puppy (transitively subClassOf ex:Animal) must conform, got {violations} violations"
    );
}

#[test]
fn test_class_rejects_unrelated_instance() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Rock rdfs:subClassOf ex:Mineral .
        ex:alice a ex:Owner ; ex:pet ex:pebble .
        ex:pebble a ex:Rock .
    "#;
    let (conforms, violations) = validate(data, SHAPES);
    assert!(!conforms, "instance of an unrelated class must not conform");
    assert_eq!(violations, 1);
}

#[test]
fn test_class_rejects_superclass_instance() {
    // Subclassing is directional: an ex:Animal is not an ex:Dog.
    let shapes = r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:OwnerShape a sh:NodeShape ;
            sh:targetClass ex:Owner ;
            sh:property [ sh:path ex:pet ; sh:class ex:Dog ; ] .
    "#;
    let data = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Dog rdfs:subClassOf ex:Animal .
        ex:alice a ex:Owner ; ex:pet ex:generic .
        ex:generic a ex:Animal .
    "#;
    let (conforms, violations) = validate(data, shapes);
    assert!(
        !conforms,
        "an instance of the superclass must not satisfy sh:class of the subclass"
    );
    assert_eq!(violations, 1);
}
