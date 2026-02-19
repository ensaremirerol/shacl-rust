use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::NodeConstraint, path::Path, shape::Shape},
    utils,
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for NodeConstraint<'a> {
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
            if let Some(value_as_node) = utils::term_to_named_or_blank(value_node) {
                let nested_report = self.0.validate_node_report(data_graph, value_as_node);
                if !nested_report.conforms {
                    let is_focus = value_node == focus_node;
                    let builder = ViolationBuilder::new(focus_node)
                        .value(value_node)
                        .message(if is_focus {
                            "Focus node does not conform to sh:node constraint"
                        } else {
                            "Value does not conform to sh:node constraint"
                        })
                        .component(sh::NODE_CONSTRAINT_COMPONENT)
                        .detail(format!(
                            "sh:node constraint referencing shape {}",
                            self.0.node
                        ))
                        .trace_entry("sh:node validation")
                        .details(nested_report.results);

                    violations.push(shape.build_validation_result(builder));
                }
            } else {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message("Value is not a node (is a literal)")
                    .component(sh::NODE_CONSTRAINT_COMPONENT)
                    .detail(format!(
                        "sh:node constraint referencing shape {}",
                        self.0.node
                    ));

                violations.push(shape.build_validation_result(builder));
            }
        }

        violations
    }
}
