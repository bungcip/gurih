use std::fmt;

#[derive(Debug, Clone)]
pub enum RuntimeError {
    ValidationError(String),
    WorkflowError(String),
    DataStoreError(String),
    PermissionError(String),
    InternalError(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            RuntimeError::WorkflowError(msg) => write!(f, "Workflow Error: {}", msg),
            RuntimeError::DataStoreError(msg) => write!(f, "DataStore Error: {}", msg),
            RuntimeError::PermissionError(msg) => write!(f, "Permission Error: {}", msg),
            RuntimeError::InternalError(msg) => write!(f, "Internal Error: {}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

// Helper conversion from String
impl From<String> for RuntimeError {
    fn from(err: String) -> Self {
        RuntimeError::InternalError(err)
    }
}

// Helper conversion from &str
impl From<&str> for RuntimeError {
    fn from(err: &str) -> Self {
        RuntimeError::InternalError(err.to_string())
    }
}
