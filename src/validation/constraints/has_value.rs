use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::HasValueConstraint, path::Path, shape::Shape},
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for HasValueConstraint<'a> {
    fn validate(
        &'a self,
        _data_graph: &'a Graph,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Vec<ValidationResult<'a>> {
        if !value_nodes.contains(&self.0) {
            let builder = ViolationBuilder::new(focus_node)
                .message(format!("Required value {} is not present", self.0))
                .component(sh::HAS_VALUE_CONSTRAINT_COMPONENT)
                .detail(format!("sh:hasValue {}", self.0));

            vec![shape.build_validation_result(builder)]
        } else {
            Vec::new()
        }
    }
}
