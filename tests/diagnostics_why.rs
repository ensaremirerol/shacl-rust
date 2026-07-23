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
    run_with_shapes(SHAPES, data, focus)
}

fn run_with_shapes(shapes: &str, data: &str, focus: &str) -> Vec<(Option<Verdict>, String)> {
    let dg = read_graph_from_string(data, "turtle").unwrap();
    let sg = read_graph_from_string(shapes, "turtle").unwrap();
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

#[test]
fn trace_includes_a_header_entry_for_the_traced_node_shape() {
    let data =
        "@prefix ex: <http://example.org/> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:a a ex:Person ; ex:age \"3\"^^xsd:integer .";
    let out = run(data, "http://example.org/a");
    assert!(
        out.iter().any(|(v, title)| v.is_none()
            && title.contains("NodeShape")
            && title.contains("PersonShape")),
        "{out:?}"
    );
}

/// Regression test for the misattribution bug in `find_matching_result`
/// (`src/diagnostics/explain_pass.rs`): `sh:class ex:Person, ex:Employee` on
/// one property shape parses into two separate `Constraint::Class`
/// instances (one per object of `sh:class` - see
/// `parser/constraints/class.rs`). Before the fix, both traces were matched
/// to the *same* report result purely by (source_shape, component code), so
/// whichever result the validator happened to produce first (the Employee
/// violation) got attributed to *both* the Person and the Employee trace -
/// falsely reporting the conforming Person constraint as `Violates` too.
#[test]
fn same_shape_class_constraints_are_disambiguated_by_class_iri() {
    let shapes = r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:FriendShape a sh:NodeShape ;
            sh:targetNode ex:a ;
            sh:property [ sh:path ex:friend ; sh:class ex:Person, ex:Employee ] .
    "#;
    // ex:b conforms to ex:Person but not ex:Employee.
    let data = "@prefix ex: <http://example.org/> .
        ex:a ex:friend ex:b . ex:b a ex:Person .";
    let out = run_with_shapes(shapes, data, "http://example.org/a");

    let person = out
        .iter()
        .find(|(_, title)| title.contains("Person") && !title.contains("Employee"))
        .unwrap_or_else(|| panic!("no Person trace found: {out:?}"));
    let employee = out
        .iter()
        .find(|(_, title)| title.contains("Employee"))
        .unwrap_or_else(|| panic!("no Employee trace found: {out:?}"));

    assert_eq!(person.0, Some(Verdict::Conforms), "{out:?}");
    assert_eq!(employee.0, Some(Verdict::Violates), "{out:?}");
}

/// Same misattribution bug, exercised for `sh:hasValue`: like `sh:class`,
/// `sh:hasValue ex:Red, ex:Blue` parses into two separate
/// `Constraint::HasValue` instances (see `parser/constraints/has_value.rs`,
/// which also iterates `objects_for_subject_predicate`).
#[test]
fn same_shape_has_value_constraints_are_disambiguated_by_value() {
    let shapes = r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:TagShape a sh:NodeShape ;
            sh:targetNode ex:a ;
            sh:property [ sh:path ex:tag ; sh:hasValue ex:Red, ex:Blue ] .
    "#;
    // ex:a has the Red tag but not the Blue tag.
    let data = "@prefix ex: <http://example.org/> .
        ex:a ex:tag ex:Red .";
    let out = run_with_shapes(shapes, data, "http://example.org/a");

    let red = out
        .iter()
        .find(|(_, title)| title.contains("Red"))
        .unwrap_or_else(|| panic!("no Red trace found: {out:?}"));
    let blue = out
        .iter()
        .find(|(_, title)| title.contains("Blue"))
        .unwrap_or_else(|| panic!("no Blue trace found: {out:?}"));

    assert_eq!(red.0, Some(Verdict::Conforms), "{out:?}");
    assert_eq!(blue.0, Some(Verdict::Violates), "{out:?}");
}
