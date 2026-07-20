//! Stress test: validates a generated person graph with both data-graph
//! backends (plain `oxigraph::model::Graph` and the experimental
//! `IndexedGraph`) and asserts they produce identical outcomes.
//!
//! Size defaults to a CI-friendly 20k persons; set `SHACL_STRESS_PERSONS` to
//! scale up locally (e.g. `SHACL_STRESS_PERSONS=250000 cargo test --test
//! stress --release -- --nocapture`).

use std::time::Instant;

use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::validation::dataset::ValidationDataset;
use shacl_rust::{parse_shapes, validation};

const SHAPES: &str = r#"
@prefix ex: <http://example.org/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:datatype xsd:string ; ] ;
    sh:property [
        sh:path ex:age ;
        sh:datatype xsd:integer ;
        sh:minInclusive 0 ;
        sh:maxInclusive 150 ;
    ] ;
    sh:property [ sh:path ex:email ; sh:pattern "^[^@]+@[^@]+\\.[a-z]+$" ; ] ;
    sh:property [ sh:path ex:knows ; sh:class ex:Person ; ] .
"#;

fn generate_person_graph(num_persons: usize) -> String {
    let mut ttl = String::with_capacity(num_persons * 160);
    ttl.push_str(
        "@prefix ex: <http://example.org/> .\n\
         @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
         @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    );
    for i in 0..num_persons {
        // Every 1000th person violates minInclusive (negative age) and the
        // email pattern, so result counts are exercised, not just conforms.
        let age: i64 = if i % 1000 == 999 {
            -1
        } else {
            20 + (i as i64 % 60)
        };
        let email_domain = if i % 1000 == 999 {
            "no-tld"
        } else {
            "example.org"
        };
        ttl.push_str(&format!(
            "ex:person{i} rdf:type ex:Person ;\n\
             \tex:name \"Person {i}\" ;\n\
             \tex:age \"{age}\"^^xsd:integer ;\n\
             \tex:email \"person{i}@{email_domain}\" ;\n\
             \tex:knows ex:person{} .\n",
            (i + 1) % num_persons,
        ));
    }
    ttl
}

#[test]
fn backends_agree_on_large_person_graph() {
    let num_persons: usize = std::env::var("SHACL_STRESS_PERSONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000);
    let expected_violating_persons = num_persons / 1000;

    let data_ttl = generate_person_graph(num_persons);
    let shapes_graph = read_graph_from_string(SHAPES, "turtle").expect("shapes parse");

    let t = Instant::now();
    let data_graph = read_graph_from_string(&data_ttl, "turtle").expect("data parse");
    let plain_load = t.elapsed();

    let shapes = parse_shapes(&shapes_graph).expect("parse shapes");

    let t = Instant::now();
    let plain_dataset = ValidationDataset::from_graphs(data_graph.clone(), shapes_graph.clone())
        .expect("plain dataset");
    let plain_report = validation::validate(&plain_dataset, &shapes);
    let plain_validate = t.elapsed();

    let t = Instant::now();
    let indexed_dataset = ValidationDataset::from_triples_with_experimental_index(
        data_graph.iter().map(oxigraph::model::Triple::from),
        shapes_graph.clone(),
    )
    .expect("indexed dataset");
    let indexed_build = t.elapsed();

    let t = Instant::now();
    let indexed_report = validation::validate(&indexed_dataset, &shapes);
    let indexed_validate = t.elapsed();

    println!(
        "persons={num_persons} | plain: graph-load {plain_load:?}, validate {plain_validate:?} | \
         indexed: build {indexed_build:?}, validate {indexed_validate:?}"
    );
    println!(
        "plain: conforms={} results={} | indexed: conforms={} results={}",
        plain_report.get_conforms(),
        plain_report.get_results().len(),
        indexed_report.get_conforms(),
        indexed_report.get_results().len(),
    );

    assert_eq!(
        plain_report.get_conforms(),
        indexed_report.get_conforms(),
        "backends disagree on conformance"
    );
    assert_eq!(
        plain_report.get_results().len(),
        indexed_report.get_results().len(),
        "backends disagree on violation count"
    );
    if expected_violating_persons > 0 {
        assert!(!*plain_report.get_conforms());
        // Each violating person breaks minInclusive and the email pattern.
        assert_eq!(
            plain_report.get_results().len(),
            expected_violating_persons * 2,
            "unexpected violation count"
        );
    }
}
