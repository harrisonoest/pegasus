// src/api/mod.rs
// This module contains the Axum web server setup, routes, and handlers.

pub mod handlers;

use crate::queue_manager::DownloadQueue;
use axum::{
    Router,
    extract::State,
    extract::ws::WebSocketUpgrade,
    response::IntoResponse,
    routing::{get, get_service, post},
};
use handlers::ProgressUpdate;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub download_queue: Arc<DownloadQueue>,
    pub progress_sender: broadcast::Sender<ProgressUpdate>,
}

/// Creates the main Axum application router.
///
/// Configures routes for serving static frontend files and API endpoints.
pub fn create_router(
    download_queue: Arc<DownloadQueue>,
    progress_sender: broadcast::Sender<ProgressUpdate>,
) -> Router {
    // Create the shared state
    let app_state = AppState {
        download_queue,
        progress_sender,
    };
    tracing::info!("Creating Axum router");

    // Define the service to serve static files from the `static` directory
    // The `ServeDir` service handles serving files and directories.
    // `fallback` is used to serve `index.html` for requests that don't match a file.
    let static_service =
        ServeDir::new("static").fallback(get_service(ServeDir::new("static/index.html")));

    // Build the router
    Router::new()
        // --- API Routes ---
        .route("/api/submit", post(handlers::submit_url))
        .route("/api/queue", get(handlers::get_queue_status))
        .route("/api/downloads/:id/cancel", post(handlers::cancel_download))
        // WebSocket route for real-time download progress updates
        .route("/ws", get(ws_handler))
        // --- Static File Serving ---
        .fallback_service(static_service)
        // Add the application state to the router
        .with_state(app_state)
}

/// WebSocket handler for real-time updates
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    // Accept the WebSocket connection and pass it to the handler
    ws.on_upgrade(move |socket| {
        handlers::handle_socket_connection(socket, state.progress_sender.subscribe())
    })
}
