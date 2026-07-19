use std::collections::HashSet;

use oxigraph::model::{vocab::rdf, Graph, NamedOrBlankNodeRef, TermRef};
use spargebra::{algebra::GraphPattern, term::Variable, Query, SparqlParser};

use crate::{
    core::constraints::{Constraint, SparqlConstraint, SparqlExecutable},
    err::ShaclError,
    utils::{
        get_all_string_values, get_boolean_value, is_subclass_of, local_name_from_iri,
        parse_shacl_prefixes, term_to_named_or_blank,
    },
    vocab::sh,
};

fn parse_executable<'a>(
    graph: &'a Graph,
    executable_node: NamedOrBlankNodeRef<'a>,
) -> Result<Option<SparqlExecutable>, ShaclError> {
    let sparql_executable = {
        if let Some(TermRef::Literal(lit)) =
            graph.object_for_subject_predicate(executable_node, sh::SELECT)
        {
            SparqlExecutable::Select(lit.value().to_string())
        } else if let Some(TermRef::Literal(lit)) =
            graph.object_for_subject_predicate(executable_node, sh::ASK)
        {
            SparqlExecutable::Ask(lit.value().to_string())
        } else {
            return Ok(None);
        }
    };

    let error = unsupported_prebinding_construct(
        match &sparql_executable {
            SparqlExecutable::Select(query) | SparqlExecutable::Ask(query) => query,
        },
        &parse_shacl_prefixes(graph, executable_node),
    );

    if let Some(error) = error {
        return Err(ShaclError::Parse(format!(
            "Unsupported SPARQL construct for SHACL pre-binding: {}",
            error
        )));
    }

    Ok(Some(sparql_executable))
}

/// Walks a query's algebra looking for constructs SHACL pre-binding can't support.
///
/// `at_top` tracks whether we're still within the chain of modifiers wrapping the
/// *outermost* query itself (its own Project/Filter/OrderBy/etc., however many layers
/// that happens to be) versus having crossed into a branch of a real combinator
/// (Join/Union/LeftJoin/Lateral) where a genuinely nested sub-SELECT could live. This
/// avoids hardcoding "how many Project layers does a top-level ASK vs. SELECT have" --
/// that's an internal representation detail of the SPARQL algebra library and not
/// stable across its versions (e.g. spargebra 0.4.6 started wrapping ASK's pattern in
/// an explicit `Project { variables: [], .. }`, where 0.4.5 didn't wrap it at all;
/// counting a fixed "remaining projects" budget per query type broke against that).
fn unsupported_in_pattern(
    pattern: &GraphPattern,
    at_top: bool,
    required_prebound_vars: &HashSet<Variable>,
) -> Option<&'static str> {
    match pattern {
        GraphPattern::Minus { .. } => Some("MINUS is not supported for SHACL pre-binding"),
        GraphPattern::Service { .. } => Some("SERVICE is not supported for SHACL pre-binding"),
        GraphPattern::Project { variables, inner } if !at_top => {
            // Nested SELECT is only allowed if it explicitly projects all pre-bound variables
            let projected_vars: HashSet<_> = variables.iter().cloned().collect();
            if !required_prebound_vars.is_subset(&projected_vars) {
                return Some(
                    "Nested SELECT must explicitly project all pre-bound variables (e.g., $this)",
                );
            }
            unsupported_in_pattern(inner, false, required_prebound_vars)
        }
        // Still within the outer query's own modifier chain -- transparent regardless
        // of how many layers deep, since none of these introduce a new sub-query scope.
        GraphPattern::Project { inner, .. }
        | GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => {
            unsupported_in_pattern(inner, at_top, required_prebound_vars)
        }
        // A real combination point: either branch from here on is a genuinely separate
        // (possibly nested-SELECT) sub-pattern, not part of the outer query's own chain.
        GraphPattern::Join { left, right } | GraphPattern::Union { left, right } => {
            unsupported_in_pattern(left, false, required_prebound_vars)
                .or_else(|| unsupported_in_pattern(right, false, required_prebound_vars))
        }
        GraphPattern::LeftJoin { left, right, .. } => {
            unsupported_in_pattern(left, false, required_prebound_vars)
                .or_else(|| unsupported_in_pattern(right, false, required_prebound_vars))
        }
        GraphPattern::Lateral { left, right } => {
            unsupported_in_pattern(left, false, required_prebound_vars)
                .or_else(|| unsupported_in_pattern(right, false, required_prebound_vars))
        }
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => None,
    }
}

