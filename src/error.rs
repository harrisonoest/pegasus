// src/error.rs
// Defines custom error types for the application.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PegasusError {
    #[error("Download failed: {0}")]
    DownloadError(String),

    #[error("Processing failed: {0}")]
    ProcessingError(String),

    #[error("Transfer failed: {0}")]
    TransferError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Web server error: {0}")]
    WebServerError(String), // Example for web-related errors

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// You might want specific result types for different modules
pub type Result<T> = std::result::Result<T, PegasusError>;
