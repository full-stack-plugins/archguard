use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuardError {
    #[error("invalid guard protocol: {0}")]
    InvalidProtocol(String),
    #[error("input error: {0}")]
    Input(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}
