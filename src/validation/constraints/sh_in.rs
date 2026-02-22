use oxigraph::model::TermRef;

use crate::{
    core::{constraints::InConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for InConstraint<'a> {
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
            if !self.0.contains(&value_node) {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message("Value is not in the allowed list")
                    .component(sh::IN_CONSTRAINT_COMPONENT)
                    .detail("sh:in constraint".to_string());

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
