# Pegasus

An all-in-one media downloader and processor.

## Getting Started

This guide will help you get Pegasus up and running on your system.

### Prerequisites

Before you begin, ensure you have the following installed:

-   Git
-   Rust (latest stable version recommended)
-   Docker (optional, for containerized deployment)
-   yt-dlp (must be in PATH if not using Docker)
-   FFmpeg (must be in PATH if not using Docker)

### Using Docker

Using Docker is the recommended way to get started, as it handles most dependencies automatically.

1.  **Clone the repository:**
    ```bash
    git clone <repository_url>
    ```
2.  **Navigate to the project directory:**
    ```bash
    cd pegasus
    ```
3.  **Configure environment variables:**
    Create a `.env` file by copying `.env.sample`:
    ```bash
    cp .env.sample .env
    ```
    Then, edit the `.env` file to set your desired configuration (e.g., API keys, download paths). Refer to `.env.sample` for all available options.

4.  **Build the Docker image:**
    ```bash
    docker build -t pegasus .
    ```
5.  **Run the Docker container:**
    ```bash
    docker run -p 8000:8000 --env-file .env pegasus
    ```
    To run the container in detached mode (in the background), add the `-d` flag:
    ```bash
    docker run -d -p 8000:8000 --env-file .env pegasus
    ```

### Local Installation (without Docker)

If you prefer not to use Docker, you can install and run Pegasus locally. This requires manual installation of Rust, yt-dlp, and FFmpeg.

1.  **Clone the repository:**
    ```bash
    git clone <repository_url>
    ```
2.  **Navigate to the project directory:**
    ```bash
    cd pegasus
    ```
3.  **Install dependencies:**
    Ensure `yt-dlp` and `ffmpeg` are installed and accessible in your system's PATH. You'll also need the Rust toolchain.

4.  **Configure environment variables:**
    Create a `.env` file by copying `.env.sample`:
    ```bash
    cp .env.sample .env
    ```
    Then, edit the `.env` file to set your desired configuration. Refer to `.env.sample` for all available options and necessary settings (like paths if not using defaults).

5.  **Build the project:**
    ```bash
    cargo build --release
    ```
6.  **Run the application:**
    ```bash
    cargo run --release
    ```

## Overview

### Detailed Description

Pegasus is an all-in-one media downloader and processor. It allows users to download and process media files from various sources, such as YouTube and SoundCloud. It also allows users to transfer the processed files to a media server.

### Frontend

Pegasus uses a basic HTML, CSS, and JavaScript frontend. It has a dark mode theme, and is mobile-friendly.

### Backend

Pegasus uses a Rust backend. It has a WebSocket server that handles communication between the frontend and backend.

### Core Functionality

Pegasus has a web interface that allows users to input multiple media URLs, select options, and start the download. The application then downloads the media files, processes them, and transfers them to a media server.

### Download Options

Pegasus allows users to download media files in two modes:

- Video mode: Download the media file as a video.
- Audio mode: Download the media file as an audio file.

#### Video Mode

In video mode, users can select the video quality and whether to embed metadata and subtitles.

#### Audio Mode

In audio mode, users can select the audio format and quality, and whether to embed metadata and add a thumbnail.

### Transfer Options

Pegasus transfers the processed files to a media server using a WebSocket connection. The media server is a simple HTTP server that serves the files from a directory.

### Progress Updates

Pegasus provides progress updates to the user through a WebSocket connection. The progress updates include the status of each file being downloaded, processed, and transferred.

### Error Handling

Pegasus provides error handling and reporting to the user through a WebSocket connection. The error handling includes error messages and warnings to the user.

### Security

Pegasus is designed to be secure. It uses a WebSocket connection to transfer data between the client and server. The server is protected by a firewall and only allows connections from trusted sources. The server also has rate limiting and request validation to prevent abuse.

### yt-dlp

Pegasus uses yt-dlp to download media files. It is a command-line program that downloads media files from the internet. It is a fork of youtube-dl, and is used to download media files from YouTube and other video platforms. More information about the yt-dlp project is available at https://github.com/yt-dlp/yt-dlp.

### FFmpeg

Pegasus uses FFmpeg to process media files. It is a command-line program that processes media files. It is used to convert media files to different formats, and to add metadata and subtitles to media files. More information about the ffmpeg project is available at https://ffmpeg.org/.

## Usage

Once Pegasus is running (either via Docker or a local installation as described in the "Getting Started" section), you can access the web interface to manage your downloads.

By default, the web interface is available at: `http://localhost:8000`

Through the interface, you can perform the following main actions:

-   **Input Media URLs:** Paste one or more URLs for the media you wish to download.
-   **Select Download Mode:** Choose between 'Video mode' to download the full video or 'Audio mode' to extract and download only the audio.
-   **Configure Options:**
    -   For video: Select video quality, and choose whether to embed metadata and subtitles.
    -   For audio: Select audio format, quality, and choose whether to embed metadata and add a thumbnail.
-   **Start Processing:** Initiate the download and processing of the media files based on your selected options.

The interface will provide progress updates for each file and report any errors encountered during the process. For more detailed information on these features, please refer to the "Overview" section.

## Project Structure


```
pegasus/
├── Cargo.toml
├── Dockerfile
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   ├── api/
│   │   ├── mod.rs
│   │   └── handlers.rs
│   ├── download/
│   │   └── mod.rs
│   ├── process/
│   │   └── mod.rs
│   ├── transfer/
│   │   └── mod.rs
├── static/
│   ├── index.html
│   ├── styles.css
│   ├── script.js
│   └── pegasus.svg
└── .dockerignore
```

## Contributing

Contributions are welcome and greatly appreciated! If you have an idea for a new feature, a bug fix, or an improvement, please feel free to contribute.

To contribute, please follow these general guidelines:

1.  **Fork the repository.**
2.  **Create a new branch** for your feature or bug fix. It's good practice to name your branch descriptively, for example:
    -   For features: `git checkout -b feature/your-amazing-feature-name`
    -   For bug fixes: `git checkout -b bugfix/issue-number-or-description`
3.  **Make your changes** and commit them with clear, concise commit messages.
4.  **Push your changes** to your forked repository.
5.  **Create a pull request** to the `main` branch of the main Pegasus repository.

If you're planning to make a larger change, it's a good idea to **open an issue first** to discuss your ideas and ensure it aligns with the project's goals.

## License

License information TBD.
