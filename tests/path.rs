use oxigraph::model::NamedNodeRef;
use shacl_rust::io::read_graph_from_string;
use shacl_rust::path::{Path, PathElement, SimplePathElement};

fn setup_test_graph() -> oxigraph::model::Graph {
    let graph_string = r#"
        @prefix ex: <http://example.org/> .

        ex:Alice ex:knows ex:Bob .
        ex:Bob ex:knows ex:Charlie .
        ex:Bob ex:worksAt ex:CompanyX .
        ex:Charlie ex:knows ex:David .
        ex:David ex:knows ex:Eve .

        ex:Alice ex:friend ex:Frank .
        ex:Frank ex:friend ex:George .

        ex:Alice ex:parent ex:Helen .
        ex:Bob ex:parent ex:Helen .
    "#;
    read_graph_from_string(graph_string, "turtle").expect("Failed to read graph")
}

#[test]
fn test_direct_path() {
    let graph = setup_test_graph();
    let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
    let path = Path::new().add_element(PathElement::Iri(knows));

    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let results = path.resolve_path_for_given_node(&graph, &alice);

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0],
        NamedNodeRef::new("http://example.org/Bob").unwrap().into()
    );
}

#[test]
fn test_inverse_path() {
    let graph = setup_test_graph();
    let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
    let path = Path::new().add_element(PathElement::Inverse(knows));

    let bob = NamedNodeRef::new("http://example.org/Bob").unwrap();
    let results = path.resolve_path_for_given_node(&graph, &bob);

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0],
        NamedNodeRef::new("http://example.org/Alice")
            .unwrap()
            .into()
    );
}

#[test]
fn test_loopback_path() {
    let graph = setup_test_graph();
    let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
    let path = Path::new()
        .add_element(PathElement::Iri(knows))
        .add_element(PathElement::Inverse(knows));

    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let results = path.resolve_path_for_given_node(&graph, &alice);

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0],
        NamedNodeRef::new("http://example.org/Alice")
            .unwrap()
            .into()
    );
}

#[test]
fn test_zero_or_more_path() {
    let graph = setup_test_graph();
    let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
    let path = Path::new().add_element(PathElement::ZeroOrMore(Box::new(SimplePathElement::Iri(
        knows,
    ))));

    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let results = path.resolve_path_for_given_node(&graph, &alice);

    // Should include Alice (zero), Bob (one), Charlie (two), David (three), Eve (four)
    assert_eq!(results.len(), 5);
    assert!(results.contains(
        &NamedNodeRef::new("http://example.org/Alice")
            .unwrap()
            .into()
    ));
    assert!(results.contains(&NamedNodeRef::new("http://example.org/Bob").unwrap().into()));
    assert!(results.contains(
        &NamedNodeRef::new("http://example.org/Charlie")
            .unwrap()
            .into()
    ));
    assert!(results.contains(
        &NamedNodeRef::new("http://example.org/David")
            .unwrap()
            .into()
    ));
    assert!(results.contains(&NamedNodeRef::new("http://example.org/Eve").unwrap().into()));
}

#[test]
fn test_one_or_more_path() {
    let graph = setup_test_graph();
    let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
    let path = Path::new().add_element(PathElement::OneOrMore(Box::new(SimplePathElement::Iri(
        knows,
    ))));

    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let results = path.resolve_path_for_given_node(&graph, &alice);

    // Should NOT include Alice, but Bob, Charlie, David, Eve
    assert_eq!(results.len(), 4);
    assert!(!results.contains(
        &NamedNodeRef::new("http://example.org/Alice")
            .unwrap()
            .into()
    ));
    assert!(results.contains(&NamedNodeRef::new("http://example.org/Bob").unwrap().into()));
    assert!(results.contains(
        &NamedNodeRef::new("http://example.org/Charlie")
            .unwrap()
            .into()
    ));
    assert!(results.contains(
        &NamedNodeRef::new("http://example.org/David")
            .unwrap()
            .into()
    ));
    assert!(results.contains(&NamedNodeRef::new("http://example.org/Eve").unwrap().into()));
}

