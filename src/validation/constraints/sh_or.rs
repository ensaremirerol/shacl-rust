use oxigraph::model::TermRef;

use crate::{
    core::{constraints::OrConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for OrConstraint<'a> {
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
            let mut all_nested_results = Vec::new();
            let mut any_conforms = false;

            for nested_shape in &self.0 {
                let mut nested_report = crate::validation::ValidationReport::new();
                nested_shape.validate_focus_node(
                    validation_dataset,
                    value_node,
                    &mut nested_report,
                );

                if nested_report.conforms {
                    any_conforms = true;
                    break;
                } else {
                    all_nested_results.extend(nested_report.results);
                }
            }

            if !any_conforms {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message("Value does not conform to any shape in sh:or")
                    .component(sh::OR_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:or with {} shapes", self.0.len()))
                    .details(all_nested_results);

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
