use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::LanguageInConstraint, path::Path, shape::Shape},
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for LanguageInConstraint {
    fn validate(
        &'a self,
        _data_graph: &'a Graph,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Vec<ValidationResult<'a>> {
        let mut violations = Vec::new();
        let allowed_languages = self.0.join(", ");

        for &value_node in value_nodes {
            if let TermRef::Literal(lit) = value_node {
                if let Some(lang) = lit.language() {
                    if !self.0.iter().any(|l| l.eq_ignore_ascii_case(lang)) {
                        let builder = ViolationBuilder::new(focus_node)
                            .value(value_node)
                            .message(format!("Language '{}' not in allowed list", lang))
                            .component(sh::LANGUAGE_IN_CONSTRAINT_COMPONENT)
                            .detail(format!("sh:languageIn [{}]", allowed_languages));

                        violations.push(shape.build_validation_result(builder));
                    }
                } else {
                    let builder = ViolationBuilder::new(focus_node)
                        .value(value_node)
                        .message("Value has no language tag")
                        .component(sh::LANGUAGE_IN_CONSTRAINT_COMPONENT)
                        .detail(format!("sh:languageIn [{}]", allowed_languages));

                    violations.push(shape.build_validation_result(builder));
                }
            }
        }

        violations
    }
}
