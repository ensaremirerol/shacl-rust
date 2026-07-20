use oxigraph::model::TermRef;

use crate::{
    core::{constraints::LanguageInConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for LanguageInConstraint {
    fn validate(
        &'a self,
        _validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();

        // sh:languageIn uses basic language-range matching (RFC 4647): the
        // range "en" matches "en" and "en-NZ".
        fn lang_matches(lang: &str, range: &str) -> bool {
            if lang.len() == range.len() {
                return lang.eq_ignore_ascii_case(range);
            }
            lang.len() > range.len()
                && lang.as_bytes()[range.len()] == b'-'
                && lang[..range.len()].eq_ignore_ascii_case(range)
        }

        for &value_node in value_nodes {
            if let TermRef::Literal(lit) = value_node {
                if let Some(lang) = lit.language() {
                    if !self.0.iter().any(|l| lang_matches(lang, l)) {
                        let builder = ViolationBuilder::new(focus_node)
                            .value(value_node)
                            .message(format!("Language '{}' not in allowed list", lang))
                            .component(sh::LANGUAGE_IN_CONSTRAINT_COMPONENT)
                            .detail(format!("sh:languageIn [{}]", self.0.join(", ")));

                        violations.push(shape.build_validation_result(builder));
                    }
                } else {
                    let builder = ViolationBuilder::new(focus_node)
                        .value(value_node)
                        .message("Value has no language tag")
                        .component(sh::LANGUAGE_IN_CONSTRAINT_COMPONENT)
                        .detail(format!("sh:languageIn [{}]", self.0.join(", ")));

                    violations.push(shape.build_validation_result(builder));
                }
            } else {
                // Non-literal value nodes always violate sh:languageIn.
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message("Value is not a literal with a language tag")
                    .component(sh::LANGUAGE_IN_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:languageIn [{}]", self.0.join(", ")));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
