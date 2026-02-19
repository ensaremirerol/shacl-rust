use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::QualifiedValueShapeConstraint, path::Path, shape::Shape},
    utils,
    validation::{Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
};

impl<'a> Validate<'a> for QualifiedValueShapeConstraint<'a> {
    fn validate(
        &'a self,
        data_graph: &'a Graph,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Vec<ValidationResult<'a>> {
        let mut violations = Vec::new();

        if self.qualified_value_shapes_disjoint {
            return violations;
        }

        let mut conforming_count = 0;

        for &value_node in value_nodes {
            if let Some(value_as_node) = utils::term_to_named_or_blank(value_node) {
                if self.shape.validate_node(data_graph, value_as_node) {
                    conforming_count += 1;
                }
            }
        }

        if let Some(min) = self.qualified_min_count {
            if conforming_count < min {
                let builder = ViolationBuilder::new(focus_node)
                    .message(format!(
                        "Qualified value shape: {} values conform (min: {})",
                        conforming_count, min
                    ))
                    .component(sh::QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:qualifiedMinCount {}", min));

                violations.push(shape.build_validation_result(builder));
            }
        }

        if let Some(max) = self.qualified_max_count {
            if conforming_count > max {
                let builder = ViolationBuilder::new(focus_node)
                    .message(format!(
                        "Qualified value shape: {} values conform (max: {})",
                        conforming_count, max
                    ))
                    .component(sh::QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:qualifiedMaxCount {}", max));

                violations.push(shape.build_validation_result(builder));
            }
        }

        violations
    }
}
