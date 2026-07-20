//! Regressions distilled from the eCREAM/AIDAVA clinical benchmark: recursive
//! property paths and UNION inside sh:sparql selects, and SPARQL-based
//! targets. Expected outcome for every case: exactly one violation, focus
//! node ex:bad.

use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::validation::dataset::ValidationDataset;
use shacl_rust::{parse_shapes, validation};

/// ex:good's code reaches the measurement root via a subClassOf chain;
/// ex:bad's code is an orphan with no IS-A ancestors.
const DATA: &str = r#"
<http://example.org/good> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://biomedit.ch/rdf/sphn-ontology/sphn#Measurement> .
<http://example.org/good> <https://biomedit.ch/rdf/sphn-ontology/sphn#hasCode> <http://snomed.info/id/386725007> .
<http://example.org/bad> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://biomedit.ch/rdf/sphn-ontology/sphn#Measurement> .
<http://example.org/bad> <https://biomedit.ch/rdf/sphn-ontology/sphn#hasCode> <http://snomed.info/id/1371017004> .
<http://snomed.info/id/386725007> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://snomed.info/id/363789004> .
<http://snomed.info/id/363789004> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://snomed.info/id/363788007> .
<http://snomed.info/id/363788007> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://snomed.info/id/363787002> .
"#;

const SNOMED_CONSTRAINT: &str = r#"
    sh:sparql [
        a sh:SPARQLConstraint ;
        sh:message "Measurement code must be 363787002 or 105590001 or a subclass of it" ;
        sh:select """
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            SELECT $this ?code WHERE {
                ?this <https://biomedit.ch/rdf/sphn-ontology/sphn#hasCode> ?code .
                FILTER NOT EXISTS {
                    { ?code (rdfs:subClassOf|<http://snomed.info/id/116680003>)* <http://snomed.info/id/363787002> }
                    UNION
                    { ?code (rdfs:subClassOf|<http://snomed.info/id/116680003>)* <http://snomed.info/id/105590001> } .
                }
            }
        """
    ] .
"#;

fn validate(shapes_ttl: &str) -> Vec<String> {
    let data_graph = read_graph_from_string(DATA, "nt").expect("data parse");
    let shapes_graph = read_graph_from_string(shapes_ttl, "turtle").expect("shapes parse");
    let shapes = parse_shapes(&shapes_graph).expect("parse shapes");
    let dataset =
        ValidationDataset::from_graphs(data_graph, shapes_graph.clone()).expect("dataset");
    let report = validation::validate(&dataset, &shapes);
    let json = report.as_json();
    let mut focus: Vec<String> = json["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|r| r["focusNode"].as_str().expect("focusNode").to_string())
        .collect();
    focus.sort();
    focus.dedup();
    focus
}

/// Case 02: core sh:targetClass + SPARQL constraint with a recursive path and
/// UNION. The conforming node must NOT be flagged (was: phantom violation
/// whenever the query text contained "UNION").
#[test]
fn union_in_sparql_constraint_does_not_flag_conforming_nodes() {
    let shapes = format!(
        r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix sphn: <https://biomedit.ch/rdf/sphn-ontology/sphn#> .
        <http://codecheck#MeasurementSnomedCheck>
            a sh:NodeShape ;
            sh:targetClass sphn:Measurement ;
        {SNOMED_CONSTRAINT}
        "#
    );
    let focus = validate(&shapes);
    assert_eq!(
        focus,
        vec!["<http://example.org/bad>".to_string()],
        "exactly ex:bad must violate"
    );
}

/// Case 03 (control): the same check expressed as a core property path;
/// guards the core path engine.
#[test]
fn core_property_path_control_case() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix sphn: <https://biomedit.ch/rdf/sphn-ontology/sphn#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        <http://codecheck#MeasurementSnomedCheckCore>
            a sh:NodeShape ;
            sh:targetClass sphn:Measurement ;
            sh:property [
                sh:path ( sphn:hasCode [ sh:zeroOrMorePath [ sh:alternativePath ( rdfs:subClassOf <http://snomed.info/id/116680003> ) ] ] ) ;
                sh:qualifiedValueShape [ sh:hasValue <http://snomed.info/id/363787002> ] ;
                sh:qualifiedMinCount 1 ;
            ] .
    "#;
    let focus = validate(shapes);
    assert_eq!(
        focus,
        vec!["<http://example.org/bad>".to_string()],
        "exactly ex:bad must violate"
    );
}
