use std::fmt::Display;
use std::path::Path;

#[derive(Debug)]
pub enum ShaclError {
    Io(String),
    Parse(String),
    Validation(String),
}

impl Display for ShaclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaclError::Io(e) => write!(f, "IO error: {}", e),
            ShaclError::Parse(e) => write!(f, "Parse error: {}", e),
            ShaclError::Validation(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl std::error::Error for ShaclError {}

/// Helper function to convert PathBuf to &str with better error messages
pub fn path_to_str(path: &Path) -> Result<&str, ShaclError> {
    path.to_str()
        .ok_or_else(|| ShaclError::Io(format!("Invalid UTF-8 in file path: {}", path.display())))
}
