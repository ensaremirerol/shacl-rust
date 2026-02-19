use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::constraints::ClassConstraint, parser::constraint_parser_trait::ConstraintParserTrait, sh,
    Constraint, ShaclError,
};

struct SHClassConstraintParser;

impl ConstraintParserTrait for SHClassConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .objects_for_subject_predicate(shape_node, sh::CLASS)
            .filter_map(|term| match term {
                TermRef::NamedNode(nn) => Some(Constraint::Class(ClassConstraint(nn))),
                _ => None,
            })
            .map(Ok)
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHClassConstraintParser
}
