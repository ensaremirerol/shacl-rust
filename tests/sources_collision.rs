use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::sources::{detect_collisions, merge_sources, NamedSource};

fn named(name: &str, ttl: &str) -> NamedSource {
    NamedSource {
        name: name.to_string(),
        graph: read_graph_from_string(ttl, "turtle").unwrap(),
    }
}

const SET_A: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;
        sh:property [ sh:path ex:name ; sh:minCount 1 ] .
"#;

const SET_B_CONFLICTING: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Employee ;
        sh:property [ sh:path ex:badge ; sh:minCount 1 ] .
"#;

#[test]
fn conflicting_definitions_across_sources_yield_d0001() {
    let sources = vec![named("set-a", SET_A), named("set-b", SET_B_CONFLICTING)];
    let diags = detect_collisions(&sources);

    let d0001 = diags
        .iter()
        .find(|d| d.code == "D0001")
        .expect("expected a D0001 diagnostic");
    assert_eq!(
        d0001.source_shape.as_deref(),
        Some("<http://example.org/PersonShape>")
    );
    let joined_notes = d0001.notes.join(" | ");
    assert!(joined_notes.contains("set-a"), "{joined_notes}");
    assert!(joined_notes.contains("set-b"), "{joined_notes}");
    assert!(!diags.iter().any(|d| d.code == "D0002"), "{diags:?}");
}

#[test]
fn identical_redefinition_across_sources_yields_d0002_not_d0001() {
    let sources = vec![named("set-a", SET_A), named("set-c", SET_A)];
    let diags = detect_collisions(&sources);

    assert!(!diags.iter().any(|d| d.code == "D0001"), "{diags:?}");
    let d0002 = diags
        .iter()
        .find(|d| d.code == "D0002")
        .expect("expected a D0002 diagnostic");
    let joined_notes = d0002.notes.join(" | ");
    assert!(joined_notes.contains("set-a"), "{joined_notes}");
    assert!(joined_notes.contains("set-c"), "{joined_notes}");
}

#[test]
fn distinct_shape_iris_across_sources_produce_no_collision_diagnostics() {
    let other = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:AddressShape a sh:NodeShape ; sh:targetClass ex:Address .
    "#;
    let sources = vec![named("set-a", SET_A), named("set-d", other)];
    assert!(detect_collisions(&sources).is_empty());
}

#[test]
fn merge_still_unions_all_sources_regardless_of_collisions() {
    let sources = vec![named("set-a", SET_A), named("set-b", SET_B_CONFLICTING)];
    let merged = merge_sources(&sources);
    // Both sources' sh:targetClass triples survive the union (this is the
    // exact silent-merge behavior D0001 exists to surface, not prevent -
    // validation must still be able to proceed).
    let target_class =
        oxigraph::model::NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#targetClass");
    let person_shape = oxigraph::model::NamedOrBlankNodeRef::from(
        oxigraph::model::NamedNodeRef::new_unchecked("http://example.org/PersonShape"),
    );
    let targets: Vec<_> = merged
        .objects_for_subject_predicate(person_shape, target_class)
        .collect();
    assert_eq!(targets.len(), 2, "{targets:?}");
}
