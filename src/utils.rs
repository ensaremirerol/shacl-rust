use oxigraph::model::{vocab::rdfs, Graph, NamedNodeRef, NamedOrBlankNodeRef, TermRef};
use regex::Regex;

use crate::{core::constraints::NodeKind, vocab::sh};

pub fn is_subclass_of(
    node: NamedOrBlankNodeRef,
    class: NamedOrBlankNodeRef,
    graph: &oxigraph::model::Graph,
) -> bool {
    let mut visited = std::collections::HashSet::new();
    let mut to_visit = vec![node];

    while let Some(current) = to_visit.pop() {
        if current == class {
            return true;
        }
        if visited.insert(current) {
            let objects = graph.objects_for_subject_predicate(current, rdfs::SUB_CLASS_OF);
            to_visit.extend(objects.filter_map(|o| match o {
                TermRef::NamedNode(nn) => Some(NamedOrBlankNodeRef::from(nn)),
                TermRef::BlankNode(bn) => Some(NamedOrBlankNodeRef::from(bn)),
                _ => None,
            }));
        }
    }
    false
}

pub fn collect_all_superclasses<'a>(
    node: NamedOrBlankNodeRef<'a>,
    graph: &'a oxigraph::model::Graph,
) -> std::collections::HashSet<NamedNodeRef<'a>> {
    let mut visited = std::collections::HashSet::new();
    let mut to_visit = vec![node];

    while let Some(current) = to_visit.pop() {
        if visited.insert(current) {
            let objects = graph.objects_for_subject_predicate(current, rdfs::SUB_CLASS_OF);
            to_visit.extend(objects.filter_map(|o| match o {
                TermRef::NamedNode(nn) => Some(NamedOrBlankNodeRef::from(nn)),
                TermRef::BlankNode(bn) => Some(NamedOrBlankNodeRef::from(bn)),
                _ => None,
            }));
        }
    }
    visited
        .into_iter()
        .filter_map(|n| match n {
            NamedOrBlankNodeRef::NamedNode(nn) => Some(nn),
            _ => None,
        })
        .collect()
}

pub fn collect_all_subclasses<'a>(
    node: NamedOrBlankNodeRef<'a>,
    graph: &'a oxigraph::model::Graph,
) -> std::collections::HashSet<NamedNodeRef<'a>> {
    let mut visited = std::collections::HashSet::new();
    let mut to_visit = vec![node];

    while let Some(current) = to_visit.pop() {
        if visited.insert(current) {
            to_visit.extend(graph.subjects_for_predicate_object(rdfs::SUB_CLASS_OF, current));
        }
    }
    visited
        .into_iter()
        .filter_map(|n| match n {
            NamedOrBlankNodeRef::NamedNode(nn) => Some(nn),
            _ => None,
        })
        .collect()
}

pub fn is_subproperty_of<'a>(
    node: NamedOrBlankNodeRef<'a>,
    property: NamedOrBlankNodeRef<'a>,
    graph: &'a oxigraph::model::Graph,
) -> bool {
    let mut visited = std::collections::HashSet::new();
    let mut to_visit = vec![node];

    while let Some(current) = to_visit.pop() {
        if current == property {
            return true;
        }
        if visited.insert(current) {
            let objects = graph.objects_for_subject_predicate(current, rdfs::SUB_PROPERTY_OF);
            to_visit.extend(objects.filter_map(|o| match o {
                TermRef::NamedNode(nn) => Some(NamedOrBlankNodeRef::from(nn)),
                TermRef::BlankNode(bn) => Some(NamedOrBlankNodeRef::from(bn)),
                _ => None,
            }));
        }
    }
    false
}

pub fn collect_all_superproperties<'a>(
    node: NamedNodeRef<'a>,
    graph: &'a oxigraph::model::Graph,
) -> std::collections::HashSet<NamedNodeRef<'a>> {
    let mut visited = std::collections::HashSet::new();
    let mut to_visit = vec![node];

    while let Some(current) = to_visit.pop() {
        if visited.insert(current) {
            let objects = graph.objects_for_subject_predicate(current, rdfs::SUB_PROPERTY_OF);
            to_visit.extend(objects.filter_map(|o| match o {
                TermRef::NamedNode(nn) => Some(nn),
                _ => None,
            }));
        }
    }
    visited
}

