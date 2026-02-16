use std::fmt::Display;

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
