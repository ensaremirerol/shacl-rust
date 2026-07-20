//! The RDF serialization of validation reports must carry the full
//! sh:resultPath structure for complex paths (sequence as an RDF list,
//! alternative/inverse/quantified paths as their sh: structures), not just
//! the first predicate. Distilled from the shacl-benchmark
//! `resultpath-complex-path` divergence pattern.

use oxigraph::model::{Graph, NamedNodeRef, NamedOrBlankNodeRef, TermRef};
use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::validation::dataset::ValidationDataset;
use shacl_rust::{parse_shapes, validation};

const SH_RESULT_PATH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#resultPath");
const SH_INVERSE_PATH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#inversePath");
const SH_ALTERNATIVE_PATH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#alternativePath");
const RDF_FIRST: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#first");
const RDF_REST: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest");
const RDF_NIL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil");

fn report_graph(shapes_ttl: &str, data_ttl: &str) -> Graph {
    let data_graph = read_graph_from_string(data_ttl, "turtle").expect("data parse");
    let shapes_graph = read_graph_from_string(shapes_ttl, "turtle").expect("shapes parse");
    let shapes = parse_shapes(&shapes_graph).expect("parse shapes");
    let dataset =
        ValidationDataset::from_graphs(data_graph, shapes_graph.clone()).expect("dataset");
    let report = validation::validate(&dataset, &shapes);
    assert!(!report.get_conforms(), "fixture must produce a violation");
    report.to_graph()
}

fn sole_result_path(graph: &Graph) -> TermRef<'_> {
    let mut paths = graph.triples_for_predicate(SH_RESULT_PATH);
    let path = paths
        .next()
        .expect("report must carry sh:resultPath")
        .object;
    assert!(paths.next().is_none(), "expected exactly one result");
    path
}

fn list_items<'a>(graph: &'a Graph, head: TermRef<'a>) -> Vec<TermRef<'a>> {
    let mut items = Vec::new();
    let mut current = head;
    loop {
        if let TermRef::NamedNode(nn) = current {
            if nn == RDF_NIL {
                return items;
            }
        }
        let node: NamedOrBlankNodeRef = match current {
            TermRef::NamedNode(nn) => nn.into(),
            TermRef::BlankNode(bn) => bn.into(),
            TermRef::Literal(_) => panic!("literal in list"),
        };
        items.push(
            graph
                .object_for_subject_predicate(node, RDF_FIRST)
                .expect("rdf:first"),
        );
        current = graph
            .object_for_subject_predicate(node, RDF_REST)
            .expect("rdf:rest");
    }
}

#[test]
fn sequence_path_serializes_as_rdf_list() {
    let graph = report_graph(
        r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:S a sh:NodeShape ;
            sh:targetNode ex:i ;
            sh:property [ sh:path ( ex:p1 ex:p2 ) ; sh:minCount 1 ; ] .
        "#,
        r#"
        @prefix ex: <http://example.org/> .
        ex:i ex:unrelated ex:j .
        "#,
    );
    let path = sole_result_path(&graph);
    let items = list_items(&graph, path);
    let rendered: Vec<String> = items.iter().map(|t| t.to_string()).collect();
    assert_eq!(
        rendered,
        vec![
            "<http://example.org/p1>".to_string(),
            "<http://example.org/p2>".to_string()
        ],
        "sequence path must serialize as the full RDF list"
    );
}

#[test]
fn alternative_path_serializes_with_sh_alternative_path() {
    let graph = report_graph(
        r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:S a sh:NodeShape ;
            sh:targetNode ex:i ;
            sh:property [ sh:path [ sh:alternativePath ( ex:p1 ex:p2 ) ] ; sh:minCount 1 ; ] .
        "#,
        r#"
        @prefix ex: <http://example.org/> .
        ex:i ex:unrelated ex:j .
        "#,
    );
    let path = sole_result_path(&graph);
    let node: NamedOrBlankNodeRef = match path {
        TermRef::BlankNode(bn) => bn.into(),
        other => panic!("alternative path must be a blank node, got {other}"),
    };
    let alts_head = graph
        .object_for_subject_predicate(node, SH_ALTERNATIVE_PATH)
        .expect("sh:alternativePath");
    let rendered: Vec<String> = list_items(&graph, alts_head)
        .iter()
        .map(|t| t.to_string())
        .collect();
    assert_eq!(
        rendered,
        vec![
            "<http://example.org/p1>".to_string(),
            "<http://example.org/p2>".to_string()
        ]
    );
}

#[test]
fn inverse_path_serializes_with_sh_inverse_path() {
    let graph = report_graph(
        r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:S a sh:NodeShape ;
            sh:targetNode ex:i ;
            sh:property [ sh:path [ sh:inversePath ex:p1 ] ; sh:minCount 1 ; ] .
        "#,
        r#"
        @prefix ex: <http://example.org/> .
        ex:i ex:unrelated ex:j .
        "#,
    );
    let path = sole_result_path(&graph);
    let node: NamedOrBlankNodeRef = match path {
        TermRef::BlankNode(bn) => bn.into(),
        other => panic!("inverse path must be a blank node, got {other}"),
    };
    let inner = graph
        .object_for_subject_predicate(node, SH_INVERSE_PATH)
        .expect("sh:inversePath");
    assert_eq!(inner.to_string(), "<http://example.org/p1>");
}

#[test]
fn simple_path_serializes_as_plain_iri() {
    let graph = report_graph(
        r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:S a sh:NodeShape ;
            sh:targetNode ex:i ;
            sh:property [ sh:path ex:p1 ; sh:minCount 1 ; ] .
        "#,
        r#"
        @prefix ex: <http://example.org/> .
        ex:i ex:unrelated ex:j .
        "#,
    );
    let path = sole_result_path(&graph);
    assert_eq!(path.to_string(), "<http://example.org/p1>");
}
