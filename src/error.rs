use thiserror::Error;

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("request error: {message}")]
    RequestError { message: String, code: u16 },

    #[error("file system error: {message}")]
    FileError { message: String },

    #[error("I/O error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("serde error: {0}")]
    SerdeError(#[from] serde_yaml::Error),

    #[error("zip error: {0}")]
    ZipError(#[from] zip::result::ZipError),
}

pub(crate) type EResult<T> = Result<T, CustomError>;