fn unsupported_prebinding_construct(
    query: &str,
    prefixes: &[(String, String)],
) -> Option<&'static str> {
    let mut parser = SparqlParser::new();
    for (prefix, namespace) in prefixes {
        if let Ok(with_prefix) = parser
            .clone()
            .with_prefix(prefix.clone(), namespace.clone())
        {
            parser = with_prefix;
        }
    }

    let parsed = match parser.parse_query(query) {
        Ok(parsed) => parsed,
        Err(_) => return None,
    };

    let (pattern, prebound_vars) = match parsed {
        Query::Select { pattern, .. } | Query::Ask { pattern, .. } => {
            // $this is always pre-bound, for both SELECT and ASK queries.
            let mut vars = HashSet::new();
            vars.insert(Variable::new_unchecked("this"));
            (pattern, vars)
        }
        Query::Construct { pattern, .. } | Query::Describe { pattern, .. } => {
            (pattern, HashSet::new())
        }
    };

    unsupported_in_pattern(&pattern, true, &prebound_vars)
}

fn parse_direct_shape_sparql_constraints<'a>(
    graph: &'a Graph,
    shape_node: NamedOrBlankNodeRef<'a>,
) -> Result<Vec<Constraint<'a>>, ShaclError> {
    let mut constraints = Vec::new();
    let mut seen_sources = std::collections::HashSet::new();

    for sparql_term in graph.objects_for_subject_predicate(shape_node, sh::SPARQL) {
        let Some(executable_node) = term_to_named_or_blank(sparql_term) else {
            continue;
        };

        if !seen_sources.insert(executable_node) {
            continue;
        };

        let executable = match parse_executable(graph, executable_node)? {
            Some(executable) => executable,
            None => continue,
        };

        constraints.push(Constraint::Sparql(SparqlConstraint {
            source_constraint: Some(executable_node),
            source_constraint_component: None,
            executable,
            messages: get_all_string_values(graph, executable_node, sh::MESSAGE),
            prefixes: parse_shacl_prefixes(graph, executable_node),
            parameter_bindings: Vec::new(),
        }));
    }

    if seen_sources.insert(shape_node) {
        let executable = match parse_executable(graph, shape_node)? {
            Some(executable) => executable,
            None => return Ok(constraints),
        };
        constraints.push(Constraint::Sparql(SparqlConstraint {
            source_constraint: Some(shape_node),
            source_constraint_component: None,
            executable,
            messages: get_all_string_values(graph, shape_node, sh::MESSAGE),
            prefixes: parse_shacl_prefixes(graph, shape_node),
            parameter_bindings: Vec::new(),
        }));
    }

    Ok(constraints)
}

fn is_constraint_component_instance<'a>(
    graph: &'a Graph,
    component: NamedOrBlankNodeRef<'a>,
) -> bool {
    graph
        .objects_for_subject_predicate(component, rdf::TYPE)
        .filter_map(term_to_named_or_blank)
        .any(|component_type| {
            component_type == sh::CONSTRAINT_COMPONENT.into()
                || is_subclass_of(component_type, sh::CONSTRAINT_COMPONENT.into(), graph)
        })
}

fn parse_component_parameter_bindings<'a>(
    graph: &'a Graph,
    component: NamedOrBlankNodeRef<'a>,
    shape_node: NamedOrBlankNodeRef<'a>,
) -> Option<Vec<(String, TermRef<'a>)>> {
    let mut bindings = Vec::new();

    for parameter_term in graph.objects_for_subject_predicate(component, sh::PARAMETER) {
        let parameter_node = term_to_named_or_blank(parameter_term)?;

        let path = graph
            .object_for_subject_predicate(parameter_node, sh::PATH)
            .and_then(|t| match t {
                TermRef::NamedNode(nn) => Some(nn),
                _ => None,
            })?;

        let var_name = local_name_from_iri(path.as_str())?;
        let optional = get_boolean_value(graph, parameter_node, sh::OPTIONAL).unwrap_or(false);

        let mut values = graph.objects_for_subject_predicate(shape_node, path);
        if let Some(value) = values.next() {
            bindings.push((var_name, value));
        } else if !optional {
            return None;
        }
    }

    Some(bindings)
}

