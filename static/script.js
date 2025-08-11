document.addEventListener('DOMContentLoaded', () => {
  const form = document.getElementById('download-form');
  const urlInput = document.getElementById('url');
  const responseDiv = document.getElementById('response');
  const queueContainer = document.getElementById('queue-container');
  const jobTemplate = document.getElementById('job-template');
  const historyContainer = document.getElementById('history-container');
  const submitButton = form.querySelector('button[type="submit"]');
  
  // Options elements
  const videoModeRadio = document.getElementById('video-mode');
  const audioModeRadio = document.getElementById('audio-mode');
  const videoOptionsSection = document.getElementById('video-options');
  const audioOptionsSection = document.getElementById('audio-options');

  // Navigation handling
  const navLinks = document.querySelectorAll('.main-nav a');
  const sections = document.querySelectorAll('main section');

  function showSection(targetId) {
    sections.forEach(section => {
      section.style.display = section.id === targetId ? 'block' : 'none';
    });
    navLinks.forEach(link => {
      link.classList.toggle('active', link.getAttribute('href') === `#${targetId}`);
    });
  }

  navLinks.forEach(link => {
    link.addEventListener('click', (e) => {
      e.preventDefault();
      const targetId = e.target.getAttribute('href').substring(1);
      showSection(targetId);
    });
  });
  // Show initial section
  showSection('submit-section');

  // Handle mode switching
  function toggleModeOptions() {
    const isVideoMode = videoModeRadio.checked;
    videoOptionsSection.style.display = isVideoMode ? 'block' : 'none';
    audioOptionsSection.style.display = isVideoMode ? 'none' : 'block';
  }

  videoModeRadio.addEventListener('change', toggleModeOptions);
  audioModeRadio.addEventListener('change', toggleModeOptions);
  
  // Initialize mode display
  toggleModeOptions();


  /**
   * Renders a single job item using the template.
   * @param {object} job - The job object.
   * @returns {HTMLElement} - The rendered job element.
   */
  function renderJob(job) {
    const jobClone = jobTemplate.content.cloneNode(true);
    const jobElement = jobClone.querySelector('.job-item');

    jobElement.dataset.jobId = job.id;
    jobElement.querySelector('.job-url').textContent = job.url;
    const statusDiv = jobElement.querySelector('.job-status');
    statusDiv.textContent = job.status;
    const statusLower = job.status.toLowerCase();
    statusDiv.className = 'job-status';
    if (statusLower === 'downloading') statusDiv.classList.add('status-downloading');
    else if (statusLower === 'processing') statusDiv.classList.add('status-processing');
    else if (statusLower === 'completed') statusDiv.classList.add('status-completed');
    else if (statusLower === 'error' || statusLower === 'cancelled') statusDiv.classList.add('status-error');

    jobElement.querySelector('.job-message').textContent = job.message || '';

    // Speed / ETA
    const speedEl = jobElement.querySelector('.job-speed');
    const etaEl = jobElement.querySelector('.job-eta');
    speedEl.textContent = job.speed ? `| ${job.speed}` : '';
    etaEl.textContent = job.eta ? `(ETA: ${job.eta})` : '';

    const progressElement = jobElement.querySelector('.job-progress');
    progressElement.value = job.progress || 0;
    // Hide progress bar if progress is 0
    progressElement.style.display = (job.progress && job.progress > 0) ? 'block' : 'none';


    // Show/hide cancel button based on status
    const cancelButton = jobElement.querySelector('.cancel-btn');
    const isTerminal = ['completed', 'error', 'cancelled'].includes(job.status.toLowerCase());
    cancelButton.style.display = isTerminal ? 'none' : 'block';

    return jobElement;
  }

  /**
   * Updates an existing job item in the queue or adds a new one.
   * @param {object} update - The progress update object from the WebSocket.
   */
  function updateJobInQueue(update) {
    console.log('hhh WS update', update);
    let jobElement = queueContainer.querySelector(`.job-item[data-job-id='${update.job_id}']`);

    const jobData = {
      id: update.job_id,
      url: update.url,
      status: update.status,
      progress: update.progress,
      message: update.message,
      speed: update.speed,
      eta: update.eta,
    };

    if (jobElement) {
      // Update fields in place for smoother UI
      jobElement.querySelector('.job-status').textContent = jobData.status;
      jobElement.querySelector('.job-message').textContent = jobData.message || '';

      const speedEl = jobElement.querySelector('.job-speed');
      const etaEl = jobElement.querySelector('.job-eta');
      speedEl.textContent = jobData.speed ? `| ${jobData.speed}` : '';
      etaEl.textContent = jobData.eta ? `(ETA: ${jobData.eta})` : '';

      const progressEl = jobElement.querySelector('.job-progress');
      progressEl.value = jobData.progress || 0;
      progressEl.style.display = (jobData.progress && jobData.progress > 0) ? 'block' : 'none';

      // Update status colour class
      const statusDiv = jobElement.querySelector('.job-status');
      statusDiv.className = 'job-status';
      const s = jobData.status.toLowerCase();
      if (s === 'downloading') statusDiv.classList.add('status-downloading');
      else if (s === 'processing') statusDiv.classList.add('status-processing');
      else if (s === 'completed') statusDiv.classList.add('status-completed');
      else if (s === 'error' || s === 'cancelled') statusDiv.classList.add('status-error');
    } else {
      const newElement = renderJob(jobData);
      queueContainer.prepend(newElement);
    }

    // If terminal, schedule move to history
    const terminal = ['completed', 'error', 'cancelled'].includes(update.status.toLowerCase());
    if (terminal) {
      setTimeout(() => {
        const el = queueContainer.querySelector(`.job-item[data-job-id='${update.job_id}']`);
        if (el) {
          historyContainer.prepend(el);
          // keep history size reasonable
          if (historyContainer.children.length > 100) {
            historyContainer.lastElementChild.remove();
          }
        }
      }, 60000);
    }
  }

  /**
   * Fetches the initial queue state from the server.
   */
  async function fetchInitialQueue() {
    try {
      const response = await fetch('/api/queue');
      if (response.ok) {
        const jobs = await response.json();
        queueContainer.innerHTML = ''; // Clear existing jobs
        jobs.forEach(job => {
          const jobElement = renderJob(job);
          queueContainer.appendChild(jobElement);
        });
      } else {
        console.error('Failed to fetch initial queue:', response.statusText);
      }
    } catch (error) {
      console.error('Failed to fetch initial queue:', error);
    }
  }

  /**
   * Establishes and manages the WebSocket connection.
   */
  function connectWebSocket() {
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${wsProtocol}//${window.location.host}/ws`);

    ws.onopen = () => {
      console.log('WebSocket connection established.');
      fetchInitialQueue(); // Fetch queue state on connect
    };

    ws.onmessage = (event) => {
      if (event.data.trim().startsWith('{')) {
        try {
          const update = JSON.parse(event.data);
          if (update.job_id) {
            updateJobInQueue(update);
          }
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error, event.data);
        }
      } else {
        console.log('hhh WS text', event.data);
      }
    };

    ws.onclose = () => {
      console.log('WebSocket connection closed. Reconnecting in 2 seconds...');
      setTimeout(connectWebSocket, 2000);
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };
  }

  /**
   * Collects form options and returns them as an object.
   */
  function collectFormOptions() {
    const isVideoMode = videoModeRadio.checked;
    const options = {
      mode: isVideoMode ? 'video' : 'audio',
    };

    if (isVideoMode) {
      // Video options
      options.videoQuality = document.getElementById('video-quality').value;
      options.embedMetadata = document.getElementById('video-metadata').checked;
      options.embedSubtitles = document.getElementById('video-subtitles').checked;
      options.subtitleLanguage = document.getElementById('subtitle-language').value;
      options.embedChapters = document.getElementById('video-chapters').checked;
    } else {
      // Audio options
      options.audioFormat = document.getElementById('audio-format').value;
      options.audioQuality = document.getElementById('audio-quality').value;
      options.embedMetadata = document.getElementById('audio-metadata').checked;
      options.addThumbnail = document.getElementById('audio-thumbnail').checked;
      options.normalizeAudio = document.getElementById('audio-normalize').checked;
    }

    return options;
  }

  /**
   * Handles form submission to submit a new URL.
   */
  form.addEventListener('submit', async (e) => {
    e.preventDefault();
    const url = urlInput.value;
    const options = collectFormOptions();
    
    responseDiv.textContent = 'Submitting...';
    responseDiv.className = 'message';
    submitButton.disabled = true;

    try {
      const response = await fetch('/api/submit', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          mediaUrl: url,
          outputDir: null,
          downloadOptions: options,
        }),
      });

      // Safely parse response
      const contentType = response.headers.get('content-type') || '';
      let result;
      if (contentType.includes('application/json')) {
        result = await response.json();
      } else {
        // Fallback to text when JSON is not returned (e.g., error string)
        const text = await response.text();
        result = { error: text };
      }

      if (response.ok) {
        responseDiv.textContent = `Job submitted successfully! Job ID: ${result.job_id}`;
        responseDiv.className = 'message success';
        urlInput.value = ''; // Clear input field
        submitButton.disabled = false;
      } else {
        responseDiv.textContent = `Error: ${result.error || 'Unknown error'}`;
        responseDiv.className = 'message error';
        submitButton.disabled = false;
      }
    } catch (error) {
      responseDiv.textContent = `Network Error: ${error.message}`;
      responseDiv.className = 'message error';
      submitButton.disabled = false;
    }
  });

  /**
   * Handles click events on cancel buttons.
   */
  queueContainer.addEventListener('click', async (e) => {
    if (e.target.classList.contains('cancel-btn')) {
      const jobItem = e.target.closest('.job-item');
      const jobId = jobItem.dataset.jobId;

      if (!confirm(`Are you sure you want to cancel job for ${jobItem.querySelector('.job-url').textContent}?`)) {
        return;
      }

      try {
        const response = await fetch(`/api/downloads/${jobId}/cancel`, {
          method: 'POST',
        });

        if (response.ok) {
          // Optimistic UI update
          jobItem.querySelector('.job-status').textContent = 'Cancelling…';
          jobItem.classList.add('cancelling');
          e.target.style.display = 'none';
        }
      } catch (error) {
        alert(`Error cancelling job: ${error.message}`);
      }
    }
  });

  // Initial setup
  fetchInitialQueue(); // populate immediately
  connectWebSocket();
});
