use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::MaxLengthConstraint, parser::constraint_parser_trait::ConstraintParserTrait,
    sh, utils::get_integer_value, Constraint, ShaclError,
};

struct SHMaxLengthConstraintParser;

impl ConstraintParserTrait for SHMaxLengthConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        get_integer_value(graph, shape_node, sh::MAX_LENGTH)
            .map(|v| Constraint::MaxLength(MaxLengthConstraint(v)))
            .map(Ok)
            .into_iter()
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHMaxLengthConstraintParser
}
