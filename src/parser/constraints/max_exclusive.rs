use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::MaxExclusiveConstraint,
    parser::constraint_parser_trait::ConstraintParserTrait, sh, Constraint, ShaclError,
};

struct SHMaxExclusiveConstraintParser;

impl ConstraintParserTrait for SHMaxExclusiveConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .object_for_subject_predicate(shape_node, sh::MAX_EXCLUSIVE)
            .into_iter()
            .map(|v| Constraint::MaxExclusive(MaxExclusiveConstraint(v)))
            .map(Ok)
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHMaxExclusiveConstraintParser
}
