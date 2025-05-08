document.addEventListener("DOMContentLoaded", function () {
  const form = document.getElementById("uploadForm");
  const submitBtn = document.getElementById("submitBtn");
  const statusDiv = document.getElementById("status");
  const audioOnlyCheckbox = document.getElementById("audio-only");
  const videoOptionsSection = document.getElementById("video-options");
  const audioOptionsSection = document.getElementById("audio-options");

  // Create separate containers for different types of status messages
  const formStatusDiv = document.createElement('div');
  formStatusDiv.id = 'form-status';
  statusDiv.appendChild(formStatusDiv);

  const progressStatusDiv = document.createElement('div');
  progressStatusDiv.id = 'progress-status';
  progressStatusDiv.style.marginBottom = '20px'; // Add margin below progress updates
  statusDiv.appendChild(progressStatusDiv);

  // Map to store active downloads and their progress elements
  const activeDownloads = new Map();

  // WebSocket connection for real-time progress updates
  let socket = null;

  // Connect to WebSocket server
  function connectWebSocket() {
    // Get the current host and protocol
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const wsUrl = `${protocol}//${host}/ws`;

    // Create WebSocket connection
    socket = new WebSocket(wsUrl);

    // Connection opened
    socket.addEventListener('open', (event) => {
      console.log('Connected to Pegasus WebSocket server');
    });

    // Listen for messages
    socket.addEventListener('message', (event) => {
      try {
        // Check if this is the welcome message
        if (event.data.startsWith('Connected to')) {
          console.log(event.data);
          return;
        }

        // Parse the progress update
        const update = JSON.parse(event.data);
        handleProgressUpdate(update);
      } catch (error) {
        console.error('Error handling WebSocket message:', error);
      }
    });

    // Connection closed
    socket.addEventListener('close', (event) => {
      console.log('Disconnected from Pegasus WebSocket server');
      // Try to reconnect after a delay
      setTimeout(connectWebSocket, 3000);
    });

    // Connection error
    socket.addEventListener('error', (event) => {
      console.error('WebSocket error:', event);
    });
  }

  // Handle progress updates from WebSocket
  function handleProgressUpdate(update) {
    // Get or create progress element for this job
    let progressElement = activeDownloads.get(update.job_id);

    if (!progressElement) {
      // Create new progress element if this is a new job
      progressElement = createProgressElement(update);
      activeDownloads.set(update.job_id, progressElement);
      progressStatusDiv.appendChild(progressElement.container);
    }

    // Update the progress element
    updateProgressElement(progressElement, update);

    // If download is complete or failed, remove from active downloads after a delay
    if (update.status === 'completed' || update.status === 'error') {
      setTimeout(() => {
        activeDownloads.delete(update.job_id);
      }, 60000); // Keep completed downloads visible for 1 minute
    }
  }

  // Create a new progress element for a download
  function createProgressElement(update) {
    const container = document.createElement('div');
    container.className = 'download-progress';
    container.dataset.jobId = update.job_id;

    const header = document.createElement('div');
    header.className = 'download-header';

    const title = document.createElement('h3');
    // Get a cleaner filename or use the URL domain
    const displayName = getDisplayNameFromUrl(update.url);
    title.textContent = `${displayName}`;
    header.appendChild(title);

    const progressContainer = document.createElement('div');
    progressContainer.className = 'progress-container';

    const progressBar = document.createElement('div');
    progressBar.className = 'progress-bar infinite'; // Add infinite class by default
    progressContainer.appendChild(progressBar);

    const progressText = document.createElement('div');
    progressText.className = 'progress-text';
    progressText.textContent = 'Processing...'; // Default text

    const statusText = document.createElement('div');
    statusText.className = 'status-text';
    statusText.textContent = update.message || 'Starting download...';

    container.appendChild(header);
    container.appendChild(progressContainer);
    container.appendChild(progressText);
    container.appendChild(statusText);

    return {
      container,
      progressBar,
      progressText,
      statusText
    };
  }

  // Update an existing progress element with new data
  function updateProgressElement(element, update) {
    // Force the update to be applied immediately
    window.requestAnimationFrame(() => {
      // Only change infinite animation to completed/error when done
      if (update.status === 'completed') {
        element.progressBar.classList.remove('infinite');
        element.progressBar.style.width = '100%';
        element.progressText.textContent = 'Complete';
        // Add a subtle animation for completion
        element.container.style.transform = 'translateY(-2px)';
      } else if (update.status === 'error') {
        element.progressBar.classList.remove('infinite');
        element.progressBar.style.width = '100%';
        element.progressText.textContent = 'Failed';
      } else {
        // For all other statuses, show infinite animation
        element.progressBar.classList.add('infinite');
        element.progressText.textContent = 'Processing...';
      }

      // Update status text
      element.statusText.textContent = update.message;

      // Update classes based on status
      element.container.className = 'download-progress';
      element.container.classList.add(update.status);
    });
  }

  // This function was removed as it's not used anywhere in the codebase
  // and getDisplayNameFromUrl is used instead

  // Helper function to get a display name from URL
  function getDisplayNameFromUrl(url) {
    try {
      const urlObj = new URL(url);

      // Check for YouTube or other video platforms
      if (urlObj.hostname.includes('youtube.com') || urlObj.hostname.includes('youtu.be')) {
        return 'YouTube Video';
      } else if (urlObj.hostname.includes('vimeo.com')) {
        return 'Vimeo Video';
      } else if (urlObj.hostname.includes('soundcloud.com')) {
        return 'SoundCloud Audio';
      }

      // For other URLs, use the domain name
      return urlObj.hostname.replace('www.', '');
    } catch (e) {
      // If URL parsing fails, just return a generic name
      return 'Media Download';
    }
  }

  // Initialize the UI based on the default state
  updateOptionsVisibility();

  // Add event listener for the audio-only checkbox
  audioOnlyCheckbox.addEventListener("change", updateOptionsVisibility);

  // Function to update the visibility of options sections
  function updateOptionsVisibility() {
    if (audioOnlyCheckbox.checked) {
      videoOptionsSection.style.display = "none";
      audioOptionsSection.style.display = "block";
    } else {
      videoOptionsSection.style.display = "block";
      audioOptionsSection.style.display = "none";
    }
  }

  // Initialize WebSocket connection
  connectWebSocket();

  form.addEventListener("submit", async function (e) {
    e.preventDefault();

    // Get form values
    const mediaUrlsRaw = document.getElementById("mediaUrls").value;
    let outputDir = document.getElementById("outputDir").value;

    const urls = mediaUrlsRaw.split('\n')
      .map(url => url.trim())
      .filter(url => url !== "");

    // Get selected options (these apply to all URLs)
    const selectedOptions = [];
    document
      .querySelectorAll('input[name="options"]:checked')
      .forEach((option) => {
        selectedOptions.push(option.value);
      });

    if (audioOnlyCheckbox.checked) {
      const audioFormat = document.getElementById('audio-format').value;
      const audioQuality = document.getElementById('audio-quality').value;
      selectedOptions.push(audioFormat);
      selectedOptions.push(audioQuality);
    } else {
      const videoQuality = document.getElementById('video-quality').value;
      selectedOptions.push(videoQuality);
    }

    // Validate form
    if (urls.length === 0) {
      showStatus("Please enter at least one valid media URL in the list.", "error");
      return;
    }

    if (!outputDir) {
      // Default outputDir applies to all URLs if not specified
      outputDir = "/tmp/pegasus_downloads";
      // No need for showStatus here yet, will be part of overall status
    }

    // Show loading state
    submitBtn.disabled = true;
    submitBtn.textContent = "Processing...";
    formStatusDiv.innerHTML = ''; // Clear previous form statuses ONLY
    showStatus(`Starting processing for ${urls.length} URL(s)...`, "info");
    if (!document.getElementById("outputDir").value && outputDir === "/tmp/pegasus_downloads") {
      appendStatus(`No output directory specified. Using default: ${outputDir}`, "info");
    }

    let allSuccessful = true;
    let successfulCount = 0;
    let failedCount = 0;

    for (let i = 0; i < urls.length; i++) {
      const currentUrl = urls[i];
      appendStatus(`(${i + 1}/${urls.length}) Processing: ${currentUrl}`, "info");

      try {
        // Prepare data for submission for the current URL
        const data = {
          mediaUrl: currentUrl,
          outputDir: outputDir,
          processingOptions: selectedOptions,
        };

        // Send data to API
        const response = await fetch("/api/submit", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(data),
        });

        if (response.ok) {
          const result = await response.json();
          appendStatus(`(${i + 1}/${urls.length}) Success for ${currentUrl} - Job ID: ${result.job_id}`, "success");
          successfulCount++;
        } else {
          allSuccessful = false;
          failedCount++;
          const error = await response.json();
          appendStatus(`(${i + 1}/${urls.length}) Error for ${currentUrl}: ${error.message || 'Unknown server error'}`, "error");
        }
      } catch (error) {
        allSuccessful = false;
        failedCount++;
        appendStatus(`(${i + 1}/${urls.length}) Network error for ${currentUrl}: ${error.message}`, "error");
      }
    }

    // Final status update
    let finalMessage = `All URL processing complete. Successful: ${successfulCount}, Failed: ${failedCount}.`;
    showStatus(finalMessage, allSuccessful && failedCount === 0 ? "success" : "info"); // info if there were any failures, success if all green
    if (failedCount > 0) {
      appendStatus("Check individual statuses above for details on failures.", "info")
    }

    // Reset button state
    submitBtn.disabled = false;
    submitBtn.textContent = "Submit";
  });

  function showStatus(message, type) {
    // Only clear the form status section, not the entire statusDiv
    formStatusDiv.innerHTML = '';
    const p = document.createElement('p');
    p.textContent = message;
    p.className = type || '';
    formStatusDiv.appendChild(p);

    statusDiv.className = "status"; // Base class
    if (type) {
      // For overall status, apply class to statusDiv itself if it influences background etc.
      // However, individual lines are now <p> tags, so this might need adjustment
      // For now, let's assume .status .success, .status .error can target p tags
      // Or, we can just style p.success, p.error directly.
    }
    statusDiv.style.display = "block";
  }

  // Helper function to append status messages as new paragraphs
  function appendStatus(message, type) {
    const p = document.createElement('p');
    p.textContent = message;
    if (type) {
      p.classList.add(type); // e.g., 'success', 'error', 'info'
    }
    formStatusDiv.appendChild(p);
    statusDiv.style.display = "block"; // Ensure div is visible
  }
});
