use oxigraph::model::TermRef;

use crate::{
    core::{constraints::XoneConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for XoneConstraint<'a> {
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
            let mut conforming_count = 0;
            let mut conforming_shapes = Vec::new();
            let mut all_nested_results = Vec::new();

            for nested_shape in &self.0 {
                let mut nested_report = crate::validation::ValidationReport::new();
                nested_shape.validate_focus_node(
                    validation_dataset,
                    value_node,
                    &mut nested_report,
                );

                if *nested_report.get_conforms() {
                    conforming_count += 1;
                    conforming_shapes.push(nested_shape.node.to_string());
                } else {
                    nested_report
                        .get_results()
                        .iter()
                        .for_each(|r| all_nested_results.push(r.clone()));
                }
            }

            if conforming_count != 1 {
                let message = if conforming_count == 0 {
                    "Value does not conform to exactly one shape in sh:xone (conforms to none)"
                        .to_string()
                } else {
                    format!(
                        "Value does not conform to exactly one shape in sh:xone (conforms to {} shapes: {})",
                        conforming_count,
                        conforming_shapes.join(", ")
                    )
                };

                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(message)
                    .component(sh::XONE_CONSTRAINT_COMPONENT)
                    .detail(format!(
                        "sh:xone with {} shapes, {} conforming",
                        self.0.len(),
                        conforming_count
                    ));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
