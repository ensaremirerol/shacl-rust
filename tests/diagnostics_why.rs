use oxigraph::model::NamedNodeRef;
use shacl_rust::diagnostics::{explain_conformance, Verdict};
use shacl_rust::parse_shapes;
use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::validation::dataset::ValidationDataset;

const SHAPES: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person ;
        sh:property [ sh:path ex:age ; sh:minInclusive 0 ; ] .
"#;

fn run(data: &str, focus: &str) -> Vec<(Option<Verdict>, String)> {
    let dg = read_graph_from_string(data, "turtle").unwrap();
    let sg = read_graph_from_string(SHAPES, "turtle").unwrap();
    let shapes = parse_shapes(&sg).unwrap();
    let dataset = ValidationDataset::from_graphs(dg, sg.clone()).unwrap();
    let focus_nn = NamedNodeRef::new(focus).unwrap();
    let focus_term = dataset
        .data()
        .canonical_term(focus_nn.into())
        .expect("focus in data");
    // Leak the dataset/shapes borrow scope by asserting inside:
    explain_conformance(&dataset, &shapes, focus_term, None)
        .into_iter()
        .map(|d| (d.verdict, d.title))
        .collect()
}

#[test]
fn violating_node_traces_violates() {
    let data =
        "@prefix ex: <http://example.org/> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:a a ex:Person ; ex:age \"-3\"^^xsd:integer .";
    let out = run(data, "http://example.org/a");
    assert!(
        out.iter().any(|(v, _)| *v == Some(Verdict::Violates)),
        "{out:?}"
    );
}

#[test]
fn conforming_node_traces_conforms() {
    let data =
        "@prefix ex: <http://example.org/> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:a a ex:Person ; ex:age \"3\"^^xsd:integer .";
    let out = run(data, "http://example.org/a");
    assert!(
        out.iter().any(|(v, _)| *v == Some(Verdict::Conforms)),
        "{out:?}"
    );
}

#[test]
fn untargeted_node_traces_not_targeted() {
    let data = "@prefix ex: <http://example.org/> .
        ex:a ex:age 3 .";
    let out = run(data, "http://example.org/a");
    assert!(
        out.iter().any(|(v, _)| *v == Some(Verdict::NotTargeted)),
        "{out:?}"
    );
}

#[test]
fn missing_path_traces_vacuous() {
    let data = "@prefix ex: <http://example.org/> .
        ex:a a ex:Person .";
    let out = run(data, "http://example.org/a");
    assert!(
        out.iter().any(|(v, _)| *v == Some(Verdict::Vacuous)),
        "{out:?}"
    );
}
