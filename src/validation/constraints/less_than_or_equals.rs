use oxigraph::model::{Graph, TermRef};
use std::collections::HashSet;

use crate::{
    core::{constraints::LessThanOrEqualsConstraint, path::Path, shape::Shape},
    utils,
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for LessThanOrEqualsConstraint<'a> {
    fn validate(
        &'a self,
        data_graph: &'a Graph,
        focus_node: TermRef<'a>,
        path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Vec<ValidationResult<'a>> {
        let Some(focus_as_node) = utils::term_to_named_or_blank(focus_node) else {
            return vec![];
        };

        let mut violations = Vec::new();

        let other_values: HashSet<TermRef<'a>> = self.0
            .resolve_path_for_given_node(data_graph, &focus_as_node)
            .into_iter()
            .collect();

        let nodes_to_check = if path.is_some() {
            value_nodes.to_vec()
        } else {
            vec![focus_node]
        };

        for node in nodes_to_check {
            let mut found_valid = false;
            for other_value in &other_values {
                if utils::compare_values(node, *other_value, |cmp| cmp <= 0) {
                    found_valid = true;
                    break;
                }
            }
            if !found_valid && !other_values.is_empty() {
                let builder = ViolationBuilder::new(focus_node)
                    .value(node)
                    .message(format!(
                        "Value is not less than or equal to values of property {}",
                        self.0
                    ))
                    .component(sh::LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:lessThanOrEquals {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        violations
    }
}
