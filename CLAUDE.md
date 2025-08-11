# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Pegasus is an all-in-one media downloader and processor built in Rust. It downloads media files from sources like YouTube and SoundCloud, processes them with options for video/audio extraction, and provides a web interface for management.

### Architecture

- **Backend**: Rust-based web server using Axum framework with WebSocket support
- **Frontend**: Static HTML/CSS/JavaScript with WebSocket for real-time updates
- **External Dependencies**: Requires `yt-dlp` and `ffmpeg` in PATH (or available in Docker)
- **Async Architecture**: Uses Tokio for async runtime, broadcast channels for progress updates

### Core Components

- `src/main.rs` - Entry point, server initialization, binary dependency checks
- `src/api/` - HTTP handlers and WebSocket connection management  
- `src/queue_manager.rs` - Central download queue management with worker tasks
- `src/task_manager.rs` - Manages active child processes (yt-dlp, ffmpeg) and cancellation
- `src/download/` - Download job definitions and processing logic
- `src/process/` - Media processing with ffmpeg
- `src/transfer/` - File transfer functionality
- `static/` - Web frontend files (HTML, CSS, JS)

## Development Commands

### Building and Running
- `cargo build` - Build the project
- `cargo build --release` - Build optimized release version
- `cargo run` - Run in development mode
- `cargo run --release` - Run optimized version
- `cargo check` - Check compilation without building

### Docker Development
- `docker build -t pegasus .` - Build Docker image
- `docker run -p 8000:8000 --env-file .env pegasus` - Run container

### Configuration
- Copy `.env.sample` to `.env` for environment configuration
- Default server runs on port 8000 (configurable via SERVER_PORT env var)
- Requires `yt-dlp` and `ffmpeg` binaries in PATH for local development

## Key Technical Details

### State Management
- Uses `DashMap` for concurrent job storage across async tasks
- Broadcast channels (`tokio::sync::broadcast`) for real-time progress updates to WebSocket clients
- `TaskManager` tracks active child processes with cancellation support

### Error Handling
- Custom error type `PegasusError` in `src/error.rs`
- Structured logging with `tracing` crate
- WebSocket error handling for client disconnections

### Processing Flow
1. Frontend submits URL via `/api/submit` endpoint
2. Job added to `DownloadQueue` with unique ID
3. Background worker processes jobs using `yt-dlp` and `ffmpeg`
4. Progress updates sent via WebSocket to connected clients
5. Completed files available in output directory

### WebSocket Protocol
- Progress updates are JSON-serialized `ProgressUpdate` structs
- Includes job_id, URL, status, progress percentage, and messages
- Handles client disconnection gracefully