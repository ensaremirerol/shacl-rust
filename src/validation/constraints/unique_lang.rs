use oxigraph::model::{Graph, TermRef};
use std::collections::HashMap;

use crate::{
    core::{constraints::UniqueLangConstraint, path::Path, shape::Shape},
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for UniqueLangConstraint {
    fn validate(
        &'a self,
        _data_graph: &'a Graph,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Vec<ValidationResult<'a>> {
        let mut violations = Vec::new();
        let mut seen_languages = HashMap::new();

        for &value_node in value_nodes {
            if let TermRef::Literal(lit) = value_node {
                if let Some(lang) = lit.language() {
                    if let Some(first_value) = seen_languages.get(lang) {
                        let builder = ViolationBuilder::new(focus_node)
                            .value(value_node)
                            .message(format!(
                                "Duplicate language tag '{}' (first seen: {})",
                                lang, first_value
                            ))
                            .component(sh::UNIQUE_LANG_CONSTRAINT_COMPONENT)
                            .detail("sh:uniqueLang true".to_string());

                        violations.push(shape.build_validation_result(builder));
                    } else {
                        seen_languages.insert(lang, lit.value());
                    }
                }
            }
        }

        violations
    }
}
