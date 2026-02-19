use oxigraph::model::{Graph, TermRef};
use std::collections::HashSet;

use crate::{
    core::{constraints::DisjointConstraint, path::Path, shape::Shape},
    utils,
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for DisjointConstraint<'a> {
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
            if other_values.contains(&node) {
                let builder = ViolationBuilder::new(focus_node)
                    .value(node)
                    .message("Value appears in both properties (not disjoint)")
                    .component(sh::DISJOINT_CONSTRAINT_COMPONENT)
                    .detail("sh:disjoint");
                violations.push(shape.build_validation_result(builder));
            }
        }

        violations
    }
}
