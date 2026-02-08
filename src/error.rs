//! Error types for SHACL validation

use ::Error;

/// Result type alias for SHACL operations
pub type Result<T> = std::result::Result<T, ShaclError>;

/// Errors that can occur during SHACL validation
#[derive(Debug, Error)]
pub enum ShaclError {
    /// Error parsing RDF data
    #[error("Failed to parse RDF: {0}")]
    ParseError(String),

    /// Error loading SHACL shapes
    #[error("Failed to load SHACL shapes: {0}")]
    ShapeLoadError(String),

    /// Invalid SHACL shape definition
    #[error("Invalid shape definition: {0}")]
    InvalidShape(String),

    /// Error during validation
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Unsupported feature
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}
