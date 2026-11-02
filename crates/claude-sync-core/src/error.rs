use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("설정 오류: {0}")]
    Config(String),

    #[error("IO 오류: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 오류: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML 파싱 오류: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML 직렬화 오류: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("번들 오류: {0}")]
    Bundle(String),

    #[error("ZIP 오류: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("템플릿 오류: {0}")]
    Template(String),

    #[error("백업 오류: {0}")]
    Backup(String),

    #[error("P2P 오류: {0}")]
    P2p(String),

    #[error("찾을 수 없음: {0}")]
    NotFound(String),

    #[error("취소됨")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, SyncError>;
