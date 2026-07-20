use oxigraph::model::TermRef;

use crate::{
    core::{constraints::MinLengthConstraint, path::Path, shape::Shape},
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for MinLengthConstraint {
    fn validate(
        &'a self,
        _validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();

        for &value_node in value_nodes {
            // Per spec, the string representation of literals and IRIs is
            // measured; blank-node value nodes always violate.
            let string_value = match value_node {
                TermRef::Literal(lit) => Some(lit.value()),
                TermRef::NamedNode(nn) => Some(nn.as_str()),
                TermRef::BlankNode(_) => None,
            };
            let violating = match string_value {
                Some(v) => {
                    let len = v.chars().count() as i32;
                    len < self.0
                }
                None => true,
            };
            if violating {
                let len_repr = string_value
                    .map(|v| v.chars().count().to_string())
                    .unwrap_or_else(|| "blank node".to_string());
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message(format!(
                        "String length {} is less than minimum {}",
                        len_repr, self.0
                    ))
                    .component(sh::MIN_LENGTH_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:minLength {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
