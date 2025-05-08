// src/api/handlers.rs
// Contains the handler functions for API endpoints.

use axum::{
    Json,
    extract::ws::{Message, WebSocket},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::broadcast;
use tracing::{debug, error, info};
use uuid::Uuid;

// Import download module
use crate::download;

// Create a static channel for broadcasting download progress updates
static PROGRESS_CHANNEL: once_cell::sync::Lazy<(
    broadcast::Sender<ProgressUpdate>,
    broadcast::Receiver<ProgressUpdate>,
)> = once_cell::sync::Lazy::new(|| {
    let (tx, rx) = broadcast::channel(100);
    (tx, rx)
});

// Define the structure expected in the JSON request body from the frontend
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubmitPayload {
    media_url: String,
    output_dir: Option<String>,
    processing_options: Vec<String>,
}

// Define a struct for the JSON response
#[derive(Serialize, Debug)]
struct SubmitResponse {
    message: String,
    job_id: String,
}

// Define a struct for progress updates
#[derive(Clone, Serialize, Debug)]
pub struct ProgressUpdate {
    pub job_id: String,
    pub url: String,
    pub status: String,
    pub progress: f32,
    pub message: String,
}

// Updated handler function for the POST /api/submit route.
// It now accepts a JSON payload matching the SubmitPayload struct.
// Marked as async because it now calls the async download_video function.
pub async fn submit_url(Json(payload): Json<SubmitPayload>) -> Response {
    // Log the received payload for debugging using tracing::info!
    // Include payload details in structured logging.
    tracing::info!(media_url = %payload.media_url, output_dir = ?payload.output_dir, processing_options = ?payload.processing_options, "Received submission payload");

    // TODO: Validate the input (e.g., URL format, output_dir validity)

    // Generate a unique job ID
    let job_id = Uuid::new_v4().to_string();
    info!(job_id = %job_id, "Generated new job ID");

    // --- Call Download Logic ---
    // Define a temporary download directory
    let download_base_dir = PathBuf::from("/tmp/pegasus_downloads");
    // Use the output_dir from payload if provided, otherwise use a default within the base dir
    let target_download_dir = match &payload.output_dir {
        Some(dir) => download_base_dir.join(dir),
        None => download_base_dir.join("default"),
    };

    // Send initial progress update
    send_progress_update(
        &job_id,
        &payload.media_url,
        "starting",
        0.0,
        "Preparing download...",
    );

    // Clone values for the async task
    let job_id_clone = job_id.clone();
    let url_clone = payload.media_url.clone();
    let target_dir_clone = target_download_dir.clone();
    let options_clone = payload.processing_options.clone();

    // Spawn a task to handle the download asynchronously
    tokio::spawn(async move {
        // Call the download function asynchronously with progress updates
        let download_result = download::download_video_with_progress(
            &url_clone,
            &target_dir_clone,
            &options_clone,
            &job_id_clone,
        )
        .await;

        // Handle the download result
        match download_result {
            Ok(downloaded_file_path) => {
                info!(job_id = %job_id_clone, file_path = %downloaded_file_path, "Video download successful");
                // Send completion update
                send_progress_update(
                    &job_id_clone,
                    &url_clone,
                    "completed",
                    1.0,
                    "Download completed successfully",
                );
                // TODO: Trigger processing and transfer steps with downloaded_file_path
            }
            Err(e) => {
                error!(job_id = %job_id_clone, error = %e, "Video download failed");
                // Send error update
                send_progress_update(
                    &job_id_clone,
                    &url_clone,
                    "error",
                    0.0,
                    &format!("Download failed: {}", e),
                );
            }
        }
    });

    // Return an immediate response with the job ID
    let response_body = SubmitResponse {
        message: "Submission received and download started.".to_string(),
        job_id,
    };
    (StatusCode::OK, Json(response_body)).into_response()
}

/// Helper function to send progress updates to all connected WebSocket clients
pub fn send_progress_update(job_id: &str, url: &str, status: &str, progress: f32, message: &str) {
    let update = ProgressUpdate {
        job_id: job_id.to_string(),
        url: url.to_string(),
        status: status.to_string(),
        progress,
        message: message.to_string(),
    };

    // Send the update to all subscribers
    if let Err(e) = PROGRESS_CHANNEL.0.send(update) {
        debug!("Failed to broadcast progress update: {}", e);
    }
}

/// Handle WebSocket connections for real-time progress updates
pub async fn handle_socket_connection(mut socket: WebSocket) {
    // Subscribe to the progress channel
    let mut rx = PROGRESS_CHANNEL.0.subscribe();

    info!("New WebSocket client connected");

    // Send a welcome message
    if let Err(e) = socket
        .send(Message::Text(
            "Connected to Pegasus download progress feed".to_string(),
        ))
        .await
    {
        error!("Failed to send welcome message: {}", e);
        return;
    }

    // Main WebSocket message loop
    loop {
        tokio::select! {
            // Handle incoming messages from the client
            Some(msg) = socket.next() => {
                match msg {
                    Ok(Message::Close(_)) => {
                        info!("WebSocket client disconnected");
                        break;
                    },
                    Ok(_) => {},
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            },
            // Handle progress updates from the channel
            Ok(update) = rx.recv() => {
                // Serialize the update to JSON
                match serde_json::to_string(&update) {
                    Ok(json) => {
                        // Send the update to the client
                        if let Err(e) = socket.send(Message::Text(json)).await {
                            error!("Failed to send progress update: {}", e);
                            break;
                        }
                    },
                    Err(e) => {
                        error!("Failed to serialize progress update: {}", e);
                    }
                }
            },
        }
    }
}
