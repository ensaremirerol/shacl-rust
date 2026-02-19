use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::MinExclusiveConstraint,
    parser::constraint_parser_trait::ConstraintParserTrait, sh, Constraint, ShaclError,
};

struct SHMinExclusiveConstraintParser;

impl ConstraintParserTrait for SHMinExclusiveConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .object_for_subject_predicate(shape_node, sh::MIN_EXCLUSIVE)
            .into_iter()
            .map(|v| Constraint::MinExclusive(MinExclusiveConstraint(v)))
            .map(Ok)
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHMinExclusiveConstraintParser
}