#[test]
fn test_zero_or_one_path() {
    let graph = setup_test_graph();
    let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
    let path = Path::new().add_element(PathElement::ZeroOrOne(Box::new(SimplePathElement::Iri(
        knows,
    ))));

    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let results = path.resolve_path_for_given_node(&graph, &alice);

    // Should include Alice (zero) and Bob (one), but not Charlie
    assert_eq!(results.len(), 2);
    assert!(results.contains(
        &NamedNodeRef::new("http://example.org/Alice")
            .unwrap()
            .into()
    ));
    assert!(results.contains(&NamedNodeRef::new("http://example.org/Bob").unwrap().into()));
    assert!(!results.contains(
        &NamedNodeRef::new("http://example.org/Charlie")
            .unwrap()
            .into()
    ));
}

#[test]
fn test_alternative_path() {
    let graph = setup_test_graph();
    let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
    let friend = NamedNodeRef::new("http://example.org/friend").unwrap();
    let path = Path::new().add_element(PathElement::Alternative(vec![
        PathElement::Iri(knows),
        PathElement::Iri(friend),
    ]));

    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let results = path.resolve_path_for_given_node(&graph, &alice);

    // Should include both Bob (via knows) and Frank (via friend)
    assert_eq!(results.len(), 2);
    assert!(results.contains(&NamedNodeRef::new("http://example.org/Bob").unwrap().into()));
    assert!(results.contains(
        &NamedNodeRef::new("http://example.org/Frank")
            .unwrap()
            .into()
    ));
}

#[test]
fn test_chained_path() {
    let graph = setup_test_graph();
    let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
    // Two hops: Alice -> Bob -> Charlie
    let path = Path::new()
        .add_element(PathElement::Iri(knows))
        .add_element(PathElement::Iri(knows));

    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let results = path.resolve_path_for_given_node(&graph, &alice);

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0],
        NamedNodeRef::new("http://example.org/Charlie")
            .unwrap()
            .into()
    );
}

#[test]
fn test_complex_path() {
    let graph = setup_test_graph();
    let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
    let works_at = NamedNodeRef::new("http://example.org/worksAt").unwrap();
    let parent = NamedNodeRef::new("http://example.org/parent").unwrap();
    // Complex path: ^worksAt / (knows* | parent)
    let path = Path::new()
        .add_element(PathElement::Inverse(works_at))
        .add_element(PathElement::Alternative(vec![
            PathElement::ZeroOrMore(Box::new(SimplePathElement::Iri(knows))),
            PathElement::Iri(parent),
        ]));

    let company_x = NamedNodeRef::new("http://example.org/CompanyX").unwrap();
    let results = path.resolve_path_for_given_node(&graph, &company_x);
    let bob = NamedNodeRef::new("http://example.org/Bob").unwrap();
    let helen = NamedNodeRef::new("http://example.org/Helen").unwrap();
    let charlie = NamedNodeRef::new("http://example.org/Charlie").unwrap();
    let david = NamedNodeRef::new("http://example.org/David").unwrap();
    let eve = NamedNodeRef::new("http://example.org/Eve").unwrap();
    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();

    assert_eq!(results.len(), 5);
    assert!(results.contains(&bob.into()));
    assert!(results.contains(&helen.into()));
    assert!(results.contains(&charlie.into()));
    assert!(results.contains(&david.into()));
    assert!(results.contains(&eve.into()));
    assert!(!results.contains(&alice.into()));
}

#[test]
fn test_empty_results() {
    let graph = setup_test_graph();
    let unknown = NamedNodeRef::new("http://example.org/unknown").unwrap();
    let path = Path::new().add_element(PathElement::Iri(unknown));

    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let results = path.resolve_path_for_given_node(&graph, &alice);

    assert_eq!(results.len(), 0);
}
