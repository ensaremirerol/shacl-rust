use oxigraph::model::TermRef;

use crate::{
    core::{constraints::NotConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for NotConstraint<'a> {
    fn validate(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();

        for &value_node in value_nodes {
            let mut nested_report = crate::validation::ValidationReport::new();
            self.0
                .validate_focus_node(validation_dataset, value_node, &mut nested_report);

            if *nested_report.get_conforms() {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message("Value conforms to shape in sh:not (should not conform)")
                    .component(sh::NOT_CONSTRAINT_COMPONENT)
                    .detail(format!(
                        "sh:not constraint referencing shape {}",
                        self.0.node
                    ))
                    .trace_entry("sh:not validation");

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
