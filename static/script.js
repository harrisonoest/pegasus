document.addEventListener('DOMContentLoaded', () => {
  const form = document.getElementById('download-form');
  const urlInput = document.getElementById('url');
  const responseDiv = document.getElementById('response');
  const queueContainer = document.getElementById('queue-container');
  const jobTemplate = document.getElementById('job-template');

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
    jobElement.querySelector('.job-status').textContent = job.status;
    jobElement.querySelector('.job-message').textContent = job.message || '';

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
    let jobElement = queueContainer.querySelector(`.job-item[data-job-id='${update.job_id}']`);

    const jobData = {
      id: update.job_id,
      url: update.url,
      status: update.status,
      progress: update.progress,
      message: update.message,
    };

    const newElement = renderJob(jobData);

    if (jobElement) {
      // Update existing element
      jobElement.replaceWith(newElement);
    } else {
      // Add new element
      queueContainer.prepend(newElement);
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
      try {
        const update = JSON.parse(event.data);
        if (update.job_id) {
          updateJobInQueue(update);
        }
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error, event.data);
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
   * Handles form submission to submit a new URL.
   */
  form.addEventListener('submit', async (e) => {
    e.preventDefault();
    const url = urlInput.value;
    responseDiv.textContent = 'Submitting...';
    responseDiv.className = 'message';

    try {
      const response = await fetch('/api/submit', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          mediaUrl: url,
          // Placeholder for future advanced options
          outputDir: null,
          processingOptions: [],
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
      } else {
        responseDiv.textContent = `Error: ${result.error || 'Unknown error'}`;
        responseDiv.className = 'message error';
      }
    } catch (error) {
      responseDiv.textContent = `Network Error: ${error.message}`;
      responseDiv.className = 'message error';
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
