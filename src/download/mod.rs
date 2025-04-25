// src/download/mod.rs

use crate::error::{PegasusError, Result};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};
use yt_dlp::Youtube;
use yt_dlp::fetcher::deps::Libraries;
use yt_dlp::model::{AudioCodecPreference, AudioQuality};

/// Downloads media using the yt_dlp crate.
///
/// # Arguments
///
/// * `url` - The URL of the media to download.
/// * `output_dir` - The directory where the downloaded file should be saved.
/// * `processing_options` - A slice of strings representing the desired processing options (e.g., "audio-only", "metadata").
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
    info!(url = %url, output_path = ?output_dir, options = ?processing_options, "Attempting to download using yt_dlp crate");

    // Ensure the output directory exists, creating it if necessary.
    if !output_dir.exists() {
        info!(path = ?output_dir, "Output directory does not exist, creating it.");
        tokio::fs::create_dir_all(output_dir).await.map_err(|e| {
            error!(error = %e, path = ?output_dir, "Failed to create output directory");
            PegasusError::IoError(e)
        })?;
    }

    let libraries_dir = PathBuf::from("libs");
    let output_dir = PathBuf::from(output_dir);

    let youtube = libraries_dir.join("yt-dlp");
    let ffmpeg = libraries_dir.join("ffmpeg");

    let libraries = Libraries::new(youtube, ffmpeg);
    let fetcher = Youtube::new(libraries, output_dir)?;

    // Execute the download asynchronously
    info!("Starting yt_dlp download...");

    // Download just the audio stream with high quality and MP3 codec
    // Check if "audio-only" is in the processing options
    if processing_options.contains(&"audio-only".to_string()) {
        let video = fetcher.fetch_video_infos(String::from(url)).await?;

        // Download audio-only with high quality and MP3 codec
        let audio_stream_path = fetcher
            .download_audio_stream_with_quality(
                url,
                format!("{}.mp3", video.title),
                AudioQuality::High,
                AudioCodecPreference::MP3,
            )
            .await;

        match audio_stream_path {
            Ok(filename) => {
                info!(file_path = %filename.display(), "Audio-only download successful");
                Ok(filename.display().to_string())
            }
            Err(e) => {
                error!(error = %e, "Audio-only download failed");
                Err(PegasusError::YtDlpError(e))
            }
        }
    } else {
        // Download full video (default behavior)
        let video = fetcher.fetch_video_infos(String::from(url)).await?;
        let video_path = fetcher
            .download_video_from_url(String::from(url), video.title)
            .await;

        match video_path {
            Ok(filename) => {
                info!(file_path = %filename.display(), "Video download successful");
                Ok(filename.display().to_string())
            }
            Err(e) => {
                error!(error = %e, "Video download failed");
                Err(PegasusError::YtDlpError(e))
            }
        }
    }
}
