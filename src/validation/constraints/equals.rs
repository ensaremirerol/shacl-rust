use oxigraph::model::{Graph, TermRef};
use std::collections::HashSet;

use crate::{
    core::{constraints::EqualsConstraint, path::Path, shape::Shape},
    utils,
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for EqualsConstraint<'a> {
    fn validate(
        &'a self,
        data_graph: &'a Graph,
        focus_node: TermRef<'a>,
        path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Vec<ValidationResult<'a>> {
        let mut violations = Vec::new();

        let Some(focus_as_node) = utils::term_to_named_or_blank(focus_node) else {
            return violations;
        };

        let other_values: HashSet<TermRef<'a>> = self
            .0
            .resolve_path_for_given_node(data_graph, &focus_as_node)
            .into_iter()
            .collect();

        if path.is_some() {
            let current_values: HashSet<TermRef<'a>> = value_nodes.iter().copied().collect();

            if current_values != other_values {
                let builder = ViolationBuilder::new(focus_node)
                    .message(format!("Values do not equal values of property {}", self.0))
                    .component(sh::EQUALS_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:equals {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }
        if other_values.is_empty() {
            let builder = ViolationBuilder::new(focus_node)
                .message(format!(
                    "Focus node does not equal (no values of property {})",
                    self.0
                ))
                .component(sh::EQUALS_CONSTRAINT_COMPONENT)
                .detail(format!("sh:equals {}", self.0));

            violations.push(shape.build_validation_result(builder));
        } else {
            for other_value in other_values {
                if focus_node != other_value {
                    let builder = ViolationBuilder::new(focus_node)
                        .value(other_value)
                        .message(format!(
                            "Focus node does not equal value of property {}",
                            self.0
                        ))
                        .component(sh::EQUALS_CONSTRAINT_COMPONENT)
                        .detail(format!("sh:equals {}", self.0));

                    violations.push(shape.build_validation_result(builder));
                }
            }
        }

        violations
    }
}
