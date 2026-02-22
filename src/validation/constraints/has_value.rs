use oxigraph::model::TermRef;

use crate::{
    core::{constraints::HasValueConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for HasValueConstraint<'a> {
    fn validate(
        &'a self,
        _validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        if !value_nodes.contains(&self.0) {
            let builder = ViolationBuilder::new(focus_node)
                .message(format!("Required value {} is not present", self.0))
                .component(sh::HAS_VALUE_CONSTRAINT_COMPONENT)
                .detail(format!("sh:hasValue {}", self.0));

            Ok(vec![shape.build_validation_result(builder)])
        } else {
            Ok(Vec::new())
        }
    }
}
