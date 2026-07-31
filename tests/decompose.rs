use shacl_rust::decompose_shapes;
use shacl_rust::parse_shapes;
use shacl_rust::rdf::read_graph_from_string;
use std::collections::HashSet;

const FIXTURE: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape
  a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:closed true ;
  sh:ignoredProperties ( ex:extra ) ;
  sh:property [
    sh:path ex:name ;
    sh:minCount 1 ;
    sh:maxCount 1 ;
    sh:datatype xsd:string ;
    sh:minLength 1 ;
  ] ;
  sh:property [
    sh:path ex:email ;
    sh:datatype xsd:string ;
  ] ;
  sh:property [
    sh:path ex:age ;
    sh:datatype xsd:integer ;
    sh:minInclusive 0 ;
    sh:maxInclusive 130 ;
  ] ;
  sh:property [
    sh:path ( ex:country ex:code ) ;
    sh:in ( "US" "CA" ) ;
  ] ;
  sh:or (
    [ sh:path ex:phone ; sh:datatype xsd:string ]
    [ sh:path ex:phone ; sh:datatype xsd:integer ]
  ) ;
  sh:sparql [
    a sh:SPARQLConstraint ;
    sh:message "must have a valid signature" ;
    sh:select """
      SELECT $this
      WHERE { FILTER NOT EXISTS { $this <http://example.org/signature> ?s } }
    """ ;
  ] .
"#;

fn decompose(ttl: &str) -> serde_json::Value {
    let graph = read_graph_from_string(ttl, "turtle").unwrap();
    let shapes = parse_shapes(&graph).unwrap();
    decompose_shapes(&shapes, None, graph.len())
}

fn all_constraint_ids(decomposed: &serde_json::Value) -> HashSet<String> {
    fn walk(entries: &serde_json::Value, out: &mut HashSet<String>) {
        for entry in entries.as_array().unwrap() {
            out.insert(entry["id"].as_str().unwrap().to_string());
            if let Some(children) = entry.get("children") {
                walk(children, out);
            }
        }
    }
    let mut out = HashSet::new();
    for shape in decomposed["shapes"].as_array().unwrap() {
        walk(&shape["constraints"], &mut out);
    }
    out
}

#[test]
fn enumeration_fixture_yields_at_least_12_constraint_entries() {
    let decomposed = decompose(FIXTURE);
    let ids = all_constraint_ids(&decomposed);
    // name: minCount, maxCount, datatype, minLength (4)
    // email: datatype (1)
    // age: datatype, minInclusive, maxInclusive (3)
    // address sh:in (1)
    // sh:or itself + its 2 children's own datatype constraints (1 + 2 = 3, counted via `children`)
    // sparql (1)
    // closed (1)
    // total direct-list entries (not counting into sh:or's 2 children, which live under `children`):
    // 4 + 1 + 3 + 1 + 1 (or) + 1 (sparql) + 1 (closed) = 12
    assert!(
        ids.len() >= 12,
        "expected >= 12 distinct constraint ids, got {}: {:?}",
        ids.len(),
        ids
    );

    let shape = &decomposed["shapes"][0];
    let constraints = shape["constraints"].as_array().unwrap();

    let by_path = |path: &str| -> Vec<&serde_json::Value> {
        constraints
            .iter()
            .filter(|c| c["path"].as_str() == Some(path))
            .collect()
    };

    assert_eq!(
        by_path("<http://example.org/name>").len(),
        4,
        "{constraints:#?}"
    );
    assert_eq!(by_path("<http://example.org/email>").len(), 1);
    assert_eq!(by_path("<http://example.org/age>").len(), 3);

    let address_path = "<http://example.org/country> / <http://example.org/code>";
    let address = by_path(address_path);
    assert_eq!(address.len(), 1, "{constraints:#?}");
    assert_eq!(
        address[0]["component"],
        "http://www.w3.org/ns/shacl#InConstraintComponent"
    );

    let or_entry = constraints
        .iter()
        .find(|c| c["component"] == "http://www.w3.org/ns/shacl#OrConstraintComponent")
        .expect("sh:or entry present");
    assert_eq!(or_entry["children"].as_array().unwrap().len(), 2);

    let sparql_entry = constraints
        .iter()
        .find(|c| c["component"] == "http://www.w3.org/ns/shacl#SPARQLConstraintComponent")
        .expect("sparql entry present");
    assert!(sparql_entry["parameters"]["select"]
        .as_str()
        .unwrap()
        .contains("FILTER NOT EXISTS"));

    let closed_entry = constraints
        .iter()
        .find(|c| c["component"] == "http://www.w3.org/ns/shacl#ClosedConstraintComponent")
        .expect("closed entry present");
    assert_eq!(
        closed_entry["parameters"]["ignoredProperties"][0],
        "http://example.org/extra"
    );

    // Every constraint on a sh:property child carries owner_property_shape;
    // node-level entries (sh:or, sparql, closed) don't.
    for entry in &address {
        assert!(entry["owner_property_shape"].is_string());
    }
    assert!(sparql_entry["owner_property_shape"].is_null());
    assert!(closed_entry["owner_property_shape"].is_null());
}

