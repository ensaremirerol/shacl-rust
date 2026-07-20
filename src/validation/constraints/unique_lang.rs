use oxigraph::model::TermRef;
use std::collections::HashMap;

use crate::{
    core::{constraints::UniqueLangConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for UniqueLangConstraint {
    fn validate(
        &'a self,
        _validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();

        if !self.0 {
            return Ok(violations);
        }

        // Per spec: one validation result per language tag used by more than
        // one value node (not one per offending value).
        let mut lang_counts: HashMap<String, usize> = HashMap::new();
        for &value_node in value_nodes {
            if let TermRef::Literal(lit) = value_node {
                if let Some(lang) = lit.language() {
                    *lang_counts.entry(lang.to_ascii_lowercase()).or_insert(0) += 1;
                }
            }
        }

        let mut duplicated: Vec<&String> = lang_counts
            .iter()
            .filter(|(_, count)| **count > 1)
            .map(|(lang, _)| lang)
            .collect();
        duplicated.sort();

        for lang in duplicated {
            let builder = ViolationBuilder::new(focus_node)
                .message(format!("Language \"{}\" is used more than once", lang))
                .component(sh::UNIQUE_LANG_CONSTRAINT_COMPONENT)
                .detail("sh:uniqueLang true".to_string());

            violations.push(shape.build_validation_result(builder));
        }

        Ok(violations)
    }
}
