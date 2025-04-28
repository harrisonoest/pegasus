// src/download/mod.rs

use crate::error::{PegasusError, Result};
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use tracing::{error, info};

/// Downloads media using the yt-dlp binary directly.
///
/// # Arguments
///
/// * `url` - The URL of the media to download.
/// * `output_dir` - The directory where the downloaded file should be saved.
/// * `processing_options` - A slice of strings representing the desired processing options (e.g., "audio-only", "add-thumbnail").
///
/// # Returns
///
/// A `Result` containing the full path (`String`) to the primary downloaded file on success,
/// or a `PegasusError` on failure.
pub async fn download_video(
    url: &str,
    output_dir: &Path,
    processing_options: &[String],
) -> Result<String> {
    info!(url = %url, output_path = ?output_dir, options = ?processing_options, "Attempting to download using yt-dlp binary");

    // Ensure the output directory exists, creating it if necessary.
    if !output_dir.exists() {
        info!(path = ?output_dir, "Output directory does not exist, creating it.");
        tokio::fs::create_dir_all(output_dir).await.map_err(|e| {
            error!(error = %e, path = ?output_dir, "Failed to create output directory");
            PegasusError::IoError(e)
        })?;
    }

    // First, get video information to use for naming and thumbnails
    info!("Fetching video information");
    let video_info = get_video_info(url).await?;

    let video_title = video_info["title"].as_str().unwrap_or("unknown_title");
    let safe_title = sanitize_filename(video_title);

    let is_audio_only = processing_options.contains(&"audio-only".to_string());
    let add_thumbnail = processing_options.contains(&"add-thumbnail".to_string());

    let output_path = if is_audio_only {
        // Download audio only
        info!("Downloading audio only");
        let output_filename = format!("{}.mp3", safe_title);
        let output_path = output_dir.join(&output_filename);

        // Build yt-dlp command for audio extraction
        let mut cmd = Command::new("yt-dlp");
        cmd.arg("--extract-audio")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--audio-quality")
            .arg("0") // Best quality
            .arg("--embed-metadata");

        // Add thumbnail embedding if requested
        if add_thumbnail {
            info!("Adding thumbnail embedding to download");
            cmd.arg("--embed-thumbnail");
        }

        // Execute the command
        let status = cmd
            .arg("--output")
            .arg(output_path.to_string_lossy().to_string())
            .arg(url)
            .status()
            .map_err(|e| {
                error!(error = %e, "Failed to execute yt-dlp command");
                PegasusError::ExternalCommandError(format!(
                    "Failed to execute yt-dlp command: {}",
                    e
                ))
            })?;

        if !status.success() {
            error!("yt-dlp command failed with status: {}", status);
            return Err(PegasusError::ExternalCommandError(format!(
                "yt-dlp command failed with status: {}",
                status
            )));
        }

        output_path
    } else {
        // Download full video
        info!("Downloading full video");
        let output_filename = format!("{}.mp4", safe_title);
        let output_path = output_dir.join(&output_filename);

        // Build yt-dlp command for video download
        let mut cmd = Command::new("yt-dlp");
        cmd.arg("-f")
            .arg("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best")
            .arg("--merge-output-format")
            .arg("mp4");

        // Add thumbnail embedding if requested
        if add_thumbnail {
            info!("Adding thumbnail embedding to download");
            cmd.arg("--embed-thumbnail");
        }

        // Execute the command
        let status = cmd
            .arg("--output")
            .arg(output_path.to_string_lossy().to_string())
            .arg(url)
            .status()
            .map_err(|e| {
                error!(error = %e, "Failed to execute yt-dlp command");
                PegasusError::ExternalCommandError(format!(
                    "Failed to execute yt-dlp command: {}",
                    e
                ))
            })?;

        if !status.success() {
            error!("yt-dlp command failed with status: {}", status);
            return Err(PegasusError::ExternalCommandError(format!(
                "yt-dlp command failed with status: {}",
                status
            )));
        }

        output_path
    };

    info!(file_path = %output_path.display(), "Download successful");
    Ok(output_path.display().to_string())
}

// No custom thumbnail embedding function needed anymore since we're using yt-dlp's built-in --embed-thumbnail flag

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
