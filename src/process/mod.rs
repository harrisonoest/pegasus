// src/process/mod.rs
// This module handles media processing using ffmpeg.

use crate::api::handlers::send_progress_update;
use crate::error::{PegasusError, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::task;
use tracing::{debug, error, info};

/// Media quality options for video and audio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaQuality {
    High,
    Medium,
    Low,
}

/// Audio format options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioFormat {
    MP3,
    M4A,
    Opus,
}

/// Media processing options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOptions {
    /// Whether to process as audio only
    pub audio_only: bool,
    /// Video quality (if video mode)
    pub video_quality: Option<MediaQuality>,
    /// Audio format (if audio only mode)
    pub audio_format: Option<AudioFormat>,
    /// Audio quality (if audio only mode)
    pub audio_quality: Option<MediaQuality>,
    /// Whether to embed metadata
    pub embed_metadata: bool,
    /// Whether to embed subtitles (video mode only)
    pub embed_subtitles: bool,
    /// Whether to embed thumbnail (audio mode only)
    pub embed_thumbnail: bool,
    /// Subtitle language (if embed_subtitles is true)
    pub subtitle_language: Option<String>,
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            audio_only: false,
            video_quality: Some(MediaQuality::High),
            audio_format: Some(AudioFormat::MP3),
            audio_quality: Some(MediaQuality::High),
            embed_metadata: true,
            embed_subtitles: false,
            embed_thumbnail: true,
            subtitle_language: None,
        }
    }
}

/// Process a media file using FFmpeg
///
/// # Arguments
///
/// * `input_path` - Path to the input media file
/// * `output_dir` - Directory where the processed file should be saved
/// * `options` - Processing options
/// * `job_id` - Unique identifier for this job
///
/// # Returns
///
/// A `Result` containing the path to the processed file
pub async fn process_media(
    input_path: &Path,
    output_dir: &Path,
    options: &ProcessingOptions,
    job_id: &str,
    url: &str,
) -> Result<PathBuf> {
    info!(
        job_id = %job_id,
        input = %input_path.display(),
        output_dir = %output_dir.display(),
        options = ?options,
        "Processing media with FFmpeg"
    );

    // Ensure output directory exists
    if !output_dir.exists() {
        tokio::fs::create_dir_all(output_dir).await.map_err(|e| {
            error!(error = %e, path = ?output_dir, "Failed to create output directory");
            PegasusError::IoError(e)
        })?;
    }

    // Send progress update
    send_progress_update(
        job_id,
        url,
        "processing",
        0.0,
        "Starting media processing...",
    );

    // Get file name without extension
    let file_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("processed_media");

    let output_path = if options.audio_only {
        // Process as audio
        process_audio(
            input_path,
            output_dir,
            file_stem,
            options.clone(),
            job_id,
            url,
        )
        .await?
    } else {
        // Process as video
        process_video(
            input_path,
            output_dir,
            file_stem,
            options.clone(),
            job_id,
            url,
        )
        .await?
    };

    // Send completion update
    send_progress_update(job_id, url, "processing", 1.0, "Media processing completed");

    Ok(output_path)
}

