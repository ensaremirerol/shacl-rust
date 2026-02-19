//! SHACL path parsing.

use oxigraph::model::{vocab::rdf, Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::path::{Path, PathElement},
    err::ShaclError,
    utils::parse_rdf_list,
    vocab::sh,
};

/// Parses a SHACL path.
pub fn parse_path<'a>(graph: &'a Graph, path_term: TermRef<'a>) -> Result<Path<'a>, ShaclError> {
    let mut path = Path::new();

    match path_term {
        TermRef::NamedNode(iri) => {
            path = path.add_element(PathElement::Iri(iri));
        }
        TermRef::BlankNode(bn) => {
            let node = NamedOrBlankNodeRef::from(bn);
            path = path.set_source(node);

            if graph
                .object_for_subject_predicate(node, rdf::FIRST)
                .is_some()
            {
                let list_items = parse_rdf_list(graph, node);
                for item in list_items {
                    match item {
                        TermRef::NamedNode(iri) => {
                            path = path.add_element(PathElement::Iri(iri));
                        }
                        TermRef::BlankNode(bn) => {
                            let element = parse_path_element(graph, NamedOrBlankNodeRef::from(bn))?;
                            path = path.add_element(element);
                        }
                        _ => {
                            return Err(ShaclError::Parse(
                                "Invalid path element in sequence".to_string(),
                            ))
                        }
                    }
                }
            } else {
                let element = parse_path_element(graph, node)?;
                path = path.add_element(element);
            }
        }
        _ => {
            return Err(ShaclError::Parse(
                "Invalid path: must be IRI or blank node".to_string(),
            ))
        }
    }

    Ok(path)
}

/// Parses one path element.
fn parse_path_element<'a>(
    graph: &'a Graph,
    node: NamedOrBlankNodeRef<'a>,
) -> Result<PathElement<'a>, ShaclError> {
    if let Some(TermRef::NamedNode(iri)) =
        graph.object_for_subject_predicate(node, sh::INVERSE_PATH)
    {
        return Ok(PathElement::Inverse(iri));
    }

    if let Some(alt_obj) = graph.object_for_subject_predicate(node, sh::ALTERNATIVE_PATH) {
        let list_node = match alt_obj {
            TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
            TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
            _ => return Err(ShaclError::Parse("Invalid alternative path".to_string())),
        };
        let list_items = parse_rdf_list(graph, list_node);
        let mut alternatives = Vec::new();
        for item in list_items {
            match item {
                TermRef::NamedNode(iri) => {
                    alternatives.push(PathElement::Iri(iri));
                }
                TermRef::BlankNode(bn) => {
                    alternatives.push(parse_path_element(graph, NamedOrBlankNodeRef::from(bn))?);
                }
                _ => {}
            }
        }
        return Ok(PathElement::Alternative(alternatives));
    }

    if let Some(zero_or_more_obj) = graph.object_for_subject_predicate(node, sh::ZERO_OR_MORE_PATH)
    {
        let inner_elem = match zero_or_more_obj {
            TermRef::NamedNode(iri) => PathElement::Iri(iri),
            TermRef::BlankNode(bn) => parse_path_element(graph, NamedOrBlankNodeRef::from(bn))?,
            _ => {
                return Err(ShaclError::Parse(
                    "Invalid path in sh:zeroOrMorePath".to_string(),
                ))
            }
        };
        return Ok(PathElement::ZeroOrMore(Box::new(inner_elem)));
    }

    if let Some(one_or_more_obj) = graph.object_for_subject_predicate(node, sh::ONE_OR_MORE_PATH) {
        let inner_elem = match one_or_more_obj {
            TermRef::NamedNode(iri) => PathElement::Iri(iri),
            TermRef::BlankNode(bn) => parse_path_element(graph, NamedOrBlankNodeRef::from(bn))?,
            _ => {
                return Err(ShaclError::Parse(
                    "Invalid path in sh:oneOrMorePath".to_string(),
                ))
            }
        };
        return Ok(PathElement::OneOrMore(Box::new(inner_elem)));
    }

    if let Some(zero_or_one_obj) = graph.object_for_subject_predicate(node, sh::ZERO_OR_ONE_PATH) {
        let inner_elem = match zero_or_one_obj {
            TermRef::NamedNode(iri) => PathElement::Iri(iri),
            TermRef::BlankNode(bn) => parse_path_element(graph, NamedOrBlankNodeRef::from(bn))?,
            _ => {
                return Err(ShaclError::Parse(
                    "Invalid path in sh:zeroOrOnePath".to_string(),
                ))
            }
        };
        return Ok(PathElement::ZeroOrOne(Box::new(inner_elem)));
    }

    Err(ShaclError::Parse(
        "Could not parse path element".to_string(),
    ))
}
