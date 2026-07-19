use criterion::{criterion_group, criterion_main, Criterion};
use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::{parse_shapes, validation};
use std::hint::black_box;

fn generate_data(num_persons: usize) -> String {
    let mut ttl = String::from(
        "@prefix ex: <http://example.org/> .\n\
         @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
         @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    );
    for i in 0..num_persons {
        ttl.push_str(&format!(
            "ex:person{i} rdf:type ex:Person ;\n\
             \tex:name \"Person {i}\" ;\n\
             \tex:age \"{}\"^^xsd:integer ;\n\
             \tex:email \"person{i}@example.org\" ;\n\
             \tex:knows ex:person{} .\n",
            20 + (i % 60),
            (i + 1) % num_persons,
        ));
    }
    ttl
}

const SHAPES: &str = r#"
@prefix ex: <http://example.org/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [
        sh:path ex:name ;
        sh:minCount 1 ;
        sh:datatype xsd:string ;
    ] ;
    sh:property [
        sh:path ex:age ;
        sh:datatype xsd:integer ;
        sh:minInclusive 0 ;
        sh:maxInclusive 150 ;
    ] ;
    sh:property [
        sh:path ex:email ;
        sh:pattern "^[^@]+@[^@]+\\.[a-z]+$" ;
    ] ;
    sh:property [
        sh:path ex:knows ;
        sh:class ex:Person ;
    ] .
"#;

const SPARQL_SHAPES: &str = r#"
@prefix ex: <http://example.org/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

ex:PersonSparqlShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:sparql [
        sh:message "Age must not be negative" ;
        sh:select """SELECT $this WHERE { $this <http://example.org/age> ?age . FILTER (?age < 0) }""" ;
    ] .
"#;

fn bench_sparql_validation(c: &mut Criterion) {
    for &size in &[100usize, 1000] {
        let data_ttl = generate_data(size);
        let data_graph = read_graph_from_string(&data_ttl, "turtle").unwrap();
        let shapes_graph = read_graph_from_string(SPARQL_SHAPES, "turtle").unwrap();

        let shapes = parse_shapes(&shapes_graph).unwrap();
        let dataset = validation::dataset::ValidationDataset::from_graphs(
            data_graph.clone(),
            shapes_graph.clone(),
        )
        .unwrap();
        c.bench_function(&format!("sparql_validate_{size}"), |b| {
            b.iter(|| {
                let report = validation::validate(&dataset, &shapes);
                black_box(report.get_results().len())
            })
        });
    }
}

fn bench_validation(c: &mut Criterion) {
    for &size in &[100usize, 1000] {
        let data_ttl = generate_data(size);
        let data_graph = read_graph_from_string(&data_ttl, "turtle").unwrap();
        let shapes_graph = read_graph_from_string(SHAPES, "turtle").unwrap();

        // Measures parsing + dataset construction + validation.
        c.bench_function(&format!("full_pipeline_{size}"), |b| {
            b.iter(|| {
                let shapes = parse_shapes(&shapes_graph).unwrap();
                let dataset = validation::dataset::ValidationDataset::from_graphs(
                    data_graph.clone(),
                    shapes_graph.clone(),
                )
                .unwrap();
                let report = validation::validate(&dataset, &shapes);
                black_box(report.get_results().len())
            })
        });

        // Measures validation only, with shapes and dataset prepared once.
        let shapes = parse_shapes(&shapes_graph).unwrap();
        let dataset = validation::dataset::ValidationDataset::from_graphs(
            data_graph.clone(),
            shapes_graph.clone(),
        )
        .unwrap();
        c.bench_function(&format!("validate_only_{size}"), |b| {
            b.iter(|| {
                let report = validation::validate(&dataset, &shapes);
                black_box(report.get_results().len())
            })
        });
    }
}

criterion_group!(benches, bench_validation, bench_sparql_validation);
criterion_main!(benches);
