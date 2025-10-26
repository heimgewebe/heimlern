use thiserror::Error;

#[derive(Debug, Error)]
pub enum BanditError {
    #[error("Snapshot deserialization failed: {0}")]
    Snapshot(#[from] serde_json::Error),
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    #[error("Internal error: {0}")]
    Internal(&'static str),
}

pub type Result<T> = std::result::Result<T, BanditError>;
