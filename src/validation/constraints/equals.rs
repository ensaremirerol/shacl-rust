use oxigraph::model::TermRef;
use std::collections::HashSet;

use crate::{
    core::{constraints::EqualsConstraint, path::Path, shape::Shape},
    utils,
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for EqualsConstraint<'a> {
    fn validate(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();

        let Some(focus_as_node) = utils::term_to_named_or_blank(focus_node) else {
            return Ok(violations);
        };

        let data_graph = validation_dataset.data();

        let other_values: HashSet<TermRef<'a>> = self
            .0
            .resolve_path_for_given_node(data_graph, &focus_as_node)
            .into_iter()
            .collect();

        // Per spec: one result for each value node absent from the other
        // property's values, and one for each of the other property's values
        // absent from the value nodes — the differing term as sh:value.
        // (For node shapes value_nodes is the focus node itself, so the same
        // symmetric difference applies.)
        let _ = path;
        let current_values: HashSet<TermRef<'a>> = value_nodes.iter().copied().collect();

        for &value_node in &current_values {
            if !other_values.contains(&value_node) {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!(
                        "Value is missing from values of property {}",
                        self.0
                    ))
                    .component(sh::EQUALS_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:equals {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }
        for &other_value in &other_values {
            if !current_values.contains(&other_value) {
                let builder = ViolationBuilder::new(focus_node)
                    .value(other_value)
                    .message(format!(
                        "Value of property {} is missing from the value nodes",
                        self.0
                    ))
                    .component(sh::EQUALS_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:equals {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