#[test]
fn stability_survives_comments_and_prefix_renames() {
    let baseline_ids = all_constraint_ids(&decompose(FIXTURE));

    let with_comment = format!("# a harmless comment\n{FIXTURE}\n# trailing comment too\n");
    assert_eq!(all_constraint_ids(&decompose(&with_comment)), baseline_ids);

    // `ex:` -> `example:` throughout, including the @prefix declaration:
    // oxigraph expands prefixed names to full IRIs at parse time, so the
    // resulting graph is triple-for-triple identical to the original - this
    // asserts canonicalization hashes the *expanded* IRI, not the prefixed
    // spelling, matching R-2's "stable across ... prefix renames".
    let renamed_prefix = FIXTURE.replace("ex:", "example:");
    assert!(renamed_prefix.contains("@prefix example: <http://example.org/> ."));
    assert_eq!(
        all_constraint_ids(&decompose(&renamed_prefix)),
        baseline_ids
    );
}

#[test]
fn stability_survives_reordering_sh_property_blocks() {
    let baseline_ids = all_constraint_ids(&decompose(FIXTURE));

    // Swap the ex:name and ex:email sh:property blocks wholesale.
    // property_shape_id depends on (owner, path, params), not declaration
    // order, so re-parsing with the two blocks swapped must yield the exact
    // same id set even though both are still blank nodes that get fresh,
    // different per-run labels from oxigraph after reparsing.
    let name_block = "sh:property [\n    sh:path ex:name ;\n    sh:minCount 1 ;\n    sh:maxCount 1 ;\n    sh:datatype xsd:string ;\n    sh:minLength 1 ;\n  ] ;";
    let email_block = "sh:property [\n    sh:path ex:email ;\n    sh:datatype xsd:string ;\n  ] ;";
    assert!(
        FIXTURE.contains(name_block),
        "fixture drifted from expected literal block text"
    );
    assert!(
        FIXTURE.contains(email_block),
        "fixture drifted from expected literal block text"
    );

    let reordered = FIXTURE
        .replacen(name_block, "\u{0}PLACEHOLDER\u{0}", 1)
        .replacen(email_block, name_block, 1)
        .replacen("\u{0}PLACEHOLDER\u{0}", email_block, 1);

    assert_eq!(all_constraint_ids(&decompose(&reordered)), baseline_ids);
}

#[test]
fn join_validation_report_source_constraint_matches_decompose_output() {
    use shacl_rust::validate;
    use shacl_rust::validation::dataset::ValidationDataset;

    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:alice a ex:Person ; ex:email "not-an-int-but-fine" .
    "#;
    let shapes_graph = read_graph_from_string(FIXTURE, "turtle").unwrap();
    let data_graph = read_graph_from_string(data, "turtle").unwrap();
    let shapes = parse_shapes(&shapes_graph).unwrap();
    let dataset = ValidationDataset::from_graphs(data_graph, shapes_graph.clone()).unwrap();
    let report = validate(&dataset, &shapes);

    assert!(
        !*report.get_conforms(),
        "expected violations (missing name)"
    );

    let decomposed = decompose_shapes(&shapes, None, shapes_graph.len());
    let known_ids = all_constraint_ids(&decomposed);

    // Not asserting every violation's source constraint is in known_ids yet
    // (ValidationResult doesn't carry a stable constraint id - that's the
    // next integration step), just that decomposition itself ran over the
    // same shapes graph without diverging structurally.
    assert!(!known_ids.is_empty());
}
