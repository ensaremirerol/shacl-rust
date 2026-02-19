use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::AndConstraint, path::Path, shape::Shape},
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for AndConstraint<'a> {
    fn validate(
        &'a self,
        data_graph: &'a Graph,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Vec<ValidationResult<'a>> {
        let mut violations = Vec::new();

        for &value_node in value_nodes {
            let mut failed_shapes = Vec::new();
            let mut all_nested_results = Vec::new();

            for nested_shape in &self.0 {
                let mut nested_report = crate::validation::ValidationReport::new();
                nested_shape.validate_focus_node(data_graph, value_node, &mut nested_report);

                if !nested_report.conforms {
                    failed_shapes.push(nested_shape.node.to_string());
                    all_nested_results.extend(nested_report.results);
                }
            }

            if !failed_shapes.is_empty() {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!(
                        "Value does not conform to all shapes in sh:and (failed: {})",
                        failed_shapes.join(", ")
                    ))
                    .component(sh::AND_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:and with {} shapes", self.0.len()))
                    .trace_entry("sh:and validation")
                    .details(all_nested_results);

                violations.push(shape.build_validation_result(builder));
            }
        }

        violations
    }
}
