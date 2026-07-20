use oxigraph::model::TermRef;

use crate::{
    core::{constraints::PatternConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for PatternConstraint {
    fn validate(
        &'a self,
        _validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();

        let Some(re) = self.compiled.as_ref() else {
            return Ok(violations);
        };

        for &value_node in value_nodes {
            let value_str = match value_node {
                TermRef::Literal(lit) => lit.value(),
                TermRef::NamedNode(nn) => nn.as_str(),
                TermRef::BlankNode(_) => {
                    // Per spec, blank-node value nodes always violate sh:pattern.
                    let builder = ViolationBuilder::new(focus_node)
                        .value(value_node)
                        .message("Blank nodes cannot match a pattern")
                        .component(sh::PATTERN_CONSTRAINT_COMPONENT)
                        .detail(format!("sh:pattern {}", self.pattern));
                    violations.push(shape.build_validation_result(builder));
                    continue;
                }
            };

            if !re.is_match(value_str) {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!("Value does not match pattern: {}", self.pattern))
                    .component(sh::PATTERN_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:pattern {}", self.pattern));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
