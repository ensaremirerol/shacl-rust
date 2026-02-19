use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::NodeKindConstraint, path::Path, shape::Shape},
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for NodeKindConstraint {
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
            let valid = match self.0 {
                crate::core::constraints::NodeKind::BlankNode => value_node.is_blank_node(),
                crate::core::constraints::NodeKind::IRI => value_node.is_named_node(),
                crate::core::constraints::NodeKind::Literal => value_node.is_literal(),
                crate::core::constraints::NodeKind::BlankNodeOrIRI => {
                    value_node.is_blank_node() || value_node.is_named_node()
                }
                crate::core::constraints::NodeKind::BlankNodeOrLiteral => {
                    value_node.is_blank_node() || value_node.is_literal()
                }
                crate::core::constraints::NodeKind::IRIOrLiteral => {
                    value_node.is_named_node() || value_node.is_literal()
                }
            };

            if !valid {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!("Value does not have node kind: {}", self.0))
                    .component(sh::NODE_KIND_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:nodeKind {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        violations
    }
}
