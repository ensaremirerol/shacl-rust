use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::constraints::InConstraint, parser::constraint_parser_trait::ConstraintParserTrait, sh,
    utils::parse_rdf_list, Constraint, ShaclError,
};

struct SHInConstraintParser;

impl ConstraintParserTrait for SHInConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        if let Some(in_node) = graph.object_for_subject_predicate(shape_node, sh::IN) {
            if let Some(in_node) = match in_node {
                TermRef::NamedNode(nn) => Some(NamedOrBlankNodeRef::NamedNode(nn)),
                TermRef::BlankNode(bn) => Some(NamedOrBlankNodeRef::BlankNode(bn)),
                _ => None,
            } {
                let values = parse_rdf_list(graph, in_node);
                if !values.is_empty() {
                    return Ok(vec![Constraint::In(InConstraint(values))]);
                }
            }
        }
        Ok(vec![])
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHInConstraintParser
}
