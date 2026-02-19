use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::EqualsConstraint, parser::constraint_parser_trait::ConstraintParserTrait,
    sh, Constraint, ShaclError,
};

struct SHEqualsConstraintParser;

impl ConstraintParserTrait for SHEqualsConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .objects_for_subject_predicate(shape_node, sh::EQUALS)
            .map(|path_term| {
                crate::parser::path::parse_path(graph, path_term)
                    .map(|p| Constraint::Equals(EqualsConstraint(p)))
            })
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHEqualsConstraintParser
}
