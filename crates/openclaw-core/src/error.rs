use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpenClawError {
    #[error("config error: {0}")]
    Config(String),
    #[error("session error: {0}")]
    Session(String),
    #[error("channel error: {0}")]
    Channel(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, OpenClawError>;
