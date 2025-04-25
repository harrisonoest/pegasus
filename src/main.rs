// src/main.rs
// Entry point for the Pegasus application.

// Declare modules
pub mod api;
pub mod config;
pub mod download;
pub mod error;
pub mod process;
pub mod transfer;

use std::path::PathBuf;

// Use common types
use config::Config;
// Removed unused PegasusError import, keeping Result
use error::Result;

// Import Axum and Tokio listener
use axum::serve; // Updated import for serve
use tokio::net::TcpListener;

// Import tracing macros and subscriber initialization
use tracing::{error, info};
// Import dotenvy for loading .env files
use dotenvy::dotenv;
// Import EnvFilter for tracing configuration
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use yt_dlp::Youtube;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    // This should be called before any code that reads environment variables.
    match dotenv() {
        Ok(_) => info!("Loaded .env file successfully"),
        Err(_) => info!(".env file not found, using default environment variables"),
    };

    // Initialize tracing subscriber using EnvFilter
    // This reads the RUST_LOG environment variable (loaded from .env or system env).
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default tracing subscriber failed");

    // Basic placeholder
    info!("Pegasus starting...");

    // Load configuration (placeholder)
    let _config = Config::load();
    // Log configuration loading
    info!("Configuration loaded (placeholder)");

    // Install ffmpeg and yt-dlp
    install_binaries().await?;

    // Create the Axum router
    let app = api::create_router();

    // Read server port from environment variable, defaulting to 8000
    let port = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8000);

    // Define the address and port to listen on
    let addr = format!("0.0.0.0:{}", port);
    info!(address = %addr, "Starting server"); // Structured logging with address
    let listener = TcpListener::bind(&addr).await.map_err(|e| {
        // Use reference to addr
        // Log the binding error
        error!(address = %addr, error = %e, "Failed to bind to address");
        // Wrap the IO error into our custom error type (or a dedicated web server error)
        error::PegasusError::WebServerError(format!("Failed to bind to {}: {}", addr, e))
    })?;

    // Run the Axum server
    serve(listener, app.into_make_service())
        .await
        .map_err(|e| {
            // Log the server error
            error!(error = %e, "Server error");
            // Wrap the Hyper error into our custom error type
            error::PegasusError::WebServerError(format!("Server failed: {}", e))
        })?;

    Ok(())
}

/// Installs ffmpeg and yt-dlp binaries.
///
/// # Returns
///
/// A `Result` containing `()` on success, or a `PegasusError` on failure.
async fn install_binaries() -> Result<()> {
    let executables_dir = PathBuf::from("libs");
    let output_dir = PathBuf::from("output");

    Youtube::with_new_binaries(executables_dir, output_dir).await?;
    Ok(())
}
