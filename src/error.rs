//! Error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Lance: {0}")]
    Lance(String),
    
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Arrow: {0}")]
    Arrow(#[from] arrow::error::ArrowError),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
