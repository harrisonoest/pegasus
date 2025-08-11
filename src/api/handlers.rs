// src/api/handlers.rs
// Contains the handler functions for API endpoints.

use super::AppState;
use crate::download::{DownloadJob, DownloadOptions};
use crate::error::PegasusError;
use crate::process::{ProcessingOptions, MediaQuality, AudioFormat};
use axum::{
    Json,
    extract::{
        Path, State,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::broadcast;
use tracing::{debug, error, info};

// Define the structure for download options from frontend
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FrontendDownloadOptions {
    mode: String, // "video" or "audio"
    
    // Video options
    video_quality: Option<String>,
    embed_subtitles: Option<bool>,
    subtitle_language: Option<String>,
    embed_chapters: Option<bool>,
    
    // Audio options
    audio_format: Option<String>,
    audio_quality: Option<String>,
    add_thumbnail: Option<bool>,
    normalize_audio: Option<bool>,
    
    // Common options
    embed_metadata: Option<bool>,
}

// Define the structure expected in the JSON request body from the frontend
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubmitPayload {
    media_url: String,
    output_dir: Option<String>,
    download_options: Option<FrontendDownloadOptions>,
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
    pub speed: Option<String>,
    pub eta: Option<String>,
}

/// Convert frontend download options to backend DownloadOptions
fn convert_frontend_options(frontend_opts: &FrontendDownloadOptions) -> DownloadOptions {
    let is_audio_mode = frontend_opts.mode == "audio";
    
    // Map video quality
    let video_quality = if !is_audio_mode {
        match frontend_opts.video_quality.as_deref() {
            Some("best") | Some("2160") | Some("1080") => Some(MediaQuality::High),
            Some("720") | Some("480") => Some(MediaQuality::Medium),
            Some("360") | _ => Some(MediaQuality::Low),
        }
    } else {
        None
    };
    
    // Map audio format
    let audio_format = if is_audio_mode {
        match frontend_opts.audio_format.as_deref() {
            Some("mp3") => Some(AudioFormat::MP3),
            Some("m4a") | Some("aac") => Some(AudioFormat::M4A),
            Some("ogg") | Some("opus") => Some(AudioFormat::Opus),
            _ => Some(AudioFormat::MP3), // default
        }
    } else {
        None
    };
    
    // Map audio quality
    let audio_quality = if is_audio_mode {
        match frontend_opts.audio_quality.as_deref() {
            Some("320") | Some("best") => Some(MediaQuality::High),
            Some("256") | Some("192") => Some(MediaQuality::Medium),
            Some("128") | Some("96") | _ => Some(MediaQuality::Low),
        }
    } else {
        None
    };
    
    let processing_options = ProcessingOptions {
        audio_only: is_audio_mode,
        video_quality,
        audio_format,
        audio_quality,
        embed_metadata: frontend_opts.embed_metadata.unwrap_or(true),
        embed_subtitles: frontend_opts.embed_subtitles.unwrap_or(false),
        embed_thumbnail: frontend_opts.add_thumbnail.unwrap_or(true),
        subtitle_language: frontend_opts.subtitle_language.clone(),
    };
    
    DownloadOptions {
        processing_options,
        download_subtitles: frontend_opts.embed_subtitles.unwrap_or(false),
        subtitle_language: frontend_opts.subtitle_language.clone().or_else(|| Some("en".to_string())),
        download_thumbnail: if is_audio_mode { 
            frontend_opts.add_thumbnail.unwrap_or(true) 
        } else { 
            false 
        },
        download_metadata: frontend_opts.embed_metadata.unwrap_or(true),
    }
}

/// Handler for the POST /api/submit route.
///
/// Adds a new download job to the queue.
/// Handler for the POST /api/submit route.
///
/// Adds a new download job to the queue.
pub async fn submit_url(
    State(state): State<AppState>,
    Json(payload): Json<SubmitPayload>,
) -> Response {
    info!(media_url = %payload.media_url, "Received submission");

    // Define a temporary download directory
    let download_base_dir = PathBuf::from("/tmp/pegasus_downloads");
    let target_download_dir = match &payload.output_dir {
        Some(dir) => download_base_dir.join(dir),
        None => download_base_dir.join("default"),
    };

    // Convert frontend options to backend structures
    let download_options = if let Some(frontend_opts) = &payload.download_options {
        convert_frontend_options(frontend_opts)
    } else {
        DownloadOptions::default()
    };

    // Create a new download job
    let job = DownloadJob::new(payload.media_url, target_download_dir, download_options);
    let job_id = job.id.clone();

    // Add the job to the queue
    match state.download_queue.add_job(job).await {
        Ok(_) => {
            info!(job_id = %job_id, "Successfully added job to queue");
            let response_body = SubmitResponse {
                message: "Submission received and job queued.".to_string(),
                job_id,
            };
            (StatusCode::ACCEPTED, Json(response_body)).into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to add job to queue");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

/// Handler for GET /api/queue.
///
/// Returns the current state of all jobs in the queue.
pub async fn get_queue_status(State(state): State<AppState>) -> impl IntoResponse {
    let jobs = state.download_queue.get_all_jobs();
    (StatusCode::OK, Json(jobs))
}

/// Handler for POST /api/downloads/:id/cancel.
///
/// Cancels a running or queued download job.
pub async fn cancel_download(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    info!(job_id = %job_id, "Received cancellation request");
    match state.download_queue.cancel_job(&job_id).await {
        Ok(_) => (StatusCode::OK, "Job cancellation initiated.").into_response(),
        Err(PegasusError::JobNotFound(_)) => {
            (StatusCode::NOT_FOUND, "Job not found.").into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to cancel job: {}", e),
        )
            .into_response(),
    }
}

/// Handle WebSocket connections for real-time progress updates.
pub async fn handle_socket_connection(
    mut socket: WebSocket,
    mut rx: broadcast::Receiver<ProgressUpdate>,
) {
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
            // Handle incoming messages from the client (e.g., ping/pong)
            Some(msg) = socket.next() => {
                if let Ok(Message::Close(_)) = msg {
                    info!("WebSocket client disconnected");
                    break;
                }
            },
            // Handle progress updates from the broadcast channel
            Ok(update) = rx.recv() => {
                match serde_json::to_string(&update) {
                    Ok(json) => {
                        if socket.send(Message::Text(json)).await.is_err() {
                            debug!("WebSocket client disconnected (send failed)");
                            break;
                        }
                    },
                    Err(e) => error!(error = %e, "Failed to serialize progress update"),
                }
            },
            // Stop the loop if the broadcast channel is closed
            else => {
                break;
            }
        }
    }
}
