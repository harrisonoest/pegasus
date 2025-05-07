document.addEventListener("DOMContentLoaded", function () {
  const form = document.getElementById("uploadForm");
  const submitBtn = document.getElementById("submitBtn");
  const statusDiv = document.getElementById("status");
  const audioOnlyCheckbox = document.getElementById("audio-only");
  const videoOptionsSection = document.getElementById("video-options");
  const audioOptionsSection = document.getElementById("audio-options");

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
    statusDiv.innerHTML = ''; // Clear previous statuses
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
          appendStatus(`(${i + 1}/${urls.length}) Success for ${currentUrl} - Job ID: ${result.jobId}`, "success");
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
    // If this is the first message (clearing old ones) or an overall summary
    statusDiv.innerHTML = ''; // Clear previous content for a new summary message
    const p = document.createElement('p');
    p.textContent = message;
    p.className = type || '';
    statusDiv.appendChild(p);

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
    statusDiv.appendChild(p);
    statusDiv.style.display = "block"; // Ensure div is visible
  }
});
