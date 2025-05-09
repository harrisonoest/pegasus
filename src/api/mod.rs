// src/api/mod.rs
// This module contains the Axum web server setup, routes, and handlers.

pub mod handlers;

use axum::{
    Router,
    extract::ws::WebSocketUpgrade,
    response::IntoResponse,
    routing::{get, get_service, post},
};
use tower_http::services::ServeDir;

/// Creates the main Axum application router.
///
/// Configures routes for serving static frontend files and API endpoints.
pub fn create_router() -> Router {
    tracing::info!("Creating Axum router");

    // Define the service to serve static files from the `static` directory
    // The `ServeDir` service handles serving files and directories.
    // `fallback` is used to serve `index.html` for requests that don't match a file.
    let static_service =
        ServeDir::new("static").fallback(get_service(ServeDir::new("static/index.html")));

    // Build the router
    Router::new()
        // Define the API route `/api/submit` which accepts POST requests
        // It's linked to the `submit_url` handler function.
        .route("/api/submit", post(handlers::submit_url))
        // WebSocket route for real-time download progress updates
        .route("/ws", get(ws_handler))
        // Define a fallback service to serve static files for any other request.
        // This allows serving index.html, styles.css, script.js, etc.
        .fallback_service(static_service)
}

/// WebSocket handler for real-time updates
async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    // Accept the WebSocket connection and pass it to the handler
    ws.on_upgrade(handlers::handle_socket_connection)
}
