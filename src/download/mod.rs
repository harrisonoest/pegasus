// src/download/mod.rs

use crate::api::handlers::send_progress_update;
use crate::error::{PegasusError, Result};
use regex::Regex;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio}; // Added for piping stdout/stderr
use tracing::{debug, error, info};

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

/// Downloads media using the yt-dlp binary directly with progress updates.
///
/// # Arguments
///
/// * `url` - The URL of the media to download.
/// * `output_dir` - The directory where the downloaded file should be saved.
/// * `processing_options` - A slice of strings representing the desired processing options (e.g., "audio-only", "add-thumbnail").
/// * `job_id` - A unique identifier for this download job.
///
/// # Returns
///
/// A `Result` containing the full path (`String`) to the primary downloaded file on success,
/// or a `PegasusError` on failure.
pub async fn download_video_with_progress(
    url: &str,
    output_dir: &Path,
    processing_options: &[String],
    job_id: &str,
) -> Result<String> {
    // Log the start of the download process with job ID
    info!(job_id = %job_id, url = %url, output_path = ?output_dir, options = ?processing_options, "Attempting to download using yt-dlp binary with progress tracking");

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
    send_progress_update(job_id, url, "info", 0.1, "Fetching video information...");

    let video_info = get_video_info(url).await?;

    let video_title = video_info["title"].as_str().unwrap_or("unknown_title");
    let safe_title = sanitize_filename(video_title);

    send_progress_update(
        job_id,
        url,
        "info",
        0.2,
        &format!("Found video: {}", video_title),
    );

    let is_audio_only = processing_options.contains(&"audio-only".to_string());
    let add_thumbnail = processing_options.contains(&"add-thumbnail".to_string());

    let output_path = if is_audio_only {
        // Download audio only
        info!(job_id = %job_id, "Downloading audio only");
        let output_filename = format!("{}.mp3", safe_title);
        let output_path = output_dir.join(&output_filename);

        send_progress_update(
            job_id,
            url,
            "downloading",
            0.3,
            "Starting audio download...",
        );

        // Build yt-dlp command for audio extraction with progress
        let mut cmd = Command::new("yt-dlp");
        cmd.arg("--extract-audio")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--audio-quality")
            .arg("0") // Best quality
            .arg("--embed-metadata")
            .arg("--newline") // Important for progress parsing
            .arg("--progress");

        // Add thumbnail embedding if requested
        if add_thumbnail {
            info!(job_id = %job_id, "Adding thumbnail embedding to download");
            cmd.arg("--embed-thumbnail");
        }

        // Execute the command with stdout/stderr capture for progress tracking
        cmd.arg("--output")
            .arg(output_path.to_string_lossy().to_string())
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Start the command
        let mut child = cmd.spawn().map_err(|e| {
            error!(error = %e, "Failed to execute yt-dlp command");
            PegasusError::ExternalCommandError(format!("Failed to execute yt-dlp command: {}", e))
        })?;

        // Track progress from stdout
        if let Some(stdout) = child.stdout.take() {
            let job_id = job_id.to_string();
            let url = url.to_string();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                parse_yt_dlp_progress(reader, &job_id, &url);
            });
        }

        // Track errors from stderr
        if let Some(stderr) = child.stderr.take() {
            let job_id = job_id.to_string();
            let url = url.to_string();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                for line in reader
                    .lines()
                    .map_while(|r: std::result::Result<String, std::io::Error>| r.ok())
                {
                    debug!("yt-dlp stderr: {}", line);
                    // Only send error messages to the client if they seem important
                    if line.contains("ERROR") {
                        send_progress_update(&job_id, &url, "warning", 0.0, &line);
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
            error!(job_id = %job_id, "yt-dlp command failed with status: {}", status);
            return Err(PegasusError::ExternalCommandError(format!(
                "yt-dlp command failed with status: {}",
                status
            )));
        }

        output_path
    } else {
        // Download full video
        info!(job_id = %job_id, "Downloading full video");
        let output_filename = format!("{}.mp4", safe_title);
        let output_path = output_dir.join(&output_filename);

        send_progress_update(
            job_id,
            url,
            "downloading",
            0.3,
            "Starting video download...",
        );

        // Build yt-dlp command for video download with progress
        let mut cmd = Command::new("yt-dlp");
        cmd.arg("-f")
            .arg("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best")
            .arg("--merge-output-format")
            .arg("mp4")
            .arg("--newline") // Important for progress parsing
            .arg("--progress");

        // Add thumbnail embedding if requested
        if add_thumbnail {
            info!(job_id = %job_id, "Adding thumbnail embedding to download");
            cmd.arg("--embed-thumbnail");
        }

        // Execute the command with stdout/stderr capture for progress tracking
        cmd.arg("--output")
            .arg(output_path.to_string_lossy().to_string())
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Start the command
        let mut child = cmd.spawn().map_err(|e| {
            error!(error = %e, "Failed to execute yt-dlp command");
            PegasusError::ExternalCommandError(format!("Failed to execute yt-dlp command: {}", e))
        })?;

        // Track progress from stdout
        if let Some(stdout) = child.stdout.take() {
            let job_id = job_id.to_string();
            let url = url.to_string();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                parse_yt_dlp_progress(reader, &job_id, &url);
            });
        }

        // Track errors from stderr
        if let Some(stderr) = child.stderr.take() {
            let job_id = job_id.to_string();
            let url = url.to_string();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                for line in reader
                    .lines()
                    .map_while(|r: std::result::Result<String, std::io::Error>| r.ok())
                {
                    debug!("yt-dlp stderr: {}", line);
                    // Only send error messages to the client if they seem important
                    if line.contains("ERROR") {
                        send_progress_update(&job_id, &url, "warning", 0.0, &line);
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
            error!(job_id = %job_id, "yt-dlp command failed with status: {}", status);
            return Err(PegasusError::ExternalCommandError(format!(
                "yt-dlp command failed with status: {}",
                status
            )));
        }

        output_path
    };

    info!(job_id = %job_id, file_path = %output_path.display(), "Download successful");
    Ok(output_path.display().to_string())
}

