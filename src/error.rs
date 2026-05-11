use thiserror::Error;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Cycle detected in dependency graph")]
    CycleDetected,

    #[error("Invalid USD reference syntax in {file}: {detail}")]
    InvalidUsdRef { file: String, detail: String },

    #[error("{0}")]
    Other(String),
}
