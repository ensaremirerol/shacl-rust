use oxigraph::model::{vocab::rdf::TYPE, Graph, TermRef};

use crate::{
    core::{constraints::ClassConstraint, path::Path, shape::Shape},
    utils,
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for ClassConstraint<'a> {
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
                let is_instance = data_graph
                    .triples_for_subject(value_as_node)
                    .any(|triple| triple.predicate == TYPE && triple.object == self.0.into());

                if !is_instance {
                    let builder = ViolationBuilder::new(focus_node)
                        .value(value_node)
                        .message(format!("Value is not an instance of class {}", self.0))
                        .component(sh::CLASS_CONSTRAINT_COMPONENT)
                        .detail(format!("sh:class {}", self.0));

                    violations.push(shape.build_validation_result(builder));
                }
            } else {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message("Value must be a node to check class membership")
                    .component(sh::CLASS_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:class {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        violations
    }
}
