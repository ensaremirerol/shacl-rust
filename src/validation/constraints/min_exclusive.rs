use oxigraph::model::TermRef;

use crate::{
    core::{constraints::MinExclusiveConstraint, path::Path, shape::Shape},
    utils,
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for MinExclusiveConstraint<'a> {
    fn validate(
        &'a self,
        _validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();
        // Parsed once; `None` (non-literal bound) makes every value violate,
        // matching compare_values semantics.
        let bound = utils::to_comparable(self.0);

        for &value_node in value_nodes {
            let conforms = match (&bound, utils::to_comparable(value_node)) {
                (Some(bound), Some(value)) => {
                    utils::compare_comparables(&value, bound, |cmp| cmp > 0)
                }
                _ => false,
            };
            if !conforms {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!(
                        "Value {} is not greater than {}",
                        value_node, self.0
                    ))
                    .component(sh::MIN_EXCLUSIVE_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:minExclusive {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
