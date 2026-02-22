pub mod core;
pub mod err;
pub mod parser;
pub mod rdf;
pub mod utils;
pub mod validation;
pub mod vocab;

// Re-export commonly used items for convenience
pub use core::{
    constraints::{Constraint, NodeKind},
    path::{Path, PathElement},
    shape::{ClosedConstraint, Shape, ShapeReference},
    target::Target,
};
pub use err::ShaclError;
pub use parser::parse_shapes;
pub use validation::{report::ValidationReport, report::ValidationResult, validate};
pub use vocab::sh;
