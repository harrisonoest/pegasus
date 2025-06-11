// src/download/mod.rs

// use crate::api::handlers::send_progress_update; // TODO: Remove this and pass in progress sender
use crate::api::handlers::ProgressUpdate;
use crate::error::{PegasusError, Result};
use crate::process::{self, AudioFormat, MediaQuality, ProcessingOptions};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Supported media platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    YouTube,
    SoundCloud,
    Vimeo,
    Twitch,
    Other(String),
}

/// Download options for media
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadOptions {
    /// Processing options for the media
    pub processing_options: ProcessingOptions,
    /// Whether to download subtitles
    pub download_subtitles: bool,
    /// Subtitle language (if download_subtitles is true)
    pub subtitle_language: Option<String>,
    /// Whether to download thumbnail
    pub download_thumbnail: bool,
    /// Whether to download metadata
    pub download_metadata: bool,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            processing_options: ProcessingOptions::default(),
            download_subtitles: false,
            subtitle_language: None,
            download_thumbnail: true,
            download_metadata: true,
        }
    }
}

/// Represents the various states a download job can be in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadJobStatus {
    Queued,
    Starting,
    Downloading,
    Processing,
    Completed,
    Cancelled,
    Error,
}

impl std::fmt::Display for DownloadJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadJobStatus::Queued => write!(f, "Queued"),
            DownloadJobStatus::Starting => write!(f, "Starting"),
            DownloadJobStatus::Downloading => write!(f, "Downloading"),
            DownloadJobStatus::Processing => write!(f, "Processing"),
            DownloadJobStatus::Completed => write!(f, "Completed"),
            DownloadJobStatus::Cancelled => write!(f, "Cancelled"),
            DownloadJobStatus::Error => write!(f, "Error"),
        }
    }
}

/// Media download job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadJob {
    /// Unique job ID
    pub id: String,
    /// Media URL
    pub url: String,
    /// Output directory
    pub output_dir: PathBuf,
    /// Download options
    pub options: DownloadOptions,
    /// Platform (detected from URL)
    pub platform: Platform,
    /// Status of the job
    pub status: DownloadJobStatus,
    /// A detailed message about the current status (e.g., error details).
    pub status_message: Option<String>,
    /// Progress (0.0 - 1.0)
    pub progress: f32,
}

impl DownloadJob {
    /// Create a new download job with a unique ID.
    pub fn new(url: String, output_dir: PathBuf, options: DownloadOptions) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id,
            url: url.clone(),
            output_dir,
            options,
            platform: detect_platform(&url),
            status: DownloadJobStatus::Queued,
            status_message: Some("Job is waiting in the queue.".to_string()),
            progress: 0.0,
        }
    }
}

