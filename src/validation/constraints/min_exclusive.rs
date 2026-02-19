use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::MinExclusiveConstraint, path::Path, shape::Shape},
    utils,
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for MinExclusiveConstraint<'a> {
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
            if !utils::compare_values(value_node, self.0, |cmp| cmp > 0) {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!(
                        "Value {} is not greater than {}",
                        value_node, self.0
                    ))
                    .component(sh::MIN_EXCLUSIVE_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:minExclusive {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        violations
    }
}
