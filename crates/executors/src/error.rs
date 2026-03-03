use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("agent process failed to start: {0}")]
    SpawnFailed(String),
    #[error("agent process communication error: {0}")]
    CommunicationError(String),
    #[error("agent process not found for session: {0}")]
    ProcessNotFound(String),
    #[error("protocol error: {0}")]
    ProtocolError(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
