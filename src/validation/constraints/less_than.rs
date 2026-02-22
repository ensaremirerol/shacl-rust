use oxigraph::model::TermRef;

use crate::{
    core::{constraints::LessThanConstraint, path::Path, shape::Shape},
    utils,
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for LessThanConstraint<'a> {
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

        let data_graph = validation_dataset.data_graph();

        let other_values = self
            .0
            .resolve_path_for_given_node(data_graph, &focus_as_node);

        let nodes_to_check = if path.is_some() {
            value_nodes.to_vec()
        } else {
            vec![focus_node]
        };

        for node in nodes_to_check {
            let mut found_valid = false;
            for other_value in &other_values {
                if utils::compare_values(node, *other_value, |cmp| cmp < 0) {
                    found_valid = true;
                    break;
                }
            }
            if !found_valid && !other_values.is_empty() {
                let builder = ViolationBuilder::new(focus_node)
                    .value(node)
                    .message(format!(
                        "Value is not less than values of property {}",
                        self.0
                    ))
                    .component(sh::LESS_THAN_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:lessThan {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