/// Parse yt-dlp progress output and send progress updates
///
/// This function parses the output from yt-dlp and extracts progress information,
/// then sends updates to connected clients via WebSockets.
fn parse_yt_dlp_progress<R: BufRead>(reader: R, job_id: &str, url: &str) {
    // Regex patterns for progress extraction
    let download_regex = Regex::new(r"\[download\]\s+([\d.]+)%").unwrap();
    let eta_regex = Regex::new(r"ETA\s+([\d:]+)").unwrap();
    let speed_regex = Regex::new(r"at\s+([\d.]+[KMGT]?iB/s)").unwrap();

    for line in reader
        .lines()
        .map_while(|r: std::result::Result<String, std::io::Error>| r.ok())
    {
        // Extract download percentage
        let mut progress = 0.0;
        let mut status = "processing".to_string(); // Default status
        let mut message_str = line.clone(); // Default message is the line itself

        if let Some(caps) = download_regex.captures(&line) {
            if let Some(progress_str) = caps.get(1) {
                if let Ok(progress_val) = progress_str.as_str().parse::<f32>() {
                    progress = progress_val / 100.0;
                    status = "downloading".to_string();
                    // Try to extract ETA and speed if progress is found
                    let eta = eta_regex
                        .captures(&line)
                        .and_then(|c| c.get(1))
                        .map_or("N/A", |m| m.as_str());
                    let speed = speed_regex
                        .captures(&line)
                        .and_then(|c| c.get(1))
                        .map_or("N/A", |m| m.as_str());
                    message_str = format!(
                        "Progress: {:.1}%, ETA: {}, Speed: {}",
                        progress * 100.0,
                        eta,
                        speed
                    );
                }
            }
        }

        // Send update if status changed to 'downloading' or if it's a different important message
        // Avoid sending every single line from yt-dlp if it's not a progress line.
        if status == "downloading" {
            send_progress_update(job_id, url, &status, progress, &message_str);
        }
    }
}
