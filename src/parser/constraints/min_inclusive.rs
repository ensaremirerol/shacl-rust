use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::MinInclusiveConstraint,
    parser::constraint_parser_trait::ConstraintParserTrait, sh, Constraint, ShaclError,
};

struct SHMinInclusiveConstraintParser;

impl ConstraintParserTrait for SHMinInclusiveConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .object_for_subject_predicate(shape_node, sh::MIN_INCLUSIVE)
            .into_iter()
            .map(|v| Constraint::MinInclusive(MinInclusiveConstraint(v)))
            .map(Ok)
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHMinInclusiveConstraintParser
}