/// Process a media file as audio
async fn process_audio(
    input_path: &Path,
    output_dir: &Path,
    file_stem: &str,
    options: ProcessingOptions,
    job_id: &str,
    url: &str,
) -> Result<PathBuf> {
    // Determine audio format extension
    let extension = match options.audio_format {
        Some(AudioFormat::MP3) => "mp3",
        Some(AudioFormat::M4A) => "m4a",
        Some(AudioFormat::Opus) => "opus",
        None => "mp3", // Default to MP3
    };

    // Create output path
    let output_filename = format!("{}.{}", file_stem, extension);
    let output_path = output_dir.join(&output_filename);

    // Determine audio bitrate based on quality
    let bitrate = match options.audio_quality {
        Some(MediaQuality::High) => "320k",
        Some(MediaQuality::Medium) => "192k",
        Some(MediaQuality::Low) => "128k",
        None => "192k", // Default to medium quality
    };

    info!(
        job_id = %job_id,
        input = %input_path.display(),
        output = %output_path.display(),
        format = %extension,
        bitrate = %bitrate,
        "Processing audio"
    );

    // Send progress update
    send_progress_update(
        job_id,
        url,
        "processing",
        0.2,
        &format!("Converting to {} format...", extension.to_uppercase()),
    );

    // Build FFmpeg command
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i")
        .arg(input_path)
        .arg("-vn") // No video
        .arg("-b:a")
        .arg(bitrate);

    // Add codec based on format
    match options.audio_format {
        Some(AudioFormat::MP3) => {
            cmd.arg("-codec:a").arg("libmp3lame");
        }
        Some(AudioFormat::M4A) => {
            cmd.arg("-codec:a").arg("aac");
        }
        Some(AudioFormat::Opus) => {
            cmd.arg("-codec:a").arg("libopus");
        }
        None => {
            cmd.arg("-codec:a").arg("libmp3lame"); // Default to MP3
        }
    }

    // Add metadata embedding if requested
    if options.embed_metadata {
        cmd.arg("-map_metadata").arg("0");
    }

    // Add output path
    cmd.arg("-y") // Overwrite output file if it exists
        .arg(&output_path);

    // Run FFmpeg command
    let output_path_clone = output_path.clone();
    let job_id_clone = job_id.to_string();
    let url_clone = url.to_string();

    // Execute FFmpeg in a blocking task
    task::spawn_blocking(move || {
        // Execute the command
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                error!(error = %e, "Failed to execute FFmpeg command");
                PegasusError::ExternalCommandError(format!(
                    "Failed to execute FFmpeg command: {}",
                    e
                ))
            })?;

        // Track progress from stderr (FFmpeg outputs progress to stderr)
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            #[allow(clippy::manual_flatten)]
            for line_result in reader.lines() {
                if let Ok(line) = line_result {
                    debug!("FFmpeg: {}", line);

                    // Parse progress information
                    if line.contains("time=") {
                        if let Some(progress) = parse_ffmpeg_progress(&line) {
                            send_progress_update(
                                &job_id_clone,
                                &url_clone,
                                "processing",
                                0.2 + (progress * 0.7), // Scale to 20%-90% of the processing stage
                                &format!("Converting audio: {:.0}%", progress * 100.0),
                            );
                        }
                    }
                }
            }
        }

        // Wait for the command to complete
        let status = child.wait().map_err(|e| {
            error!(error = %e, "Failed to wait for FFmpeg command");
            PegasusError::ExternalCommandError(format!("Failed to wait for FFmpeg command: {}", e))
        })?;

        if !status.success() {
            error!(status = ?status, "FFmpeg command failed");
            return Err(PegasusError::ProcessingError(
                "FFmpeg command failed".to_string(),
            ));
        }

        // Process thumbnail if requested
        if options.embed_thumbnail {
            send_progress_update(
                &job_id_clone,
                &url_clone,
                "processing",
                0.9,
                "Adding thumbnail...",
            );

            // For MP3, we need to use a separate tool like AtomicParsley
            // For M4A/AAC, FFmpeg can handle it directly
            // This is a simplified version - in a real implementation, you'd extract the thumbnail first
        }

        Ok(output_path_clone)
    })
    .await
    .map_err(|e| {
        error!(error = %e, "Task join error");
        PegasusError::ProcessingError("Processing task failed".to_string())
    })?
}

