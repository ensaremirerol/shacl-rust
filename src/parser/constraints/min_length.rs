use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::MinLengthConstraint, parser::constraint_parser_trait::ConstraintParserTrait,
    sh, utils::get_integer_value, Constraint, ShaclError,
};

struct SHMinLengthConstraintParser;

impl ConstraintParserTrait for SHMinLengthConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        get_integer_value(graph, shape_node, sh::MIN_LENGTH)
            .map(|v| Constraint::MinLength(MinLengthConstraint(v)))
            .map(Ok)
            .into_iter()
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHMinLengthConstraintParser
}
