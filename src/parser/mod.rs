//! SHACL shape parsing.
pub mod constraint_parser_trait;
pub mod constraints;
pub mod path;
pub mod target;

use log::debug;
use oxigraph::model::{
    vocab::{rdf, rdfs},
    Graph, NamedNodeRef, NamedOrBlankNodeRef, TermRef,
};
use std::collections::HashSet;

use crate::{
    core::{
        constraints::Constraint,
        shape::{ClosedConstraint, Shape},
    },
    err::ShaclError,
    utils::{get_all_string_values, get_boolean_value, get_string_value, parse_rdf_list},
    vocab::sh,
};

use self::{path::parse_path, target::parse_targets};

/// Parses all SHACL shapes from a graph.
pub fn parse_shapes(graph: &Graph) -> Result<Vec<Shape<'_>>, ShaclError> {
    debug!("Starting shape parsing");

    #[cfg(not(target_family = "wasm"))]
    let time = std::time::Instant::now();

    let mut shapes = Vec::new();
    let mut visited = HashSet::new();

    let shape_nodes = find_shape_nodes(graph);
    debug!("Found {} shape nodes", shape_nodes.len());

    for shape_node in shape_nodes {
        if visited.contains(&shape_node) {
            continue;
        }
        visited.insert(shape_node);

        debug!("Parsing shape: {}", shape_node);
        match parse_shape(graph, shape_node, None) {
            Ok(shape) => {
                debug!("Successfully parsed shape: {}", shape_node);
                shapes.push(shape);
            }
            Err(e) => {
                log::warn!("Failed to parse shape {}: {}", shape_node, e);
            }
        }
    }

    #[cfg(not(target_family = "wasm"))]
    debug!("Finished shape parsing at {}", time.elapsed().as_secs_f64());

    debug!("Total shapes parsed: {}", shapes.len());
    Ok(shapes)
}

/// Returns nodes that look like SHACL shapes.
fn find_shape_nodes(graph: &Graph) -> HashSet<NamedOrBlankNodeRef<'_>> {
    let mut shape_nodes = HashSet::new();

    for shape_type in &[sh::NODE_SHAPE, sh::PROPERTY_SHAPE, sh::SHAPE] {
        shape_nodes.extend(graph.subjects_for_predicate_object(rdf::TYPE, *shape_type));
    }

    let shape_defining_predicates = &[
        sh::TARGET_CLASS,
        sh::TARGET_NODE,
        sh::TARGET_SUBJECTS_OF,
        sh::TARGET_OBJECTS_OF,
        sh::TARGET,
    ];

    for predicate in shape_defining_predicates {
        let subjects = graph
            .triples_for_predicate(*predicate)
            .map(|triple| triple.subject);
        shape_nodes.extend(subjects);
    }

    shape_nodes
}

fn parse_named_or_blank_node<'a>(term: TermRef<'a>) -> Option<NamedOrBlankNodeRef<'a>> {
    match term {
        TermRef::NamedNode(nn) => Some(NamedOrBlankNodeRef::NamedNode(nn)),
        TermRef::BlankNode(bn) => Some(NamedOrBlankNodeRef::BlankNode(bn)),
        _ => None,
    }
}

fn parse_severity<'a>(
    graph: &'a Graph,
    node: NamedOrBlankNodeRef<'a>,
    default: NamedNodeRef<'a>,
) -> NamedNodeRef<'a> {
    graph
        .object_for_subject_predicate(node, sh::SEVERITY)
        .and_then(|term| match term {
            TermRef::NamedNode(nn) => Some(nn),
            _ => None,
        })
        .unwrap_or(default)
}