/// Process a media file as video
async fn process_video(
    input_path: &Path,
    output_dir: &Path,
    file_stem: &str,
    options: ProcessingOptions,
    job_id: &str,
    url: &str,
) -> Result<PathBuf> {
    // Create output path
    let output_filename = format!("{}.mp4", file_stem);
    let output_path = output_dir.join(&output_filename);

    // Determine video quality settings
    let (video_preset, crf) = match options.video_quality {
        Some(MediaQuality::High) => ("slow", "18"),
        Some(MediaQuality::Medium) => ("medium", "23"),
        Some(MediaQuality::Low) => ("faster", "28"),
        None => ("medium", "23"), // Default to medium quality
    };

    info!(
        job_id = %job_id,
        input = %input_path.display(),
        output = %output_path.display(),
        preset = %video_preset,
        crf = %crf,
        "Processing video"
    );

    // Send progress update
    send_progress_update(job_id, url, "processing", 0.2, "Converting video format...");

    // Build FFmpeg command
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i")
        .arg(input_path)
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg(video_preset)
        .arg("-crf")
        .arg(crf)
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("192k");

    // Add metadata embedding if requested
    if options.embed_metadata {
        cmd.arg("-map_metadata").arg("0");
    }

    // Add subtitle embedding if requested
    if options.embed_subtitles {
        if let Some(lang) = &options.subtitle_language {
            // In a real implementation, you'd need to extract subtitles first or use yt-dlp to download them
            cmd.arg("-c:s").arg("mov_text");

            // Filter subtitles by language if specified
            cmd.arg("-metadata:s:s:0").arg(format!("language={}", lang));
        } else {
            cmd.arg("-c:s").arg("mov_text");
        }
    }

    // Add output path
    cmd.arg("-y") // Overwrite output file if it exists
        .arg(&output_path);

    // Run FFmpeg command
    let output_path_clone = output_path.clone();
    let job_id_clone = job_id.to_string();
    let url_clone = url.to_string();

    // Execute FFmpeg in a blocking task
    task::spawn_blocking(move || {
        // Execute the command
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                error!(error = %e, "Failed to execute FFmpeg command");
                PegasusError::ExternalCommandError(format!(
                    "Failed to execute FFmpeg command: {}",
                    e
                ))
            })?;

        // Track progress from stderr (FFmpeg outputs progress to stderr)
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            #[allow(clippy::manual_flatten)]
            for line_result in reader.lines() {
                if let Ok(line) = line_result {
                    debug!("FFmpeg: {}", line);

                    // Parse progress information
                    if line.contains("time=") {
                        if let Some(progress) = parse_ffmpeg_progress(&line) {
                            send_progress_update(
                                &job_id_clone,
                                &url_clone,
                                "processing",
                                0.2 + (progress * 0.8), // Scale to 20%-100% of the processing stage
                                &format!("Converting video: {:.0}%", progress * 100.0),
                            );
                        }
                    }
                }
            }
        }

        // Wait for the command to complete
        let status = child.wait().map_err(|e| {
            error!(error = %e, "Failed to wait for FFmpeg command");
            PegasusError::ExternalCommandError(format!("Failed to wait for FFmpeg command: {}", e))
        })?;

        if !status.success() {
            error!(status = ?status, "FFmpeg command failed");
            return Err(PegasusError::ProcessingError(
                "FFmpeg command failed".to_string(),
            ));
        }

        Ok(output_path_clone)
    })
    .await
    .map_err(|e| {
        error!(error = %e, "Task join error");
        PegasusError::ProcessingError("Processing task failed".to_string())
    })?
}

/// Parse FFmpeg progress output
///
/// FFmpeg outputs progress information in the format:
/// `frame=  123 fps= 25 q=29.0 size=    1234kB time=00:00:12.34 bitrate= 123.4kbits/s speed=1.23x`
///
/// This function extracts the time information and calculates progress as a value between 0.0 and 1.0.
fn parse_ffmpeg_progress(line: &str) -> Option<f32> {
    // Extract time information
    let time_part = line
        .split_whitespace()
        .find(|part| part.starts_with("time="))?;

    let time_str = time_part.trim_start_matches("time=");

    // Parse time in format HH:MM:SS.MS
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    let hours: f32 = parts[0].parse().ok()?;
    let minutes: f32 = parts[1].parse().ok()?;

    // The seconds part might contain milliseconds
    let seconds_parts: Vec<&str> = parts[2].split('.').collect();
    let seconds: f32 = seconds_parts[0].parse().ok()?;
    let milliseconds: f32 = if seconds_parts.len() > 1 {
        seconds_parts[1].parse().unwrap_or(0.0)
    } else {
        0.0
    };

    // Calculate total seconds
    let total_seconds = hours * 3600.0 + minutes * 60.0 + seconds + milliseconds / 1000.0;

    // Estimate progress based on time
    // This is a simplification - in a real implementation, you'd need to know the total duration
    // For now, we'll assume a 10-minute video as maximum
    let estimated_progress = (total_seconds / 600.0).min(1.0);

    Some(estimated_progress)
}

/// Get media information using FFmpeg
///
/// # Arguments
///
/// * `input_path` - Path to the media file
///
/// # Returns
///
/// A `Result` containing a JSON object with media information
pub async fn get_media_info(input_path: &Path) -> Result<serde_json::Value> {
    info!(
        input = %input_path.display(),
        "Getting media information with FFmpeg"
    );

    // Build FFmpeg command to get media information in JSON format
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(input_path)
        .output()
        .map_err(|e| {
            error!(error = %e, "Failed to execute ffprobe command");
            PegasusError::ExternalCommandError(format!("Failed to execute ffprobe command: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(stderr = %stderr, "ffprobe command failed");
        return Err(PegasusError::ExternalCommandError(format!(
            "ffprobe command failed: {}",
            stderr
        )));
    }

    // Parse the JSON output
    let json_str = String::from_utf8_lossy(&output.stdout);
    let media_info: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
        error!(error = %e, "Failed to parse ffprobe JSON output");
        PegasusError::ExternalCommandError(format!("Failed to parse ffprobe JSON output: {}", e))
    })?;

    Ok(media_info)
}
