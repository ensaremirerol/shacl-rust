use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::constraints::NotConstraint, parser::constraint_parser_trait::ConstraintParserTrait, sh,
    Constraint, ShaclError,
};

struct SHNotConstraintParser;

impl ConstraintParserTrait for SHNotConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        let mut constraints = Vec::new();

        if let Some(not_obj) = graph.object_for_subject_predicate(shape_node, sh::NOT) {
            let not_node = match not_obj {
                TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
                TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
                _ => return Ok(constraints),
            };

            if let Ok(not_shape) = super::super::parse_shape(graph, not_node, Some(shape_node)) {
                constraints.push(Constraint::Not(NotConstraint(Box::new(not_shape))));
            }
        }

        Ok(constraints)
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHNotConstraintParser
}
