use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::HasValueConstraint, parser::constraint_parser_trait::ConstraintParserTrait,
    sh, Constraint, ShaclError,
};

struct SHHasValueConstraintParser;

impl ConstraintParserTrait for SHHasValueConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .objects_for_subject_predicate(shape_node, sh::HAS_VALUE)
            .map(|v| Constraint::HasValue(HasValueConstraint(v)))
            .map(Ok)
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHHasValueConstraintParser
}
