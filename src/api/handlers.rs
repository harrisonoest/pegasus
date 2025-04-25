// src/api/handlers.rs
// Contains the handler functions for API endpoints.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, // Used to extract JSON request bodies
};
use serde::{Deserialize, Serialize}; // Needed for deriving Deserialize on the payload struct and Serialize for response
use tracing; // Added to use tracing::info!

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
pub async fn submit_url(Json(payload): Json<SubmitPayload>) -> Response {
    // Log the received payload for debugging using tracing::info!
    // Include payload details in structured logging.
    tracing::info!(media_url = %payload.media_url, output_dir = ?payload.output_dir, processing_options = ?payload.processing_options, "Received submission payload");

    // TODO: Validate the input (e.g., URL format, processing option)

    // TODO: Pass the payload information to the download/processing/transfer modules.
    //       This might involve sending it to a channel or calling functions directly.

    // Return a JSON response instead of plain text
    let response_body = SubmitResponse {
        message: "Submission received successfully.".to_string(),
        job_id: None, // We don't have job IDs yet
    };

    (StatusCode::OK, Json(response_body)).into_response()
}
