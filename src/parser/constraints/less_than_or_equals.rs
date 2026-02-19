use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::LessThanOrEqualsConstraint,
    parser::constraint_parser_trait::ConstraintParserTrait, sh, Constraint, ShaclError,
};

struct SHLessThanOrEqualsConstraintParser;

impl ConstraintParserTrait for SHLessThanOrEqualsConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .objects_for_subject_predicate(shape_node, sh::LESS_THAN_OR_EQUALS)
            .map(|path_term| {
                crate::parser::path::parse_path(graph, path_term)
                    .map(|p| Constraint::LessThanOrEquals(LessThanOrEqualsConstraint(p)))
            })
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHLessThanOrEqualsConstraintParser
}
