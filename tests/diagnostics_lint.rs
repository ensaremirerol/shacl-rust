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
