use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{
    core::constraints::UniqueLangConstraint,
    parser::constraint_parser_trait::ConstraintParserTrait, sh, utils::get_boolean_value,
    Constraint, ShaclError,
};

struct SHUniqueLangConstraintParser;

impl ConstraintParserTrait for SHUniqueLangConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        if let Some(unique_lang) = get_boolean_value(graph, shape_node, sh::UNIQUE_LANG) {
            Ok(vec![Constraint::UniqueLang(UniqueLangConstraint(
                unique_lang,
            ))])
        } else {
            Ok(vec![])
        }
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHUniqueLangConstraintParser
}