fn parse_component_sparql_constraints<'a>(
    graph: &'a Graph,
    shape_node: NamedOrBlankNodeRef<'a>,
    is_property_shape: bool,
) -> Result<Vec<Constraint<'a>>, ShaclError> {
    let mut constraints = Vec::new();

    let mut validator_predicates = vec![sh::VALIDATOR];
    if is_property_shape {
        validator_predicates.push(sh::PROPERTY_VALIDATOR);
    } else {
        validator_predicates.push(sh::NODE_VALIDATOR);
    }

    let components: std::collections::HashSet<_> = graph
        .triples_for_predicate(sh::PARAMETER)
        .map(|triple| triple.subject)
        .collect();

    for component in components {
        if !is_constraint_component_instance(graph, component) {
            continue;
        }

        let Some(parameter_bindings) =
            parse_component_parameter_bindings(graph, component, shape_node)
        else {
            continue;
        };

        for predicate in &validator_predicates {
            for validator_term in graph.objects_for_subject_predicate(component, *predicate) {
                let Some(validator_node) = term_to_named_or_blank(validator_term) else {
                    continue;
                };

                let executable = match parse_executable(graph, validator_node)? {
                    Some(executable) => executable,
                    None => continue,
                };
                constraints.push(Constraint::Sparql(SparqlConstraint {
                    source_constraint: Some(validator_node),
                    source_constraint_component: Some(component),
                    executable,
                    messages: get_all_string_values(graph, validator_node, sh::MESSAGE),
                    prefixes: parse_shacl_prefixes(graph, validator_node),
                    parameter_bindings: parameter_bindings.clone(),
                }));
            }
        }
    }

    Ok(constraints)
}

pub fn parse_sparql_constraints<'a>(
    graph: &'a Graph,
    shape_node: NamedOrBlankNodeRef<'a>,
    is_property_shape: bool,
) -> Result<Vec<Constraint<'a>>, ShaclError> {
    let mut constraints = parse_direct_shape_sparql_constraints(graph, shape_node)?;
    constraints.extend(parse_component_sparql_constraints(
        graph,
        shape_node,
        is_property_shape,
    )?);
    Ok(constraints)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ask_filter_is_not_flagged_as_unsupported() {
        // Regression test: spargebra >=0.4.6 wraps an ASK query's pattern in a
        // top-level `Project { variables: [], .. }` (0.4.5 didn't wrap it at all),
        // which used to false-positive here because the detection assumed ASK
        // queries could never have a top-level Project node.
        let query = r#"ASK {
            FILTER ($value != $requiredParam) .
        }"#;
        assert_eq!(unsupported_prebinding_construct(query, &[]), None);
    }

    #[test]
    fn plain_select_star_is_not_flagged_as_unsupported() {
        let query = "SELECT * WHERE { $this ?p ?o . }";
        assert_eq!(unsupported_prebinding_construct(query, &[]), None);
    }

    #[test]
    fn nested_select_not_reprojecting_this_is_flagged() {
        let query = r#"
            SELECT $this
            WHERE {
                $this ?x ?any .
                {
                    SELECT ?other ?b
                    WHERE {
                        ?other ?b ?c .
                    }
                }
            }"#;
        assert_eq!(
            unsupported_prebinding_construct(query, &[]),
            Some("Nested SELECT must explicitly project all pre-bound variables (e.g., $this)")
        );
    }

    #[test]
    fn nested_select_reprojecting_this_is_allowed() {
        let query = r#"
            SELECT $this
            WHERE {
                $this ?x ?any .
                {
                    SELECT $this ?b
                    WHERE {
                        $this ?b ?c .
                    }
                }
            }"#;
        assert_eq!(unsupported_prebinding_construct(query, &[]), None);
    }

    #[test]
    fn minus_is_flagged() {
        let query = "SELECT $this WHERE { $this ?p ?o . MINUS { $this a <urn:x> } }";
        assert_eq!(
            unsupported_prebinding_construct(query, &[]),
            Some("MINUS is not supported for SHACL pre-binding")
        );
    }

    #[test]
    fn service_is_flagged() {
        let query = "SELECT $this WHERE { SERVICE <http://example.org/sparql> { $this ?p ?o } }";
        assert_eq!(
            unsupported_prebinding_construct(query, &[]),
            Some("SERVICE is not supported for SHACL pre-binding")
        );
    }
}
