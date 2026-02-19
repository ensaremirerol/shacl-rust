//! SHACL target parsing.

use oxigraph::model::{
    vocab::{rdf, rdfs},
    Graph, NamedOrBlankNodeRef, TermRef,
};

use crate::{core::target::Target, vocab::sh};

/// Parses targets for a shape node.
pub fn parse_targets<'a>(graph: &'a Graph, node: NamedOrBlankNodeRef<'a>) -> Vec<Target<'a>> {
    let mut targets = Vec::new();

    let is_class = graph
        .objects_for_subject_predicate(node, rdf::TYPE)
        .filter_map(|term_ref| match term_ref {
            TermRef::NamedNode(nn) => Some(nn),
            _ => None,
        })
        .any(|t| t == rdfs::CLASS);

    if is_class {
        targets.push(Target::Class(node));
    }

    for obj in graph.objects_for_subject_predicate(node, sh::TARGET_CLASS) {
        let target = match obj {
            TermRef::NamedNode(nn) => Target::Class(NamedOrBlankNodeRef::NamedNode(nn)),
            TermRef::BlankNode(bn) => Target::Class(NamedOrBlankNodeRef::BlankNode(bn)),
            _ => continue,
        };
        targets.push(target);
    }

    for obj in graph.objects_for_subject_predicate(node, sh::TARGET_NODE) {
        targets.push(Target::Node(obj));
    }

    for obj in graph.objects_for_subject_predicate(node, sh::TARGET_SUBJECTS_OF) {
        if let TermRef::NamedNode(prop) = obj {
            targets.push(Target::SubjectsOf(prop));
        }
    }

    for obj in graph.objects_for_subject_predicate(node, sh::TARGET_OBJECTS_OF) {
        if let TermRef::NamedNode(prop) = obj {
            targets.push(Target::ObjectsOf(prop));
        }
    }

    for obj in graph.objects_for_subject_predicate(node, sh::TARGET) {
        match obj {
            TermRef::NamedNode(nn) => targets.push(Target::Advanced(nn.into())),
            TermRef::BlankNode(bn) => targets.push(Target::Advanced(bn.into())),
            TermRef::Literal(_) => {}
        }
    }

    targets
}