/// Gets video information from a URL using yt-dlp.
///
/// # Arguments
///
/// * `url` - The URL of the video to get information for.
///
/// # Returns
///
/// A `Result` containing the video information as a JSON Value.
async fn get_video_info(url: &str) -> Result<Value> {
    info!(url = %url, "Getting video information");

    // Use yt-dlp to get video information in JSON format
    let output = Command::new("yt-dlp")
        .arg("--dump-json")
        .arg(url)
        .output()
        .map_err(|e| {
            error!(error = %e, "Failed to execute yt-dlp command");
            PegasusError::ExternalCommandError(format!("Failed to execute yt-dlp command: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(stderr = %stderr, "yt-dlp command failed");
        return Err(PegasusError::ExternalCommandError(format!(
            "yt-dlp command failed: {}",
            stderr
        )));
    }

    // Parse the JSON output
    let json_str = String::from_utf8_lossy(&output.stdout);
    let video_info: Value = serde_json::from_str(&json_str).map_err(|e| {
        error!(error = %e, "Failed to parse yt-dlp JSON output");
        PegasusError::ExternalCommandError(format!("Failed to parse yt-dlp JSON output: {}", e))
    })?;

    Ok(video_info)
}

/// Sanitizes a filename to ensure it's valid for the filesystem.
///
/// # Arguments
///
/// * `filename` - The filename to sanitize.
///
/// # Returns
///
/// A sanitized filename string.
fn sanitize_filename(filename: &str) -> String {
    // Replace characters that are problematic in filenames
    let mut sanitized = filename.replace(
        &['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'][..],
        "_",
    );

    // Trim whitespace and limit length
    sanitized = sanitized.trim().to_string();
    if sanitized.len() > 200 {
        sanitized = sanitized[..200].to_string();
    }

    // Ensure we have a valid filename
    if sanitized.is_empty() {
        sanitized = "unknown_title".to_string();
    }

    sanitized
}

/// Detect the platform from a URL
///
/// # Arguments
///
/// * `url` - The URL to detect the platform from
///
/// # Returns
///
/// The detected platform
pub fn detect_platform(url: &str) -> Platform {
    if url.contains("youtube.com") || url.contains("youtu.be") {
        Platform::YouTube
    } else if url.contains("soundcloud.com") {
        Platform::SoundCloud
    } else if url.contains("vimeo.com") {
        Platform::Vimeo
    } else if url.contains("twitch.tv") {
        Platform::Twitch
    } else {
        Platform::Other("unknown".to_string())
    }
}

/// Process a download job through the complete pipeline
///
/// This function handles the entire download and processing pipeline:
/// 1. Download the media file using yt-dlp
/// 2. Process the media file using FFmpeg
/// 3. Clean up temporary files
///
/// # Arguments
///
/// * `job` - The download job to process
///
/// # Returns
///
/// A `Result` containing the path to the processed file
pub async fn process_download_job(
    job: &DownloadJob,
    progress_sender: &broadcast::Sender<ProgressUpdate>,
) -> Result<PathBuf> {
    info!(
        job_id = %job.id,
        url = %job.url,
        output_dir = ?job.output_dir,
        "Processing download job"
    );

    // Step 1: Download the media file
    let downloaded_file_path = download_media_with_progress(
        &job.url,
        &job.output_dir,
        &job.options,
        &job.id,
        progress_sender,
    )
    .await?;

    // Step 2: Process the media file
    let final_file_path = process::process_media(
        &downloaded_file_path,
        &job.output_dir,
        &job.options.processing_options,
        &job.id,
        &job.url,
        progress_sender,
    )
    .await?;

    // Step 3: Clean up temporary files if needed
    // Only clean up if the processed file is different from the downloaded file
    if downloaded_file_path != final_file_path {
        info!(
            job_id = %job.id,
            file = ?downloaded_file_path,
            "Cleaning up temporary file"
        );

        // Delete the downloaded file asynchronously
        // We don't want to fail the job if cleanup fails
        tokio::fs::remove_file(&downloaded_file_path)
            .await
            .unwrap_or_else(|e| {
                warn!(
                    job_id = %job.id,
                    file = ?downloaded_file_path,
                    error = %e,
                    "Failed to clean up temporary file"
                );
            });
    }

    Ok(final_file_path)
}

/// Downloads media using the yt-dlp binary directly with progress updates.
///
/// # Arguments
///
/// * `url` - The URL of the media to download.
/// * `output_dir` - The directory where the downloaded file should be saved.
/// * `options` - The download options.
/// * `job_id` - A unique identifier for this download job.
/// * `progress_sender` - The broadcast channel for sending progress updates
///
/// # Returns
///
/// A `Result` containing the path to the downloaded file.
pub async fn download_media_with_progress(
    url: &str,
    output_dir: &Path,
    options: &DownloadOptions,
    job_id: &str,
    progress_sender: &broadcast::Sender<ProgressUpdate>,
) -> Result<PathBuf> {
    // Log the start of the download process with job ID
    info!(job_id = %job_id, url = %url, output_path = ?output_dir, options = ?options, "Attempting to download using yt-dlp binary with progress tracking");

    // Ensure the output directory exists, creating it if necessary.
    if !output_dir.exists() {
        info!(path = ?output_dir, "Output directory does not exist, creating it.");
        tokio::fs::create_dir_all(output_dir).await.map_err(|e| {
            error!(error = %e, path = ?output_dir, "Failed to create output directory");
            PegasusError::IoError(e)
        })?;
    }

    // First, get video information to use for naming and thumbnails
    info!(job_id = %job_id, "Fetching video information");
    // send_progress_update(job_id, url, "info", 0.1, "Fetching video information...");

    let video_info = get_video_info(url).await?;

    let video_title = video_info["title"].as_str().unwrap_or("unknown_title");
    let safe_title = sanitize_filename(video_title);
    let duration = video_info["duration"].as_f64().unwrap_or(0.0) as f32;

    // send_progress_update(
    //     job_id,
    //     url,
    //     "info",
    //     0.2,
    //     &format!("Found media: {}", video_title),
    // );

    // Create a temporary filename for the downloaded file
    // We'll use a different extension based on whether we're downloading audio or video
    let temp_ext = if options.processing_options.audio_only {
        "mp3"
    } else {
        "mp4"
    };
    let temp_filename = format!("{}.{}", safe_title, temp_ext);
    let output_path = output_dir.join(&temp_filename);

    // Build the yt-dlp command based on options
    let mut cmd = Command::new("yt-dlp");

    // Set output template
    cmd.arg("-o").arg(output_path.to_string_lossy().to_string());

    // Add format selection based on audio/video mode
    if options.processing_options.audio_only {
        info!(job_id = %job_id, "Downloading audio only");
        // send_progress_update(
        //     job_id,
        //     url,
        //     "downloading",
        //     0.3,
        //     "Starting audio download...",
        // );

        // Extract audio
        cmd.arg("--extract-audio");

        // Set audio format based on options
        let format = match options.processing_options.audio_format {
            Some(AudioFormat::MP3) => "mp3",
            Some(AudioFormat::M4A) => "m4a",
            Some(AudioFormat::Opus) => "opus",
            None => "mp3", // Default to MP3
        };
        cmd.arg("--audio-format").arg(format);

        // Set audio quality based on options
        let quality = match options.processing_options.audio_quality {
            Some(MediaQuality::High) => "0",   // Best quality
            Some(MediaQuality::Medium) => "5", // Medium quality
            Some(MediaQuality::Low) => "7",    // Lower quality
            None => "0",                       // Default to best quality
        };
        cmd.arg("--audio-quality").arg(quality);
    } else {
        info!(job_id = %job_id, "Downloading video");
        // send_progress_update(
        //     job_id,
        //     url,
        //     "downloading",
        //     0.3,
        //     "Starting video download...",
        // );

        // Select video quality – be more permissive: let yt-dlp choose the best
        // container/codec; only limit resolution for Medium / Low so we avoid
        // "Requested format is not available" errors.
        let format = match options.processing_options.video_quality {
            Some(MediaQuality::High) => "bestvideo+bestaudio/best",
            Some(MediaQuality::Medium) => "bestvideo[height<=720]+bestaudio/best[height<=720]/best",
            Some(MediaQuality::Low) => "bestvideo[height<=480]+bestaudio/best[height<=480]/best",
            None => "bestvideo+bestaudio/best", // Default to best quality
        };
        cmd.arg("-f").arg(format);

        // Merge into mp4 container
        cmd.arg("--merge-output-format").arg("mp4");
    }

    // Add metadata embedding if requested
    if options.download_metadata || options.processing_options.embed_metadata {
        cmd.arg("--embed-metadata");
        cmd.arg("--write-info-json");
    }

    // Add thumbnail embedding if requested
    if options.download_thumbnail || options.processing_options.embed_thumbnail {
        cmd.arg("--embed-thumbnail");
        cmd.arg("--write-thumbnail");
    }

    // Add subtitle downloading if requested
    if options.download_subtitles || options.processing_options.embed_subtitles {
        cmd.arg("--write-sub");
        cmd.arg("--sub-format").arg("srt");

        // Add language filter if specified
        if let Some(lang) = &options.subtitle_language {
            cmd.arg("--sub-lang").arg(lang);
        } else if let Some(lang) = &options.processing_options.subtitle_language {
            cmd.arg("--sub-lang").arg(lang);
        } else {
            cmd.arg("--sub-lang").arg("en"); // Default to English
        }
    }

    // Add progress tracking
    cmd.arg("--newline") // Important for progress parsing
        .arg("--progress");

    // Add URL
    cmd.arg(url);

    // Set up stdout/stderr capture for progress tracking
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    // Start the command
    let mut child = cmd.spawn().map_err(|e| {
        error!(error = %e, "Failed to execute yt-dlp command");
        PegasusError::ExternalCommandError(format!("Failed to execute yt-dlp command: {}", e))
    })?;

    // Track progress from stdout
    if let Some(stdout) = child.stdout.take() {
        let job_id_clone = job_id.to_string();
        let url_clone = url.to_string();
        let duration_clone = duration;
        let progress_sender_clone = progress_sender.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            parse_yt_dlp_progress(
                reader,
                &job_id_clone,
                &url_clone,
                duration_clone,
                progress_sender_clone,
            );
        });
    }

    // Track errors from stderr
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            #[allow(clippy::manual_flatten)]
            for line_result in reader.lines() {
                if let Ok(line) = line_result {
                    debug!("yt-dlp stderr: {}", line);
                    // Only send error messages to the client if they seem important
                    if line.contains("ERROR") {
                        // TODO: Send progress update via broadcast channel
                        // send_progress_update(&job_id_clone, &url_clone, "warning", 0.0, &line);
                    }
                }
            }
        });
    }

    // Wait for the command to complete
    let status = child.wait().map_err(|e| {
        error!(error = %e, "Failed to wait for yt-dlp command");
        PegasusError::ExternalCommandError(format!("Failed to wait for yt-dlp command: {}", e))
    })?;

    if !status.success() {
        error!(status = ?status, "yt-dlp command failed");
        return Err(PegasusError::YtDlpError(format!(
            "yt-dlp command failed with status: {:?}",
            status
        )));
    }

    // Return the path to the downloaded file
    Ok(output_path)
}

