use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::LessThanConstraint, parser::constraint_parser_trait::ConstraintParserTrait,
    sh, Constraint, ShaclError,
};

struct SHLessThanConstraintParser;

impl ConstraintParserTrait for SHLessThanConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .objects_for_subject_predicate(shape_node, sh::LESS_THAN)
            .map(|path_term| {
                crate::parser::path::parse_path(graph, path_term)
                    .map(|p| Constraint::LessThan(LessThanConstraint(p)))
            })
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHLessThanConstraintParser
}
