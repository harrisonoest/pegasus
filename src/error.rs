// src/error.rs
// Defines custom error types for the application.

use thiserror::Error;

// Define a custom result type using our error enum
pub type Result<T> = std::result::Result<T, PegasusError>;

// Define a custom error enum for the application
#[derive(Error, Debug)]
pub enum PegasusError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Web server error: {0}")]
    WebServerError(String),

    #[error("Download error (yt-dlp): {0}")]
    YtDlpError(#[from] yt_dlp::error::Error),

    #[error("Download error (general): {0}")]
    DownloadError(String),

    #[error("Processing failed: {0}")]
    ProcessingError(String),

    #[error("Transfer failed: {0}")]
    TransferError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
