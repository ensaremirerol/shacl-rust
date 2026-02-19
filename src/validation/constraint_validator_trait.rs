use oxigraph::model::{Graph, TermRef};

use crate::{
    core::{constraints::Constraint, path::Path, shape::Shape},
    validation::ValidationResult,
};

/// Common interface for SHACL constraint validation.
pub trait ConstraintValidatorTrait {
    /// Validates a constraint for the given focus/value context.
    fn validate_constraint<'a>(
        &self,
        constraint: &'a Constraint<'a>,
        data_graph: &'a Graph,
        focus_node: TermRef<'a>,
        path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Vec<ValidationResult<'a>>;
}
