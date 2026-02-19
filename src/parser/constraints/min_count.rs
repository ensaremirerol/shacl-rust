use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::MinCountConstraint, parser::constraint_parser_trait::ConstraintParserTrait,
    sh, utils::get_integer_value, Constraint, ShaclError,
};

struct SHMinCountConstraintParser;

impl ConstraintParserTrait for SHMinCountConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        get_integer_value(graph, shape_node, sh::MIN_COUNT)
            .map(|v| Constraint::MinCount(MinCountConstraint(v)))
            .map(Ok)
            .into_iter()
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHMinCountConstraintParser
}
