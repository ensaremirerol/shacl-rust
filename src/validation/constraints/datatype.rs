use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::DatatypeConstraint, path::Path, shape::Shape},
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for DatatypeConstraint<'a> {
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
            let TermRef::Literal(lit) = value_node else {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message("Value is not a literal")
                    .component(sh::DATATYPE_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:datatype {}", self.0));

                violations.push(shape.build_validation_result(builder));
                continue;
            };

            if lit.datatype() != self.0 {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!("Value does not have datatype: {}", self.0))
                    .component(sh::DATATYPE_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:datatype {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        violations
    }
}
