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
    // Function to show the progress indicator
    showProgressIndicator() {
      const statusDiv = document.getElementById('status');
      if (statusDiv) {
        statusDiv.style.display = 'block';
        this.toggleProgressIndicator(true);
      }
    },

    // Function to hide the progress indicator
    hideProgressIndicator() {
      const statusDiv = document.getElementById('status');
      if (statusDiv) {
        this.toggleProgressIndicator(false);
        // Don't hide the status div as it contains messages we want to keep visible
      }
    },
    // Initialize the UI based on the default state
    init() {
      this.updateOptionsVisibility();
      this.setupEventListeners();
      this.setupNavigation();
      this.setupUrlInput();
      this.showSection('download'); // Show download section by default
    },

    // Setup URL input with validation and clear functionality
    setupUrlInput() {
      const urlInput = elements.mediaUrlsInput;
      const formGroup = urlInput.closest('.form-group');

      // Create clear button
      const clearButton = document.createElement('button');
      clearButton.type = 'button';
      clearButton.className = 'btn-clear';
      clearButton.innerHTML = '&times;';
      clearButton.title = 'Clear all URLs';
      clearButton.style.display = 'none';

      // Insert clear button after the textarea
      formGroup.style.position = 'relative';
      urlInput.parentNode.insertBefore(clearButton, urlInput.nextSibling);

      // Toggle clear button based on input
      urlInput.addEventListener('input', () => {
        clearButton.style.display = urlInput.value.trim() ? 'block' : 'none';
        this.validateUrls();
      });

      // Clear button click handler
      clearButton.addEventListener('click', () => {
        urlInput.value = '';
        clearButton.style.display = 'none';
        this.validateUrls();
      });

      // Handle paste event for multiple URLs
      urlInput.addEventListener('paste', (e) => {
        // Let the paste happen first
        setTimeout(() => {
          this.validateUrls();
        }, 0);
      });
    },

    // Validate URLs in the textarea
    validateUrls() {
      const urls = utils.parseUrls(elements.mediaUrlsInput.value);
      const hasInvalidUrls = urls.some(url => !this.isValidUrl(url));

      elements.mediaUrlsInput.classList.toggle('invalid', hasInvalidUrls);

      // Update submit button state
      elements.submitBtn.disabled = hasInvalidUrls || urls.length === 0;
      elements.submitBtn.title = hasInvalidUrls
        ? 'Please fix invalid URLs'
        : urls.length === 0 ? 'Enter at least one URL' : '';

      return !hasInvalidUrls && urls.length > 0;
    },

    // Check if a URL is valid
    isValidUrl(string) {
      try {
        const url = new URL(string);
        return url.protocol === 'http:' || url.protocol === 'https:';
      } catch (_) {
        return false;
      }
    },

    // Setup navigation between sections
    setupNavigation() {
      const navLinks = document.querySelectorAll('.nav-link');
      navLinks.forEach(link => {
        link.addEventListener('click', (e) => {
          e.preventDefault();
          const section = link.getAttribute('data-section');
          this.showSection(section);

          // Update active state
          navLinks.forEach(l => l.classList.remove('active'));
          link.classList.add('active');
        });
      });
    },

    // Show a specific section and hide others
    showSection(sectionId) {
      // Hide all sections first
      document.querySelectorAll('.content-section').forEach(section => {
        section.style.display = 'none';
      });

      // Show the requested section
      const section = document.getElementById(`${sectionId}-section`);
      if (section) {
        section.style.display = 'block';
      }
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

    // Show status message with optional auto-clear
    showStatus(message, type = 'info', autoClear = 0) {
      const statusDiv = document.getElementById('progress-info');
      if (!statusDiv) return null;

      // Clear existing status if it's an info message (to avoid stacking multiple info messages)
      if (type === 'info') {
        const existingInfo = statusDiv.querySelector('.status-message.info');
        if (existingInfo) {
          existingInfo.remove();
        }
      }

      // Create status element
      const statusEl = document.createElement('div');
      statusEl.className = `status-message ${type}`;
      statusEl.innerHTML = `
        <span class="status-indicator ${type}"></span>
        <span class="message">${message}</span>
      `;

      // Add to container
      statusDiv.appendChild(statusEl);

      // Auto-scroll to bottom
      statusDiv.scrollTop = statusDiv.scrollHeight;

      // Auto-clear if specified
      if (autoClear > 0) {
        setTimeout(() => {
          if (statusEl.parentNode === statusDiv) {
            statusEl.remove();
          }
        }, autoClear);
      }

      return statusEl;
    },

    // Append a new status message without clearing existing ones
    appendStatus(message, type = 'info') {
      return this.showStatus(message, type);
    },

    // Clear all status messages
    clearStatus() {
      const statusDiv = document.getElementById('progress-info');
      if (statusDiv) {
        statusDiv.innerHTML = '';
      }
    },

    // Show or hide the progress indicator
    toggleProgressIndicator(show = true) {
      const container = document.querySelector('.global-progress-container');
      if (!container) return;

      if (show) {
        container.style.display = 'block';
        container.classList.add('active');
      } else {
        container.classList.remove('active');
        // Add a small delay before hiding to allow final animation to complete
        setTimeout(() => {
          container.style.display = 'none';
        }, 500);
      }
    },

    // Function to show the progress indicator
    showProgressIndicator() {
      this.toggleProgressIndicator(true);
    },

    // Function to hide the progress indicator
    hideProgressIndicator() {
      this.toggleProgressIndicator(false);
    },

    // Function to update the progress indicator with a specific percentage and status
    updateProgressIndicator(percent, status = '') {
      const container = document.querySelector('.global-progress-container');
      const indicator = document.querySelector('.progress-indicator');
      if (!indicator || !container) return;

      // Update width with smooth transition
      const boundedPercent = Math.min(100, Math.max(0, percent));
      indicator.style.transform = `scaleX(${boundedPercent / 100})`;

      // Update status classes
      indicator.className = 'progress-indicator';
      if (status) {
        indicator.classList.add(status);
        container.className = 'global-progress-container';
        container.classList.add(`status-${status}`);
      }

      // Add active class when in progress
      if (boundedPercent > 0 && boundedPercent < 100) {
        indicator.classList.add('active');
        container.classList.add('active');
      } else {
        indicator.classList.remove('active');
        container.classList.remove('active');
      }

      // Directory browser functionality
      const browseDirBtn = document.getElementById('browse-dir');
      const downloadDirInput = document.getElementById('default-download-dir');

      if (browseDirBtn && downloadDirInput) {
        browseDirBtn.addEventListener('click', async () => {
          try {
            // This would be replaced with actual Electron dialog in a desktop app
            // For web, we'll use a fallback input dialog
            const dir = await showDirectoryPicker({
              id: 'downloads',
              mode: 'readwrite',
              startIn: 'downloads'
            }).catch(err => {
              console.log('Directory picker was cancelled');
              return null;
            });

            if (dir) {
              downloadDirInput.value = dir.name || 'Selected Directory';
            }
          } catch (error) {
            console.error('Error selecting directory:', error);
            // Fallback for browsers that don't support the File System Access API
            const dir = prompt('Enter download directory path:');
            if (dir) {
              downloadDirInput.value = dir;
            }
          }
        });
      }

      // Theme management
      function applyTheme(theme) {
        const root = document.documentElement;
        // Remove all theme classes first
        root.removeAttribute('data-theme');

        if (theme !== 'system') {
          root.setAttribute('data-theme', theme);
        }

        // Apply the theme to the document
        if (theme === 'dark' || (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
          document.body.classList.add('dark-theme');
        } else {
          document.body.classList.remove('dark-theme');
        }

        localStorage.setItem('theme', theme);
      }

      // Initialize theme
      const savedTheme = localStorage.getItem('theme') || 'system';
      applyTheme(savedTheme);

      // Set the dropdown to match the saved theme
      const themeSelect = document.getElementById('theme-preference');
      if (themeSelect) {
        themeSelect.value = savedTheme;

        themeSelect.addEventListener('change', (e) => {
          applyTheme(e.target.value);
        });
      }

      // Handle system theme changes
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', e => {
        if (localStorage.getItem('theme') === 'system') {
          applyTheme('system');
        }
      });

      // Add completed class when at 100%
      if (boundedPercent >= 100) {
        indicator.classList.add('completed');
        setTimeout(() => {
          this.hideProgressIndicator();
        }, 1000);
      }
    },

    // Update progress info text
    updateProgressInfo(text) {
      const progressInfo = document.querySelector('.progress-info');
      if (progressInfo) {
        progressInfo.textContent = text;
      }
    },

    // Set form to loading state
    setFormLoading(isLoading) {
      const submitBtn = document.querySelector('button[type="submit"]');
      if (submitBtn) {
        submitBtn.disabled = isLoading;
        submitBtn.textContent = isLoading ? 'Processing...' : 'Download';
        submitBtn.classList.toggle('loading', isLoading);
      }
    },

    // Handle processing state
    setProcessingState(isProcessing) {
      const indicator = document.querySelector('.progress-indicator');
      if (!indicator) return;

      if (isProcessing) {
        indicator.classList.add('processing');
        indicator.classList.add('active');
      } else {
        indicator.classList.remove('processing');
        indicator.classList.remove('active');
      }
    },

    // Handle error state
    setErrorState() {
      this.updateProgressIndicator(0, 'error');
    },

    // Handle success state
    setSuccessState() {
      this.updateProgressIndicator(100, 'success');
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

      // Validate form before submission
      if (!uiManager.validateUrls()) {
        uiManager.showStatus("Please fix the invalid URLs before submitting", "error");
        return;
      }

      // Get form data
      const { urls, outputDir, selectedOptions } = utils.collectFormData();

      // Update UI for submission
      uiManager.setFormLoading(true);
      uiManager.showStatus(`Starting download of ${urls.length} item(s)...`, "info");
      uiManager.showProgressIndicator();

      // Clear previous results
      uiManager.clearStatus();

      // Initialize job tracking
      const tracker = jobTracker.init(urls.length);

      // Track successful and failed downloads
      let successCount = 0;
      let errorCount = 0;

      try {
        // Process each URL
        for (let i = 0; i < urls.length; i++) {
          const url = urls[i];
          const displayName = utils.getDisplayNameFromUrl(url);

          // Show progress for current item
          uiManager.updateProgressInfo(`Processing ${i + 1} of ${urls.length}: ${displayName}`);
          uiManager.updateProgressIndicator((i / urls.length) * 100, 'downloading');

          try {
            const response = await apiService.submitUrl(url, outputDir, selectedOptions);

            if (response.success) {
              successCount++;
              tracker.jobIdToUrl[response.job_id] = url;
              uiManager.appendStatus(`✅ Added to queue: ${displayName}`, 'success');
            } else {
              errorCount++;
              tracker.failedJobs++;
              uiManager.appendStatus(`❌ Failed to queue ${displayName}: ${response.error || 'Unknown error'}`, 'error');
            }
          } catch (error) {
            errorCount++;
            tracker.failedJobs++;
            console.error(`Error processing ${url}:`, error);
            uiManager.appendStatus(`❌ Error processing ${displayName}: ${error.message || 'Unknown error'}`, 'error');
          }
        }

        // Show completion summary
        if (successCount > 0) {
          uiManager.appendStatus(`\n🎉 Successfully queued ${successCount} item(s) for download.`, 'success');
          if (errorCount === 0) {
            uiManager.updateProgressIndicator(100, 'completed');
          }
        }

        if (errorCount > 0) {
          const message = `\n⚠️ Failed to queue ${errorCount} item(s). Check the logs for details.`;
          uiManager.appendStatus(message, 'warning');
          uiManager.updateProgressIndicator(
            Math.round((successCount / urls.length) * 100),
            errorCount === urls.length ? 'error' : 'warning'
          );
        }

        // If no errors, show waiting message
        if (errorCount === 0) {
          uiManager.updateProgressInfo('Waiting for downloads and conversions to finish...');
        }

      } catch (error) {
        console.error('Error in form processing:', error);
        uiManager.appendStatus(`❌ An unexpected error occurred: ${error.message || 'Please try again later.'}`, 'error');
        uiManager.updateProgressIndicator(0, 'error');
      } finally {
        uiManager.setFormLoading(false);
      }
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
