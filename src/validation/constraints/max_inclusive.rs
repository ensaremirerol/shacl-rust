use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::MaxInclusiveConstraint, path::Path, shape::Shape},
    utils,
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for MaxInclusiveConstraint<'a> {
    fn validate(
        &'a self,
        _data_graph: &'a Graph,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Vec<ValidationResult<'a>> {
        let mut violations = Vec::new();

        for &value_node in value_nodes {
            if !utils::compare_values(value_node, self.0, |cmp| cmp <= 0) {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!("Value {} exceeds maximum {}", value_node, self.0))
                    .component(sh::MAX_INCLUSIVE_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:maxInclusive {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        violations
    }
}
