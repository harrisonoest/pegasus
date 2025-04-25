# Build stage
FROM rust:1.82 AS builder

WORKDIR /app

# Install system dependencies for media processing
RUN apt-get update && apt-get install -y \
  ffmpeg \
  python3 \
  python3-pip &&
  pip3 install yt-dlp

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Cache dependencies
RUN mkdir src &&
  echo "fn main() {}" >src/main.rs &&
  cargo build --release &&
  rm -rf src

# Copy source code
COPY src ./src
COPY static ./static

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
  ffmpeg \
  python3 \
  python3-pip \
  ca-certificates &&
  pip3 install yt-dlp &&
  apt-get clean &&
  rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built executable from the builder stage
COPY --from=builder /app/target/release/pegasus /app/pegasus

# Copy static files
COPY --from=builder /app/static /app/static

# Create media directories
RUN mkdir -p /app/media/downloads /app/media/processed

# Expose the web server port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV MEDIA_PATH=/app/media

# Run the application
CMD ["/app/pegasus"]
