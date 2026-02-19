use oxigraph::model::{Graph, NamedNodeRef};
use shacl_rust::core::target::Target;
use shacl_rust::rdf::read_graph_from_string;

/// Helper function to create a comprehensive test graph
fn setup_graph() -> Graph {
    let graph_string = r#"
        @prefix ex: <http://example.org/> .

        ex:Alice a ex:Person .
        ex:Bob a ex:Person .
        ex:Charlie a ex:Person .
        ex:CompanyX a ex:Organization .
        ex:CompanyY a ex:Organization .
        ex:David a ex:Person .

        ex:Alice ex:worksAt ex:CompanyX .
        ex:Bob ex:worksAt ex:CompanyY .
        ex:Charlie ex:worksAt ex:CompanyX .

        ex:Alice ex:name "Alice Smith" .
        ex:Bob ex:name "Bob Jones" .

        ex:Alice ex:knows ex:Bob .
        ex:Bob ex:knows ex:David .
        ex:Charlie ex:knows ex:David .

        ex:Alice ex:relation "Alice Smith" .
        ex:Alice ex:relation ex:Bob .
        ex:Bob ex:relation ex:CompanyX .

        _:blank1 a ex:Person .
        _:blank2 a ex:Person .
        ex:Alice ex:knows _:blank1 .

        _:blank3 ex:type "value" .
        ex:Alice ex:type "value" .

        ex:Alice ex:friend _:blank4 .
        ex:Bob ex:friend _:blank5 .
    "#;

    read_graph_from_string(graph_string, "turtle").expect("Failed to read graph")
}

#[test]
fn test_target_node() {
    let graph = setup_graph();
    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let target = Target::Node(alice.into());

    let result = target.resolve_target_for_given_graph(&graph);

    assert_eq!(result.len(), 1);
    assert!(result.contains(&alice.into()));
}

#[test]
fn test_target_class_single_instance() {
    let graph = setup_graph();
    let organization = NamedNodeRef::new("http://example.org/Organization").unwrap();
    let target = Target::Class(organization.into());

    let result = target.resolve_target_for_given_graph(&graph);

    // CompanyX and CompanyY are organizations
    assert_eq!(result.len(), 2);

    let company_x = NamedNodeRef::new("http://example.org/CompanyX").unwrap();
    let company_y = NamedNodeRef::new("http://example.org/CompanyY").unwrap();

    assert!(result.contains(&company_x.into()));
    assert!(result.contains(&company_y.into()));
}

#[test]
fn test_target_class_multiple_instances() {
    let graph = setup_graph();
    let person = NamedNodeRef::new("http://example.org/Person").unwrap();
    let target = Target::Class(person.into());

    let result = target.resolve_target_for_given_graph(&graph);

    // Alice, Bob, Charlie, David, plus 2 blank nodes = 6 persons
    assert_eq!(result.len(), 6);

    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let bob = NamedNodeRef::new("http://example.org/Bob").unwrap();
    let charlie = NamedNodeRef::new("http://example.org/Charlie").unwrap();
    let david = NamedNodeRef::new("http://example.org/David").unwrap();

    assert!(result.contains(&alice.into()));
    assert!(result.contains(&bob.into()));
    assert!(result.contains(&charlie.into()));
    assert!(result.contains(&david.into()));
}

#[test]
fn test_target_class_no_instances() {
    let graph = setup_graph();
    let animal = NamedNodeRef::new("http://example.org/Animal").unwrap();
    let target = Target::Class(animal.into());

    let result = target.resolve_target_for_given_graph(&graph);

    assert_eq!(result.len(), 0);
}

#[test]
fn test_target_subjects_of() {
    let graph = setup_graph();
    let works_at = NamedNodeRef::new("http://example.org/worksAt").unwrap();
    let target = Target::SubjectsOf(works_at);

    let result = target.resolve_target_for_given_graph(&graph);

    // Alice, Bob, and Charlie work at companies
    assert_eq!(result.len(), 3);

    let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
    let bob = NamedNodeRef::new("http://example.org/Bob").unwrap();
    let charlie = NamedNodeRef::new("http://example.org/Charlie").unwrap();

    assert!(result.contains(&alice.into()));
    assert!(result.contains(&bob.into()));
    assert!(result.contains(&charlie.into()));
}

