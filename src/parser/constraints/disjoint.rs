use oxigraph::model::{Graph, NamedOrBlankNodeRef};
use log::warn;

use crate::{
    core::constraints::DisjointConstraint, parser::constraint_parser_trait::ConstraintParserTrait,
    sh, Constraint, ShaclError,
};

struct SHDisjointConstraintParser;

impl ConstraintParserTrait for SHDisjointConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .objects_for_subject_predicate(shape_node, sh::DISJOINT)
            .map(|path_term| crate::parser::path::parse_path(graph, path_term))
            .filter_map(|item| match item {
                Ok(path) => Some(Constraint::Disjoint(DisjointConstraint(path))),
                Err(err) => {
                    warn!(
                        "Error parsing sh:disjoint constraint on shape {}: {}",
                        shape_node, err
                    );
                    None
                }
            })
            .map(Ok)
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHDisjointConstraintParser
}
