# Stage 1: Builder
# Use the official Rust image as the base for building
FROM rust:1-slim-bookworm as builder

# Set the working directory
WORKDIR /usr/src/pegasus

# Install build dependencies (like openssl, pkg-config for some crates)
# Update package list and install dependencies
RUN apt-get update &&
  apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates &&
  # Clean up apt cache
  rm -rf /var/lib/apt/lists/*

# Copy the Cargo configuration files
COPY Cargo.toml Cargo.lock ./

# Build dependencies first to leverage Docker cache
# Create a dummy main.rs to build only dependencies
RUN mkdir src && echo "fn main() {}" >src/main.rs
# Build only the dependencies to cache them
RUN cargo build --release --locked
# Remove dummy source file
RUN rm -f src/main.rs

# Copy the application source code
COPY src ./src
# Copy static frontend files
COPY static ./static

# Build the application executable
# Touch source files to ensure rebuild if needed
RUN touch src/main.rs
RUN cargo build --release --locked

# Stage 2: Runtime
# Use a minimal base image for the final container
FROM debian:bookworm-slim as runtime

# Set the working directory
WORKDIR /usr/local/pegasus

# Install runtime dependencies: yt-dlp and ffmpeg
# Update package list and install dependencies
RUN apt-get update &&
  apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    ffmpeg \
    ca-certificates &&
  # Install yt-dlp using pip
  pip3 install --no-cache-dir yt-dlp &&
  # Clean up apt cache
  rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/pegasus/target/release/pegasus .

# Copy the static files from the builder stage
COPY --from=builder /usr/src/pegasus/static ./static

# Define environment variables (optional, can be overridden)
# ENV ROCKET_ADDRESS=0.0.0.0
# ENV ROCKET_PORT=8000
# ENV MEDIA_SERVER_PATH=/media
# ENV DOWNLOAD_DIR=/data/downloads
# ENV PROCESSED_DIR=/data/processed

# Expose the port the application will run on (adjust if needed)
EXPOSE 8000

# Set the entrypoint for the container
# The application binary will be executed when the container starts
ENTRYPOINT ["./pegasus"]
