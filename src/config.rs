// src/config.rs
// Handles application configuration.

// Example struct (to be replaced)
pub struct Config {
    pub media_server_path: String,
    pub download_dir: String,
    pub processed_dir: String,
}

impl Config {
    pub fn load() -> Self {
        // TODO: Load configuration from file or environment variables
        // Use tracing::info! instead of println!
        tracing::info!("Loading configuration (placeholder)");
        Config {
            media_server_path: "/path/to/media/server".to_string(),
            download_dir: "/tmp/pegasus/downloads".to_string(),
            processed_dir: "/tmp/pegasus/processed".to_string(),
        }
    }
}
