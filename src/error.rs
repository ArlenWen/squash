use std::fmt;

#[derive(Debug)]
pub enum SquashError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    DockerError(String),
    InvalidInput(String),
    LayerNotFound(String),
}

impl fmt::Display for SquashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SquashError::IoError(err) => write!(f, "IO error: {}", err),
            SquashError::JsonError(err) => write!(f, "JSON error: {}", err),
            SquashError::DockerError(msg) => write!(f, "Docker error: {}", msg),
            SquashError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            SquashError::LayerNotFound(id) => write!(f, "Layer not found: {}", id),
        }
    }
}

impl std::error::Error for SquashError {}

impl From<std::io::Error> for SquashError {
    fn from(err: std::io::Error) -> Self {
        SquashError::IoError(err)
    }
}

impl From<serde_json::Error> for SquashError {
    fn from(err: serde_json::Error) -> Self {
        SquashError::JsonError(err)
    }
}

pub type Result<T> = std::result::Result<T, SquashError>;