/// Parse yt-dlp progress output and send progress updates
///
/// # Arguments
///
/// * `reader` - A BufRead reader for the yt-dlp output
/// * `job_id` - The job ID for the download
/// * `url` - The URL being downloaded
/// * `_duration` - The duration of the media in seconds (if known)
/// * `progress_sender` - The broadcast channel for sending progress updates
fn parse_yt_dlp_progress<R: BufRead>(
    reader: R,
    job_id: &str,
    url: &str,
    _duration: f32,
    progress_sender: broadcast::Sender<ProgressUpdate>,
) {
    // Regex patterns for progress extraction
    let download_regex = Regex::new(r"\[download\]\s+([\d.]+)%").unwrap();
    let eta_regex = Regex::new(r"ETA\s+([\d:]+)").unwrap();
    let speed_regex = Regex::new(r"at\s+([\d.]+[KMGT]?iB/s)").unwrap();

    // Process each line of output
    #[allow(clippy::manual_flatten)]
    for line_result in reader.lines() {
        if let Ok(line) = line_result {
            debug!("yt-dlp stdout: {}", line);

            // Extract download progress percentage
            if let Some(captures) = download_regex.captures(&line) {
                if let Some(percent_str) = captures.get(1) {
                    if let Ok(percent) = percent_str.as_str().parse::<f32>() {
                        // Normalize progress to 0.3-0.9 range (download phase)
                        let normalized_progress = 0.3 + (percent / 100.0) * 0.6;

                        // Extract ETA and speed if available
                        let mut message = format!("Downloading: {:.1}%", percent);

                        if let Some(eta_captures) = eta_regex.captures(&line) {
                            if let Some(eta) = eta_captures.get(1) {
                                message.push_str(&format!(" (ETA: {})", eta.as_str()));
                            }
                        }

                        if let Some(speed_captures) = speed_regex.captures(&line) {
                            if let Some(speed) = speed_captures.get(1) {
                                message.push_str(&format!(" at {}", speed.as_str()));
                            }
                        }

                        // Send progress update
                        let update = ProgressUpdate {
                            job_id: job_id.to_string(),
                            url: url.to_string(),
                            status: "downloading".to_string(),
                            progress: normalized_progress,
                            message,
                        };

                        if let Err(e) = progress_sender.send(update) {
                            warn!("Failed to send progress update: {}", e);
                        }
                    }
                }
            }
            // Check for download completion
            else if line.contains("[ffmpeg] Merging formats into")
                || line.contains("Destination:")
            {
                // TODO: Send progress update via broadcast channel
                // send_progress_update(job_id, url, "downloading", 0.9, "Finalizing download...");
            }
        }
    }
}

