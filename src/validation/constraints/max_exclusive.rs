use oxigraph::model::TermRef;

use crate::{
    core::{constraints::MaxExclusiveConstraint, path::Path, shape::Shape},
    utils,
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for MaxExclusiveConstraint<'a> {
    fn validate(
        &'a self,
        _validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();

        for &value_node in value_nodes {
            if !utils::compare_values(value_node, self.0, |cmp| cmp < 0) {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!("Value {} is not less than {}", value_node, self.0))
                    .component(sh::MAX_EXCLUSIVE_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:maxExclusive {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