pub fn collect_all_subproperties<'a>(
    node: NamedNodeRef<'a>,
    graph: &'a oxigraph::model::Graph,
) -> std::collections::HashSet<NamedNodeRef<'a>> {
    let mut visited = std::collections::HashSet::new();
    let mut to_visit = vec![node];

    while let Some(current) = to_visit.pop() {
        if visited.insert(current) {
            to_visit.extend(
                graph
                    .subjects_for_predicate_object(rdfs::SUB_PROPERTY_OF, current)
                    .filter_map(|s| match s {
                        NamedOrBlankNodeRef::NamedNode(nn) => Some(nn),
                        _ => None,
                    }),
            );
        }
    }
    visited
}

/// Parse an RDF list into a vector of terms
pub fn parse_rdf_list<'a>(
    graph: &'a Graph,
    list_node: NamedOrBlankNodeRef<'a>,
) -> Vec<TermRef<'a>> {
    let mut result = Vec::new();
    let mut current = list_node;

    loop {
        // Check if we've reached rdf:nil
        if let NamedOrBlankNodeRef::NamedNode(nn) = current {
            if nn == oxigraph::model::vocab::rdf::NIL {
                break;
            }
        }

        // Get rdf:first
        if let Some(first) =
            graph.object_for_subject_predicate(current, oxigraph::model::vocab::rdf::FIRST)
        {
            result.push(first);
        }

        // Get rdf:rest
        if let Some(rest) =
            graph.object_for_subject_predicate(current, oxigraph::model::vocab::rdf::REST)
        {
            match rest {
                TermRef::NamedNode(nn) => current = NamedOrBlankNodeRef::NamedNode(nn),
                TermRef::BlankNode(bn) => current = NamedOrBlankNodeRef::BlankNode(bn),
                _ => break,
            }
        } else {
            break;
        }
    }

    result
}

