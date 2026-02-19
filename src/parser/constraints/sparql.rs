use oxigraph::model::{vocab::rdf, Graph, NamedOrBlankNodeRef, TermRef};

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
) -> Option<SparqlExecutable> {
    if let Some(TermRef::Literal(lit)) = graph.object_for_subject_predicate(executable_node, sh::SELECT) {
        return Some(SparqlExecutable::Select(lit.value().to_string()));
    }

    if let Some(TermRef::Literal(lit)) = graph.object_for_subject_predicate(executable_node, sh::ASK) {
        return Some(SparqlExecutable::Ask(lit.value().to_string()));
    }

    None
}

fn parse_direct_shape_sparql_constraints<'a>(
    graph: &'a Graph,
    shape_node: NamedOrBlankNodeRef<'a>,
) -> Vec<Constraint<'a>> {
    let mut constraints = Vec::new();
    let mut seen_sources = std::collections::HashSet::new();

    for sparql_term in graph.objects_for_subject_predicate(shape_node, sh::SPARQL) {
        let Some(executable_node) = term_to_named_or_blank(sparql_term) else {
            continue;
        };

        if !seen_sources.insert(executable_node) {
            continue;
        }

        let Some(executable) = parse_executable(graph, executable_node) else {
            continue;
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
        if let Some(executable) = parse_executable(graph, shape_node) {
            constraints.push(Constraint::Sparql(SparqlConstraint {
                source_constraint: Some(shape_node),
                source_constraint_component: None,
                executable,
                messages: get_all_string_values(graph, shape_node, sh::MESSAGE),
                prefixes: parse_shacl_prefixes(graph, shape_node),
                parameter_bindings: Vec::new(),
            }));
        }
    }

    constraints
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
) -> Vec<Constraint<'a>> {
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

                let Some(executable) = parse_executable(graph, validator_node) else {
                    continue;
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

    constraints
}

pub fn parse_sparql_constraints<'a>(
    graph: &'a Graph,
    shape_node: NamedOrBlankNodeRef<'a>,
    is_property_shape: bool,
) -> Result<Vec<Constraint<'a>>, ShaclError> {
    let mut constraints = parse_direct_shape_sparql_constraints(graph, shape_node);
    constraints.extend(parse_component_sparql_constraints(
        graph,
        shape_node,
        is_property_shape,
    ));
    Ok(constraints)
}
