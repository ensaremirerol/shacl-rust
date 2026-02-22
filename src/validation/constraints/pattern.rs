use oxigraph::model::TermRef;
use regex::Regex;

use crate::{
    core::{constraints::PatternConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for PatternConstraint {
    fn validate(
        &'a self,
        _validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();

        let regex_pattern = if let Some(ref f) = self.flags {
            let mut pattern_with_flags = String::from("(?");
            if f.contains('i') {
                pattern_with_flags.push('i');
            }
            if f.contains('m') {
                pattern_with_flags.push('m');
            }
            if f.contains('s') {
                pattern_with_flags.push('s');
            }
            pattern_with_flags.push(')');
            pattern_with_flags.push_str(&self.pattern);
            pattern_with_flags
        } else {
            self.pattern.clone()
        };

        let Ok(re) = Regex::new(&regex_pattern) else {
            return Ok(violations);
        };

        for &value_node in value_nodes {
            let TermRef::Literal(lit) = value_node else {
                continue;
            };

            if !re.is_match(lit.value()) {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!("Value does not match pattern: {}", self.pattern))
                    .component(sh::PATTERN_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:pattern {}", self.pattern));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
