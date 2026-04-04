use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Git operation failed: {0}")]
    Git(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Secret detection error: {0}")]
    Secret(String),

    #[error("Merge conflict in {file}: {message}")]
    MergeConflict { file: String, message: String },

    #[error("Snapshot error: {0}")]
    Snapshot(String),

    #[error("Discovery error: {0}")]
    Discovery(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Not initialized. Run 'claude-sync init' first.")]
    NotInitialized,

    #[error("Authentication failed: {0}")]
    Auth(String),
}

pub type Result<T> = std::result::Result<T, SyncError>;
