use shacl_rust::diagnostics::lint_shapes;
use shacl_rust::parse_shapes;
use shacl_rust::rdf::read_graph_from_string;

fn lint_codes(shapes_ttl: &str) -> Vec<&'static str> {
    let g = read_graph_from_string(shapes_ttl, "turtle").unwrap();
    let shapes = parse_shapes(&g).unwrap_or_default();
    lint_shapes(&g, &shapes).iter().map(|d| d.code).collect()
}

#[test]
fn lint_rules_fire() {
    let cases: &[(&str, &str)] = &[
        ("L0001", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:minCount 1 ] ."),
        ("L0002", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:pattern \"[unclosed\" ] ."),
        ("L0003", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:sparql [ sh:select \"SELECT $this WHERE {\" ] ."),
        ("L0004", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:minCont 1 ] ."),
        ("L0005", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:minCount 1 ."),
        ("L0006", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:minCount 3 ; sh:maxCount 1 ] ."),
        ("L0007", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:datatype \"notAnIri\" ] ."),
        ("L0008", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:in () ] ."),
        ("L0009", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:minInclusive ex:NotALiteral ] ."),
        ("L0010", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:ignoredProperties ( rdf:type ) ."),
        ("L0011", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount 1 ] ."),
        ("L0012", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:deactivated true ."),
        ("L0013", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:closed true ; sh:ignoredProperties rdf:type ; sh:property [ sh:path ex:p ] ."),
        ("L0014", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:age ; sh:datatype xsd:integer ; sh:datatype xsd:string ] ."),
        ("L0015", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:PersonShapeA a sh:NodeShape ; sh:targetClass ex:Person ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .
          ex:PersonShapeB a sh:NodeShape ; sh:targetClass ex:Person ; sh:property [ sh:path ex:name ; sh:minCount 1 ] ."),
    ];
    for (code, ttl) in cases {
        let codes = lint_codes(ttl);
        assert!(codes.contains(code), "expected {code}, got {codes:?}");
    }
}

#[test]
fn clean_shapes_produce_no_lints() {
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:P ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ; sh:maxCount 5 ] .";
    assert!(lint_codes(ttl).is_empty(), "{:?}", lint_codes(ttl));
}

#[test]
fn did_you_mean_suggests_close_term() {
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:minCont 1 ] .";
    let g = read_graph_from_string(ttl, "turtle").unwrap();
    let shapes = parse_shapes(&g).unwrap();
    let diags = lint_shapes(&g, &shapes);
    let l4 = diags.iter().find(|d| d.code == "L0004").unwrap();
    assert!(
        l4.help.as_deref().unwrap().contains("minCount"),
        "{:?}",
        l4.help
    );
}

#[test]
fn node_shape_pair_constraints_are_not_l0005() {
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:equals ex:other .";
    assert!(!lint_codes(ttl).contains(&"L0005"), "{:?}", lint_codes(ttl));
}

#[test]
fn well_formed_ignored_properties_list_is_not_l0013() {
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:closed true ; sh:ignoredProperties ( rdf:type ) ; sh:property [ sh:path ex:p ] .";
    assert!(!lint_codes(ttl).contains(&"L0013"), "{:?}", lint_codes(ttl));
}

#[test]
fn literal_ignored_properties_is_l0013() {
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:closed true ; sh:ignoredProperties \"oops\" ; sh:property [ sh:path ex:p ] .";
    assert!(lint_codes(ttl).contains(&"L0013"), "{:?}", lint_codes(ttl));
}

#[test]
fn repeated_identical_single_valued_parameter_is_not_l0014() {
    // sh:datatype declared twice but with the SAME value: no conflict, no
    // dropped information - RDF has no duplicate-triple concept anyway.
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:age ; sh:datatype xsd:integer ; sh:datatype xsd:integer ] .";
    assert!(!lint_codes(ttl).contains(&"L0014"), "{:?}", lint_codes(ttl));
}

#[test]
fn repeated_sh_class_is_not_l0014() {
    // sh:class is legitimately multi-valued per spec (instance of ALL
    // listed classes) - must not be in SINGLE_VALUED_PREDICATES.
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:class ex:A ; sh:class ex:B ] .";
    assert!(!lint_codes(ttl).contains(&"L0014"), "{:?}", lint_codes(ttl));
}

#[test]
fn conflicting_min_count_values_is_l0014() {
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:minCount 1 ; sh:minCount 2 ] .";
    assert!(lint_codes(ttl).contains(&"L0014"), "{:?}", lint_codes(ttl));
}

#[test]
fn same_path_different_targets_is_not_l0015() {
    // Same path/component/params, but the two shapes target different
    // classes - not a duplicate, they validate genuinely different nodes.
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        ex:CompanyShape a sh:NodeShape ; sh:targetClass ex:Company ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .";
    assert!(!lint_codes(ttl).contains(&"L0015"), "{:?}", lint_codes(ttl));
}

#[test]
fn same_target_different_params_is_not_l0015() {
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:PersonShapeA a sh:NodeShape ; sh:targetClass ex:Person ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        ex:PersonShapeB a sh:NodeShape ; sh:targetClass ex:Person ; sh:property [ sh:path ex:name ; sh:minCount 2 ] .";
    assert!(!lint_codes(ttl).contains(&"L0015"), "{:?}", lint_codes(ttl));
}

#[test]
fn distinct_sh_or_members_are_not_l0015() {
    // Two different sh:or constructs must not false-positive as
    // "duplicate" just because is_owner_scoped_constraint excludes them
    // from comparison (excluded == never matched, not always-matched).
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:ShapeA a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:or ( [ sh:path ex:p ; sh:datatype xsd:string ] [ sh:path ex:p ; sh:datatype xsd:integer ] ) .
        ex:ShapeB a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:or ( [ sh:path ex:q ; sh:datatype xsd:boolean ] [ sh:path ex:q ; sh:datatype xsd:date ] ) .";
    assert!(!lint_codes(ttl).contains(&"L0015"), "{:?}", lint_codes(ttl));
}
