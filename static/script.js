/**
 * Pegasus Frontend Application
 * A modern web interface for media downloading and processing
 */

'use strict';

document.addEventListener("DOMContentLoaded", () => {
  const form = document.getElementById("uploadForm");
  const submitBtn = document.getElementById("submitBtn");
  const statusDiv = document.getElementById("status");
  const audioOnlyCheckbox = document.getElementById("audio-only");
  const videoOptionsSection = document.getElementById("video-options");
  const audioOptionsSection = document.getElementById("audio-options");

  // DOM elements cache
  const elements = {
    form: form,
    submitBtn: submitBtn,
    statusDiv: statusDiv,
    progressIndicator: statusDiv.querySelector('.progress-indicator'),
    audioOnlyCheckbox: audioOnlyCheckbox,
    videoOptionsSection: videoOptionsSection,
    audioOptionsSection: audioOptionsSection,
    mediaUrlsInput: document.getElementById("mediaUrls"),
    outputDirInput: document.getElementById("outputDir"),
    progressInfoDiv: document.getElementById('progress-info')
  };

  // Application state
  const state = {
    activeDownloads: new Map(),
    socket: null,
    isProcessing: false
  };

  /**
   * WebSocket connection management
   */
  const webSocketManager = {
    // Connect to WebSocket server
    connect() {
      // Get the current host and protocol
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = window.location.host;
      const wsUrl = `${protocol}//${host}/ws`;

      // Create WebSocket connection
      state.socket = new WebSocket(wsUrl);

      // Connection opened
      state.socket.addEventListener('open', (event) => {
        console.log('Connected to Pegasus WebSocket server');
      });

      // Listen for messages
      state.socket.addEventListener('message', (event) => {
        try {
          // Check if this is the welcome message
          if (event.data.startsWith('Connected to')) {
            console.log(event.data);
            return;
          }

          // Parse the progress update
          const update = JSON.parse(event.data);
          this.handleUpdate(update);
        } catch (error) {
          console.error('Error handling WebSocket message:', error);
        }
      });

      // Connection closed
      state.socket.addEventListener('close', (event) => {
        console.log('Disconnected from Pegasus WebSocket server');
        // Try to reconnect after a delay
        setTimeout(() => this.connect(), 3000);
      });

      // Connection error
      state.socket.addEventListener('error', (event) => {
        console.error('WebSocket error:', event);
      });
    },

    // Handle progress updates from WebSocket
    handleUpdate(update) {
      // Only handle updates for tracked jobs
      if (!window.pegasusJobTracker || !update.job_id) return;
      const tracker = window.pegasusJobTracker;

      if (update.status === 'completed') {
        if (tracker.jobIdToUrl[update.job_id]) {
          tracker.completedJobs++;
          tracker.updateProgress();
          // Optionally show a per-job status
          uiManager.appendStatus(`Completed: ${tracker.jobIdToUrl[update.job_id]}`, 'success');
          delete tracker.jobIdToUrl[update.job_id];
        }
      } else if (update.status === 'error') {
        tracker.failedJobs++;
        tracker.updateProgress();
        uiManager.appendStatus(`Error: ${update.message || update.job_id}`, 'error');
      } else if (update.status === 'downloading' && update.progress) {
        // Optionally: update the progress bar for the current job only if desired
        // For global progress, we only update on completion
      } else {
        // For other statuses, show indeterminate progress
        uiManager.showProgressIndicator();
      }
    }
  };

  /**
   * Utility functions
   */
  const utils = {
    // Helper function to get a display name from URL
    getDisplayNameFromUrl(url) {
      try {
        const urlObj = new URL(url);

        // Check for YouTube or other video platforms
        if (urlObj.hostname.includes('youtube.com') || urlObj.hostname.includes('youtu.be')) {
          return 'YouTube Video';
        } else if (urlObj.hostname.includes('vimeo.com')) {
          return 'Vimeo Video';
        } else if (urlObj.hostname.includes('soundcloud.com')) {
          return 'SoundCloud Track';
        }

        // Try to get filename from path
        const pathParts = urlObj.pathname.split('/');
        const lastPart = pathParts[pathParts.length - 1];

        if (lastPart && lastPart.length > 0 && lastPart !== '/') {
          // Remove extension and replace dashes/underscores with spaces
          return lastPart.split('.')[0].replace(/[-_]/g, ' ');
        }

        // Fallback to hostname
        return urlObj.hostname;
      } catch (e) {
        // If URL parsing fails, return a portion of the URL
        return url.substring(0, 30) + '...';
      }
    },

    // Parse URLs from textarea
    parseUrls(rawText) {
      return rawText.split('\n')
        .map(url => url.trim())
        .filter(url => url !== "");
    },

    // Collect form data
    collectFormData() {
      const mediaUrlsRaw = elements.mediaUrlsInput.value;
      const outputDir = elements.outputDirInput.value || "/tmp/pegasus_downloads";
      const urls = this.parseUrls(mediaUrlsRaw);

      // Get selected options (these apply to all URLs)
      const selectedOptions = [];
      document
        .querySelectorAll('input[name="options"]:checked')
        .forEach((option) => {
          selectedOptions.push(option.value);
        });

      if (elements.audioOnlyCheckbox.checked) {
        const audioFormat = document.getElementById('audio-format').value;
        const audioQuality = document.getElementById('audio-quality').value;
        selectedOptions.push(audioFormat);
        selectedOptions.push(audioQuality);
      } else {
        const videoQuality = document.getElementById('video-quality').value;
        selectedOptions.push(videoQuality);
      }

      return { urls, outputDir, selectedOptions };
    }
  }

  /**
   * UI Manager - handles all UI updates and interactions
   */
  const uiManager = {
    // Initialize the UI based on the default state
    init() {
      this.updateOptionsVisibility();
      this.setupEventListeners();
    },

    // Setup event listeners
    setupEventListeners() {
      elements.audioOnlyCheckbox.addEventListener("change", () => this.updateOptionsVisibility());
    },

    // Function to update the visibility of options sections
    updateOptionsVisibility() {
      if (elements.audioOnlyCheckbox.checked) {
        elements.videoOptionsSection.style.display = "none";
        elements.audioOptionsSection.style.display = "block";
      } else {
        elements.videoOptionsSection.style.display = "block";
        elements.audioOptionsSection.style.display = "none";
      }
    },

    // Show status message
    showStatus(message, type) {
      // Just update the progress indicator based on the type
      elements.statusDiv.className = "status"; // Base class
      if (type) {
        elements.statusDiv.classList.add(type);
      }
      elements.statusDiv.style.display = "block";

      // Show the progress indicator with appropriate status
      if (type === 'error') {
        this.updateProgressIndicator(35, 'error');
      } else if (type === 'success') {
        this.updateProgressIndicator(100, 'success');
      } else {
        this.showProgressIndicator();
      }
    },

    // Helper function that now just updates the progress indicator
    appendStatus(message, type) {
      // Just ensure the status div is visible
      elements.statusDiv.style.display = "block";

      // Update the progress indicator based on the type
      if (type === 'error') {
        // Don't change the progress percentage, just the color and animation
        const currentWidth = elements.progressIndicator.style.width;
        const percent = parseInt(currentWidth) || 35;
        this.updateProgressIndicator(percent, 'error');
      } else if (type === 'success') {
        // Don't update to 100% for individual successes, just keep the current progress
      }
    },

    // Function to show the progress indicator
    showProgressIndicator() {
      if (elements.progressIndicator) {
        elements.progressIndicator.style.display = 'block';
        elements.progressIndicator.style.width = '35%'; // Default animation width
      }
    },

    // Function to update the progress indicator with a specific percentage and status
    updateProgressIndicator(percent, status) {
      if (elements.progressIndicator) {
        // If percent is 100, we're done, so remove the animation
        if (percent >= 100) {
          elements.progressIndicator.style.width = '100%';
          elements.progressIndicator.style.animation = 'none';
          elements.progressIndicator.style.backgroundColor = 'var(--success-color)';
          // Add a subtle transition effect
          elements.progressIndicator.style.transition = 'width 0.5s ease-out, background-color 0.5s';
        } else if (percent <= 0) {
          // If starting or error, show the animated version
          elements.progressIndicator.style.width = '35%';
          elements.progressIndicator.style.animation = 'progressAnimation 1.5s linear infinite';
          elements.progressIndicator.style.backgroundColor = 'var(--accent-color)';
        } else {
          // Otherwise show the actual percentage with a smooth transition
          elements.progressIndicator.style.width = `${percent}%`;
          elements.progressIndicator.style.transition = 'width 0.3s ease-out';

          // Set color based on status
          if (status === 'error') {
            elements.progressIndicator.style.backgroundColor = 'var(--error-color)';
            elements.progressIndicator.style.animation = 'none';
          } else if (status === 'processing') {
            elements.progressIndicator.style.backgroundColor = 'var(--accent-color)';
            // Keep the animation for processing state
            elements.progressIndicator.style.animation = 'progressAnimation 1.5s linear infinite';
          } else {
            elements.progressIndicator.style.animation = 'none';
          }
        }
      }
    },

    // Function to hide the progress indicator
    hideProgressIndicator() {
      if (elements.progressIndicator) {
        elements.progressIndicator.style.display = 'none';
      }
    },

    // Update progress info text
    updateProgressInfo(text) {
      if (elements.progressInfoDiv) {
        elements.progressInfoDiv.textContent = text;
      }
    },

    // Set form to loading state
    setFormLoading(isLoading) {
      elements.submitBtn.disabled = isLoading;
      elements.submitBtn.textContent = isLoading ? "Processing..." : "Submit";
      state.isProcessing = isLoading;
    }
  };

  /**
   * API Service - handles all API interactions
   */
  const apiService = {
    // Submit a single URL for processing
    async submitUrl(url, outputDir, processingOptions) {
      try {
        const data = {
          mediaUrl: url,
          outputDir: outputDir,
          processingOptions: processingOptions,
        };

        const response = await fetch("/api/submit", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(data),
        });

        if (!response.ok) {
          const error = await response.json();
          throw new Error(error.message || 'Unknown server error');
        }

        return await response.json();
      } catch (error) {
        throw error;
      }
    }
  };

  /**
   * Job Tracker - manages job tracking and progress
   */
  const jobTracker = {
    // Initialize job tracker
    init(totalJobs) {
      window.pegasusJobTracker = {
        totalJobs,
        completedJobs: 0,
        failedJobs: 0,
        jobIdToUrl: {},
        updateProgress: function () {
          const percent = Math.round((this.completedJobs / this.totalJobs) * 100);
          let status = (this.failedJobs > 0) ? 'error' : (percent === 100 ? 'success' : 'processing');
          uiManager.updateProgressIndicator(percent, status);

          if (status === 'success') {
            uiManager.updateProgressInfo('All downloads and conversions complete!');
          } else if (status === 'error') {
            uiManager.updateProgressInfo('Some downloads failed. See details below.');
          } else {
            uiManager.updateProgressInfo(`In progress: ${this.completedJobs} of ${this.totalJobs} completed...`);
          }
        }
      };
      return window.pegasusJobTracker;
    }
  };

  /**
   * Form Handler - manages form submission and processing
   */
  const formHandler = {
    // Process form submission
    async processForm(e) {
      e.preventDefault();

      // Get form data
      const { urls, outputDir, selectedOptions } = utils.collectFormData();

      // Validate form
      if (urls.length === 0) {
        uiManager.showStatus("Please enter at least one valid media URL in the list.", "error");
        return;
      }

      // Show loading state
      uiManager.setFormLoading(true);

      // Show and initialize progress indicator
      uiManager.showProgressIndicator();
      uiManager.updateProgressIndicator(0); // Start with 0%

      if (!elements.outputDirInput.value && outputDir === "/tmp/pegasus_downloads") {
        uiManager.appendStatus(`No output directory specified. Using default: ${outputDir}`, "info");
      }

      // Initialize job tracking
      const tracker = jobTracker.init(urls.length);
      uiManager.updateProgressInfo('Submitting jobs to backend...');

      // Process each URL
      for (let i = 0; i < urls.length; i++) {
        const currentUrl = urls[i];
        uiManager.appendStatus(`(${i + 1}/${urls.length}) Processing: ${currentUrl}`, "info");

        try {
          // Submit URL to API
          const result = await apiService.submitUrl(currentUrl, outputDir, selectedOptions);
          uiManager.appendStatus(`(${i + 1}/${urls.length}) Success for ${currentUrl} - Job ID: ${result.job_id}`, "success");
          tracker.jobIdToUrl[result.job_id] = currentUrl;
        } catch (error) {
          tracker.failedJobs++;
          uiManager.appendStatus(`(${i + 1}/${urls.length}) Error for ${currentUrl}: ${error.message}`, "error");
        }
      }

      // Update progress info
      uiManager.updateProgressInfo('Waiting for downloads and conversions to finish...');

      // Reset button state
      uiManager.setFormLoading(false);
    }
  };

  // Initialize application
  function initApp() {
    // Initialize UI
    uiManager.init();

    // Initialize WebSocket connection
    webSocketManager.connect();

    // Setup form submission handler
    elements.form.addEventListener("submit", (e) => formHandler.processForm(e));
  }

  // Start the application
  initApp();
});