/// Parse a node kind from a term
pub fn parse_node_kind(term: TermRef) -> Option<NodeKind> {
    use crate::vocab::sh;

    match term {
        TermRef::NamedNode(nn) => {
            if nn == sh::BLANK_NODE {
                Some(NodeKind::BlankNode)
            } else if nn == sh::IRI {
                Some(NodeKind::IRI)
            } else if nn == sh::LITERAL {
                Some(NodeKind::Literal)
            } else if nn == sh::BLANK_NODE_OR_IRI {
                Some(NodeKind::BlankNodeOrIRI)
            } else if nn == sh::BLANK_NODE_OR_LITERAL {
                Some(NodeKind::BlankNodeOrLiteral)
            } else if nn == sh::IRI_OR_LITERAL {
                Some(NodeKind::IRIOrLiteral)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Get a string value from a property
pub fn get_string_value(
    graph: &Graph,
    subject: NamedOrBlankNodeRef,
    predicate: NamedNodeRef,
) -> Option<String> {
    graph
        .object_for_subject_predicate(subject, predicate)
        .and_then(|term| match term {
            TermRef::Literal(lit) => Some(lit.value().to_string()),
            TermRef::NamedNode(nn) => Some(nn.to_string()),
            _ => None,
        })
}

/// Get all string values from a property
pub fn get_all_string_values(
    graph: &Graph,
    subject: NamedOrBlankNodeRef,
    predicate: NamedNodeRef,
) -> Vec<String> {
    graph
        .objects_for_subject_predicate(subject, predicate)
        .filter_map(|term| match term {
            TermRef::Literal(lit) => Some(lit.value().to_string()),
            _ => None,
        })
        .collect()
}

/// Get a boolean value from a property
pub fn get_boolean_value(
    graph: &Graph,
    subject: NamedOrBlankNodeRef,
    predicate: NamedNodeRef,
) -> Option<bool> {
    graph
        .object_for_subject_predicate(subject, predicate)
        .and_then(|term| match term {
            TermRef::Literal(lit) => lit.value().parse::<bool>().ok(),
            _ => None,
        })
}

/// Get an integer value from a property
pub fn get_integer_value(
    graph: &Graph,
    subject: NamedOrBlankNodeRef,
    predicate: NamedNodeRef,
) -> Option<i32> {
    graph
        .object_for_subject_predicate(subject, predicate)
        .and_then(|term| match term {
            TermRef::Literal(lit) => lit.value().parse::<i32>().ok(),
            _ => None,
        })
}
/// Convert a TermRef to NamedOrBlankNodeRef, filtering out literals
pub fn term_to_named_or_blank(term: TermRef) -> Option<NamedOrBlankNodeRef> {
    match term {
        TermRef::NamedNode(n) => Some(n.into()),
        TermRef::BlankNode(b) => Some(b.into()),
        TermRef::Literal(_) => None,
    }
}

pub fn local_name_from_iri(iri: &str) -> Option<String> {
    iri.rsplit(['#', '/'])
        .next()
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

pub fn parse_shacl_prefixes<'a>(
    graph: &'a Graph,
    executable: NamedOrBlankNodeRef<'a>,
) -> Vec<(String, String)> {
    let mut prefixes = Vec::new();

    for prefixes_term in graph.objects_for_subject_predicate(executable, sh::PREFIXES) {
        let Some(prefixes_node) = term_to_named_or_blank(prefixes_term) else {
            continue;
        };

        for decl_term in graph.objects_for_subject_predicate(prefixes_node, sh::DECLARE) {
            let Some(decl_node) = term_to_named_or_blank(decl_term) else {
                continue;
            };

            let prefix = graph
                .object_for_subject_predicate(decl_node, sh::PREFIX)
                .and_then(|t| match t {
                    TermRef::Literal(lit) => Some(lit.value().to_string()),
                    _ => None,
                });

            let namespace = graph
                .object_for_subject_predicate(decl_node, sh::NAMESPACE)
                .and_then(|t| match t {
                    TermRef::Literal(lit) => Some(lit.value().to_string()),
                    _ => None,
                });

            if let (Some(p), Some(ns)) = (prefix, namespace) {
                prefixes.push((p, ns));
            }
        }
    }

    prefixes
}

pub fn rewrite_this_binding_query(query: &str, this_term: &str) -> String {
    let normalized = query.replace("$this", "?this");
    let where_re = Regex::new(r"(?i)WHERE\s*\{").unwrap();
    let bind_clause = format!(" BIND ({} AS ?this) .", this_term);
    where_re
        .replace_all(&normalized, |caps: &regex::Captures<'_>| {
            format!("{}{}", &caps[0], bind_clause)
        })
        .to_string()
}

/// Extract direct IRI predicates from a path
pub fn extract_direct_predicates<'a>(
    path: &'a crate::core::path::Path<'a>,
) -> Vec<NamedNodeRef<'a>> {
    use crate::core::path::PathElement;

    let mut predicates = Vec::new();
    let elements = path.get_elements();

    for element in elements {
        match element {
            PathElement::Iri(iri) => {
                predicates.push(*iri);
            }
            PathElement::Inverse(_) => {
                // For closed validation, inverse paths are not typically considered
                // as they represent incoming properties, not outgoing
            }
            PathElement::Alternative(alternatives) => {
                // For alternatives, extract all direct IRIs
                for alt_element in alternatives {
                    if let PathElement::Iri(iri) = alt_element {
                        predicates.push(*iri);
                    }
                }
            }
            _ => {}
        }
    }

    predicates
}

/// A literal pre-parsed for value comparisons, so constant bounds are parsed
/// once instead of once per compared value.
#[derive(Debug, Clone, PartialEq)]
pub enum ComparableValue<'a> {
    Number(f64),
    Text(&'a str),
}

/// Prepares a term for comparison. Returns `None` for non-literals, which
/// never compare equal, less, or greater to anything.
pub fn to_comparable(term: TermRef<'_>) -> Option<ComparableValue<'_>> {
    match term {
        TermRef::Literal(lit) => Some(match lit.value().parse::<f64>() {
            Ok(n) => ComparableValue::Number(n),
            Err(_) => ComparableValue::Text(lit.value()),
        }),
        _ => None,
    }
}

/// Compares two pre-parsed literals. Numbers compare numerically, text
/// compares lexically, and mixed number/text is incompatible (always false).
pub fn compare_comparables<F>(a: &ComparableValue, b: &ComparableValue, predicate: F) -> bool
where
    F: Fn(i32) -> bool,
{
    match (a, b) {
        (ComparableValue::Number(na), ComparableValue::Number(nb)) => {
            if na < nb {
                predicate(-1)
            } else if na > nb {
                predicate(1)
            } else {
                predicate(0)
            }
        }
        (ComparableValue::Text(ta), ComparableValue::Text(tb)) => predicate(match ta.cmp(tb) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }),
        _ => false,
    }
}

/// Compare two terms using a predicate function
pub fn compare_values<F>(a: TermRef, b: TermRef, predicate: F) -> bool
where
    F: Fn(i32) -> bool,
{
    match (to_comparable(a), to_comparable(b)) {
        (Some(ca), Some(cb)) => compare_comparables(&ca, &cb, predicate),
        _ => false,
    }
}