#[test]
fn test_target_subjects_of_no_matches() {
    let graph = setup_graph();
    let manages = NamedNodeRef::new("http://example.org/manages").unwrap();
    let target = Target::SubjectsOf(manages);

    let result = target.resolve_target_for_given_graph(&graph);

    assert_eq!(result.len(), 0);
}

#[test]
fn test_target_objects_of() {
    let graph = setup_graph();
    let works_at = NamedNodeRef::new("http://example.org/worksAt").unwrap();
    let target = Target::ObjectsOf(works_at);

    let result = target.resolve_target_for_given_graph(&graph);

    // CompanyX and CompanyY (Alice and Charlie work at X, Bob works at Y)
    assert_eq!(result.len(), 2);

    let company_x = NamedNodeRef::new("http://example.org/CompanyX").unwrap();
    let company_y = NamedNodeRef::new("http://example.org/CompanyY").unwrap();

    assert!(result.contains(&company_x.into()));
    assert!(result.contains(&company_y.into()));
}

#[test]
fn test_target_objects_of_filters_literals() {
    let graph = setup_graph();
    let name = NamedNodeRef::new("http://example.org/name").unwrap();
    let target = Target::ObjectsOf(name);

    let result = target.resolve_target_for_given_graph(&graph);

    // Literals should be filtered out
    assert_eq!(result.len(), 0);
}

#[test]
fn test_target_objects_of_mixed_objects() {
    let graph = setup_graph();
    let relation = NamedNodeRef::new("http://example.org/relation").unwrap();
    let target = Target::ObjectsOf(relation);

    let result = target.resolve_target_for_given_graph(&graph);

    // Should only include Bob and CompanyX (not the literal "Alice Smith")
    assert_eq!(result.len(), 2);

    let bob = NamedNodeRef::new("http://example.org/Bob").unwrap();
    let company_x = NamedNodeRef::new("http://example.org/CompanyX").unwrap();

    assert!(result.contains(&bob.into()));
    assert!(result.contains(&company_x.into()));
}

#[test]
fn test_target_objects_of_no_matches() {
    let graph = setup_graph();
    let manages = NamedNodeRef::new("http://example.org/manages").unwrap();
    let target = Target::ObjectsOf(manages);

    let result = target.resolve_target_for_given_graph(&graph);

    assert_eq!(result.len(), 0);
}

#[test]
fn test_target_with_blank_nodes() {
    let graph = setup_graph();
    let person = NamedNodeRef::new("http://example.org/Person").unwrap();
    let target = Target::Class(person.into());

    let result = target.resolve_target_for_given_graph(&graph);

    // Should find Alice, Bob, Charlie, David, and 2 blank nodes
    assert_eq!(result.len(), 6);
}

#[test]
fn test_target_subjects_of_with_blank_nodes() {
    let graph = setup_graph();
    let type_pred = NamedNodeRef::new("http://example.org/type").unwrap();
    let target = Target::SubjectsOf(type_pred);

    let result = target.resolve_target_for_given_graph(&graph);

    // Should find both the blank node (_:blank3) and Alice
    assert_eq!(result.len(), 2);
}

#[test]
fn test_target_objects_of_with_blank_nodes() {
    let graph = setup_graph();
    let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
    let target = Target::ObjectsOf(knows);

    let result = target.resolve_target_for_given_graph(&graph);

    // Should find Bob, David, and _:blank1
    assert_eq!(result.len(), 3);

    let bob = NamedNodeRef::new("http://example.org/Bob").unwrap();
    let david = NamedNodeRef::new("http://example.org/David").unwrap();

    assert!(result.contains(&bob.into()));
    assert!(result.contains(&david.into()));
}

#[test]
fn test_target_objects_of_only_blank_nodes() {
    let graph = setup_graph();
    let friend = NamedNodeRef::new("http://example.org/friend").unwrap();
    let target = Target::ObjectsOf(friend);

    let result = target.resolve_target_for_given_graph(&graph);

    // Should find 2 blank nodes (_:blank4 and _:blank5)
    assert_eq!(result.len(), 2);
}
