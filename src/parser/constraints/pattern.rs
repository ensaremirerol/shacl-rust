use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::PatternConstraint, parser::constraint_parser_trait::ConstraintParserTrait,
    sh, utils::get_string_value, Constraint, ShaclError,
};

struct SHPatternConstraintParser;

impl ConstraintParserTrait for SHPatternConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        if let Some(pattern) = get_string_value(graph, shape_node, sh::PATTERN) {
            let flags = get_string_value(graph, shape_node, sh::FLAGS);
            Ok(vec![Constraint::Pattern(PatternConstraint {
                pattern,
                flags,
            })])
        } else {
            Ok(vec![])
        }
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHPatternConstraintParser
}