/// Backwards compatibility function for the old API
///
/// # Arguments
///
/// * `url` - The URL of the media to download.
/// * `output_dir` - The directory where the downloaded file should be saved.
/// * `processing_options` - A slice of strings representing the desired processing options.
/// * `job_id` - A unique identifier for this download job.
///
/// # Returns
///
/// A `Result` containing the full path to the downloaded file.
#[deprecated(since = "0.2.0", note = "Use download_media_with_progress instead")]
#[allow(deprecated)]
pub async fn download_video_with_progress(
    url: &str,
    output_dir: &Path,
    processing_options: &[String],
    job_id: &str,
    progress_sender: &broadcast::Sender<ProgressUpdate>,
) -> Result<String> {
    // Convert old-style processing options to new DownloadOptions
    let mut options = DownloadOptions::default();

    // Set audio-only mode if requested
    if processing_options.contains(&"audio-only".to_string()) {
        options.processing_options.audio_only = true;
    }

    // Set thumbnail embedding if requested
    if processing_options.contains(&"add-thumbnail".to_string()) {
        options.processing_options.embed_thumbnail = true;
    }

    // Set subtitle embedding if requested
    if processing_options.contains(&"add-subtitles".to_string()) {
        options.processing_options.embed_subtitles = true;
    }

    // Download the media using the new function
    let path =
        download_media_with_progress(url, output_dir, &options, job_id, progress_sender).await?;

    // Return the path as a string for backwards compatibility
    Ok(path.to_string_lossy().to_string())
}
