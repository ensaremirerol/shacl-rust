use oxigraph::model::TermRef;

use crate::{
    core::{constraints::MaxLengthConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for MaxLengthConstraint {
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
            let TermRef::Literal(lit) = value_node else {
                continue;
            };

            let len = lit.value().len() as i32;
            if len > self.0 {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!("String length {} exceeds maximum {}", len, self.0))
                    .component(sh::MAX_LENGTH_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:maxLength {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