fn apply_common_shape_properties<'a>(
    graph: &'a Graph,
    node: NamedOrBlankNodeRef<'a>,
    parent: Option<NamedOrBlankNodeRef<'a>>,
    mut shape: Shape<'a>,
) -> Shape<'a> {
    if let Some(name) = get_string_value(graph, node, sh::NAME)
        .or_else(|| get_string_value(graph, node, rdfs::LABEL))
    {
        shape = shape.with_name(name);
    }

    if let Some(desc) = get_string_value(graph, node, sh::DESCRIPTION) {
        shape = shape.with_description(desc);
    }

    if let Some(deactivated) = get_boolean_value(graph, node, sh::DEACTIVATED) {
        shape = shape.with_deactivated(deactivated);
    }

    for message in get_all_string_values(graph, node, sh::MESSAGE) {
        shape = shape.add_message(message);
    }

    if let Some(p) = parent {
        shape = shape.with_parent(p);
    }

    shape
}

fn parse_nested_property_shapes<'a>(
    graph: &'a Graph,
    node: NamedOrBlankNodeRef<'a>,
    parent_severity: NamedNodeRef<'a>,
    parent: Option<NamedOrBlankNodeRef<'a>>,
) -> Vec<Shape<'a>> {
    graph
        .objects_for_subject_predicate(node, sh::PROPERTY)
        .filter_map(parse_named_or_blank_node)
        .filter_map(|nested_prop_node| {
            parse_property_shape(graph, nested_prop_node, parent_severity, parent).ok()
        })
        .collect()
}

/// Parse a single shape from the graph
pub fn parse_shape<'a>(
    graph: &'a Graph,
    node: NamedOrBlankNodeRef<'a>,
    parent: Option<NamedOrBlankNodeRef<'a>>,
) -> Result<Shape<'a>, ShaclError> {
    // Check if this shape has sh:path - if so, it's a property shape with targets
    if let Some(path_obj) = graph.object_for_subject_predicate(node, sh::PATH) {
        return parse_top_level_property_shape(graph, node, path_obj, parent);
    }

    let severity = parse_severity(graph, node, sh::VIOLATION);

    parse_node_shape_internal(graph, node, severity, true, parent)
}

/// Parse a top-level property shape (a property shape with targets)
fn parse_top_level_property_shape<'a>(
    graph: &'a Graph,
    node: NamedOrBlankNodeRef<'a>,
    path_obj: TermRef<'a>,
    parent: Option<NamedOrBlankNodeRef<'a>>,
) -> Result<Shape<'a>, ShaclError> {
    // Parse the path
    let path = parse_path(graph, path_obj)?;

    let severity = parse_severity(graph, node, sh::VIOLATION);

    // Create property shape with the path
    let mut shape = apply_common_shape_properties(
        graph,
        node,
        parent,
        Shape::property_shape(node, path, severity),
    );

    // Parse targets (top-level property shapes can have targets)
    for target in parse_targets(graph, node) {
        shape = shape.add_target(target);
    }

    // Parse all constraints
    let constraints = parse_all_constraints(graph, node, true)?;
    for constraint in constraints {
        shape = shape.add_constraint(constraint);
    }

    // Parse nested property shapes (sh:property on property shapes)
    for nested_prop_shape in parse_nested_property_shapes(graph, node, severity, Some(node)) {
        shape = shape
            .add_property_shape(nested_prop_shape)
            .with_parent(node);
    }

    Ok(shape)
}

/// Parse a closed constraint
fn parse_closed_constraint<'a>(
    graph: &'a Graph,
    node: NamedOrBlankNodeRef<'a>,
) -> Option<ClosedConstraint<'a>> {
    if let Some(true) = get_boolean_value(graph, node, sh::CLOSED) {
        let mut ignored_properties = Vec::new();

        // Parse sh:ignoredProperties (should be an RDF list)
        if let Some(list_node) = graph.object_for_subject_predicate(node, sh::IGNORED_PROPERTIES) {
            let list_node_ref = match list_node {
                TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
                _ => return Some(ClosedConstraint { ignored_properties }), // Invalid ignoredProperties definition, treat as empty
            };
            ignored_properties = parse_rdf_list(graph, list_node_ref)
                .into_iter()
                .filter_map(|term| match term {
                    TermRef::NamedNode(nn) => Some(nn),
                    _ => None,
                })
                .collect();
        }
        Some(ClosedConstraint { ignored_properties })
    } else {
        None
    }
}

