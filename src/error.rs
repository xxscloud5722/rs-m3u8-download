use std::fmt::{Debug};
use thiserror::Error;

pub type Result<T, E = DownloadError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("ParseIntError")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Utf8Error")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("SymmetricCipherError")]
    SymmetricCipherError,

    #[error("RwLockWriteGuard")]
    RwLockWriteGuard,

    #[error("RwLockReadGuard")]
    RwLockReadGuard,

    #[error("Error")]
    Error(#[from] std::io::Error),

    #[error("ReqwestError")]
    ReqwestError(#[from] reqwest::Error),

    #[error("JoinError")]
    JoinError(#[from] tokio::task::JoinError),
}