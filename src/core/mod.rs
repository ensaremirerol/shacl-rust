//! Core SHACL types and structures
//!
//! This module contains the fundamental types used to represent SHACL shapes,
//! constraints, paths, and targets.

pub mod constraints;
pub mod path;
pub mod shape;
pub mod target;

// Re-export commonly used types
pub use constraints::{Constraint, NodeKind};
pub use path::{Path, PathElement};
pub use shape::{ClosedConstraint, Shape, ShapeReference, ShapesInfo};
pub use target::Target;
