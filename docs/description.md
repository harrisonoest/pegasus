# Detailed Description

Pegasus is an all-in-one media downloader and processor. It allows users to download and process media files from various sources, such as YouTube and SoundCloud. It also allows users to transfer the processed files to a media server.

## Frontend

Pegasus uses a basic HTML, CSS, and JavaScript frontend. It has a dark mode theme, and is mobile-friendly.

## Backend

Pegasus uses a Rust backend. It has a WebSocket server that handles communication between the frontend and backend.

## Core Functionality

Pegasus has a web interface that allows users to input multiple media URLs, select options, and start the download. The application then downloads the media files, processes them, and transfers them to a media server.

## Download Options

Pegasus allows users to download media files in two modes:

- Video mode: Download the media file as a video.
- Audio mode: Download the media file as an audio file.

### Video Mode

In video mode, users can select the video quality and whether to embed metadata and subtitles.

### Audio Mode

In audio mode, users can select the audio format and quality, and whether to embed metadata and add a thumbnail.

## Transfer Options

Pegasus transfers the processed files to a media server using a WebSocket connection. The media server is a simple HTTP server that serves the files from a directory.

## Progress Updates

Pegasus provides progress updates to the user through a WebSocket connection. The progress updates include the status of each file being downloaded, processed, and transferred.

## Error Handling

Pegasus provides error handling and reporting to the user through a WebSocket connection. The error handling includes error messages and warnings to the user.

## Security

Pegasus is designed to be secure. It uses a WebSocket connection to transfer data between the client and server. The server is protected by a firewall and only allows connections from trusted sources. The server also has rate limiting and request validation to prevent abuse.

## yt-dlp

Pegasus uses yt-dlp to download media files. It is a command-line program that downloads media files from the internet. It is a fork of youtube-dl, and is used to download media files from YouTube and other video platforms. More information about the yt-dlp project is available at https://github.com/yt-dlp/yt-dlp.

## FFmpeg

Pegasus uses FFmpeg to process media files. It is a command-line program that processes media files. It is used to convert media files to different formats, and to add metadata and subtitles to media files. More information about the ffmpeg project is available at https://ffmpeg.org/.
