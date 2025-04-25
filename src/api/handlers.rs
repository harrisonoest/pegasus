// src/api/handlers.rs
// Contains the handler functions for API endpoints.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, // Used to extract JSON request bodies
};
use serde::{Deserialize, Serialize}; // Needed for deriving Deserialize on the payload struct and Serialize for response
use std::path::PathBuf;
use tracing::{error, info}; // Added to use tracing::info! and error! // Use PathBuf for directory path

// Import download module
use crate::download;

// Define the structure expected in the JSON request body from the frontend
#[derive(Deserialize, Debug)] // Derive Deserialize to parse JSON and Debug for logging
#[serde(rename_all = "camelCase")] // Rename fields to camelCase for deserialization
pub struct SubmitPayload {
    media_url: String,
    output_dir: Option<String>, // Optional field
    processing_options: Vec<String>,
}

// Define a struct for the JSON response
#[derive(Serialize, Debug)] // Derive Serialize
struct SubmitResponse {
    message: String,
    job_id: Option<String>, // Placeholder for a future job ID
}

// Updated handler function for the POST /api/submit route.
// It now accepts a JSON payload matching the SubmitPayload struct.
// Marked as async because it now calls the async download_video function.
pub async fn submit_url(Json(payload): Json<SubmitPayload>) -> Response {
    // Log the received payload for debugging using tracing::info!
    // Include payload details in structured logging.
    tracing::info!(media_url = %payload.media_url, output_dir = ?payload.output_dir, processing_options = ?payload.processing_options, "Received submission payload");

    // TODO: Validate the input (e.g., URL format, output_dir validity)

    // --- Call Download Logic ---
    // Define a temporary download directory (consider making this configurable)
    let download_base_dir = PathBuf::from("/tmp/pegasus_downloads");
    // Use the output_dir from payload if provided, otherwise use a default within the base dir
    let target_download_dir = match &payload.output_dir {
        Some(dir) => download_base_dir.join(dir),
        None => download_base_dir.join("default"), // Default subdirectory if not specified
    };

    // Call the download function asynchronously
    // Pass the processing options from the payload
    let download_result = download::download_video(
        &payload.media_url,
        &target_download_dir,
        &payload.processing_options, // Pass the options
    )
    .await; // Await the async download function

    // Handle the download result
    match download_result {
        Ok(downloaded_file_path) => {
            info!(file_path = %downloaded_file_path, "Video download successful");
            // TODO: Trigger processing and transfer steps with downloaded_file_path

            // Return a JSON success response
            let response_body = SubmitResponse {
                message: "Submission received and download successful.".to_string(),
                job_id: None, // Placeholder
            };
            (StatusCode::OK, Json(response_body)).into_response()
        }
        Err(e) => {
            error!(error = %e, "Video download failed");
            // Return a JSON error response
            // TODO: Create a specific error response struct?
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "message": format!("Download failed: {}", e) })),
            )
                .into_response()
        }
    }
}
