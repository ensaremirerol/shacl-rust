use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::MaxInclusiveConstraint,
    parser::constraint_parser_trait::ConstraintParserTrait, sh, Constraint, ShaclError,
};

struct SHMaxInclusiveConstraintParser;

impl ConstraintParserTrait for SHMaxInclusiveConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .object_for_subject_predicate(shape_node, sh::MAX_INCLUSIVE)
            .into_iter()
            .map(|v| Constraint::MaxInclusive(MaxInclusiveConstraint(v)))
            .map(Ok)
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHMaxInclusiveConstraintParser
}
