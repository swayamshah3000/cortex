use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("Vector storage error: {0}")]
    VectorStorage(String),

    #[error("Document not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Not implemented")]
    NotImplemented,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Space is user-locked: {0}")]
    SpaceLocked(String),

    #[error("Invalid input: {0}")]
    Invalid(String),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(e: tokio::task::JoinError) -> Self {
        AppError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_serializes_to_tagged_json() {
        let err = AppError::NotFound("doc-123".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""kind":"NotFound""#));
        assert!(json.contains(r#""message":"doc-123""#));
    }

    #[test]
    fn test_not_implemented_serializes() {
        let err = AppError::NotImplemented;
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""kind":"NotImplemented""#));
    }
}
