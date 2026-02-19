use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::MaxCountConstraint, parser::constraint_parser_trait::ConstraintParserTrait,
    sh, utils::get_integer_value, Constraint, ShaclError,
};

struct SHMaxCountConstraintParser;

impl ConstraintParserTrait for SHMaxCountConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        get_integer_value(graph, shape_node, sh::MAX_COUNT)
            .map(|v| Constraint::MaxCount(MaxCountConstraint(v)))
            .map(Ok)
            .into_iter()
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHMaxCountConstraintParser
}
