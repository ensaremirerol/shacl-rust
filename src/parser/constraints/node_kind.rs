use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::constraints::{NodeKind, NodeKindConstraint},
    parser::constraint_parser_trait::ConstraintParserTrait,
    sh, Constraint, ShaclError,
};

struct SHNodeKindConstraintParser;

fn parse_node_kind(term: TermRef) -> Option<NodeKind> {
    match term {
        TermRef::NamedNode(nn) if nn == sh::IRI => Some(NodeKind::IRI),
        TermRef::NamedNode(nn) if nn == sh::BLANK_NODE => Some(NodeKind::BlankNode),
        TermRef::NamedNode(nn) if nn == sh::LITERAL => Some(NodeKind::Literal),
        TermRef::NamedNode(nn) if nn == sh::BLANK_NODE_OR_IRI => Some(NodeKind::BlankNodeOrIRI),
        TermRef::NamedNode(nn) if nn == sh::BLANK_NODE_OR_LITERAL => {
            Some(NodeKind::BlankNodeOrLiteral)
        }
        TermRef::NamedNode(nn) if nn == sh::IRI_OR_LITERAL => Some(NodeKind::IRIOrLiteral),
        _ => None,
    }
}

impl ConstraintParserTrait for SHNodeKindConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .object_for_subject_predicate(shape_node, sh::NODE_KIND_PROPERTY)
            .and_then(parse_node_kind)
            .map(|nk| Constraint::NodeKind(NodeKindConstraint(nk)))
            .map(Ok)
            .into_iter()
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHNodeKindConstraintParser
}
