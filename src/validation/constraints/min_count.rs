use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::MinCountConstraint, path::Path, shape::Shape},
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for MinCountConstraint {
    fn validate(
        &'a self,
        _data_graph: &'a Graph,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Vec<ValidationResult<'a>> {
        let count = value_nodes.len() as i32;
        if count < self.0 {
            let builder = ViolationBuilder::new(focus_node)
                .message(format!("Property has {} values (min: {})", count, self.0))
                .component(sh::MIN_COUNT_CONSTRAINT_COMPONENT)
                .detail(format!("sh:minCount {}", self.0));

            let result = shape.build_validation_result(builder);
            vec![result]
        } else {
            Vec::new()
        }
    }
}