/// Internal helper to parse node shapes with or without targets
fn parse_node_shape_internal<'a>(
    graph: &'a Graph,
    node: NamedOrBlankNodeRef<'a>,
    severity: NamedNodeRef<'a>,
    include_targets: bool,
    parent: Option<NamedOrBlankNodeRef<'a>>,
) -> Result<Shape<'a>, ShaclError> {
    let mut shape =
        apply_common_shape_properties(graph, node, parent, Shape::node_shape(node, severity));

    // Parse targets (only for top-level shapes)
    if include_targets {
        for target in parse_targets(graph, node) {
            shape = shape.add_target(target);
        }
    }

    // Parse closed constraint
    if let Some(closed) = parse_closed_constraint(graph, node) {
        shape = shape.with_closed(closed);
    }

    // Parse property shapes (sh:property)
    for prop_shape in parse_nested_property_shapes(graph, node, severity, Some(node)) {
        shape = shape.add_property_shape(prop_shape).with_parent(node);
    }

    // Parse node-level constraints
    let node_constraints = parse_all_constraints(graph, node, false)?;
    for constraint in node_constraints {
        shape = shape.add_constraint(constraint)
    }

    Ok(shape)
}

/// Parse a property shape (a shape with sh:path)
fn parse_property_shape<'a>(
    graph: &'a Graph,
    node: NamedOrBlankNodeRef<'a>,
    parent_severity: NamedNodeRef<'a>,
    parent: Option<NamedOrBlankNodeRef<'a>>,
) -> Result<Shape<'a>, ShaclError> {
    // Parse the path
    let path = if let Some(path_obj) = graph.object_for_subject_predicate(node, sh::PATH) {
        parse_path(graph, path_obj)?
    } else {
        // No path means this is a node constraint, not a property constraint
        return Err(ShaclError::Parse(
            "Property shape must have sh:path".to_string(),
        ));
    };

    let severity = parse_severity(graph, node, parent_severity);

    // Parse constraints
    let constraints = parse_all_constraints(graph, node, true)?;

    // Create property shape
    let mut prop_shape = Shape::property_shape(node, path, severity);

    // Add all constraints
    for constraint in constraints {
        prop_shape = prop_shape.add_constraint(constraint);
    }

    prop_shape = apply_common_shape_properties(graph, node, parent, prop_shape);

    // Parse nested property shapes (sh:property on property shapes)
    for nested_prop_shape in parse_nested_property_shapes(graph, node, severity, Some(node)) {
        prop_shape = prop_shape.add_property_shape(nested_prop_shape);
    }

    Ok(prop_shape)
}

/// Parse all constraints from a shape node by calling individual constraint parsers
fn parse_all_constraints<'a>(
    graph: &'a Graph,
    node: NamedOrBlankNodeRef<'a>,
    is_property_shape: bool,
) -> Result<Vec<Constraint<'a>>, ShaclError> {
    let mut constraints = Vec::new();

    // Call each constraint parser in order
    constraints.extend(constraints::class::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::datatype::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::node_kind::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::min_count::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::max_count::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::min_length::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::max_length::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::pattern::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::min_inclusive::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::max_inclusive::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::min_exclusive::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::max_exclusive::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::language_in::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::unique_lang::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::equals::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::disjoint::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::less_than::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::less_than_or_equals::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::has_value::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::sh_in::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::sh_node::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::qualified_value_shape::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::sh_and::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::sh_or::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::sh_xone::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::sh_not::parser().parse_constraint(node, graph)?);
    constraints.extend(constraints::sparql::parse_sparql_constraints(
        graph,
        node,
        is_property_shape,
    )?);

    Ok(constraints)
}
