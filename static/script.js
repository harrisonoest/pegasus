document.addEventListener("DOMContentLoaded", function () {
  const form = document.getElementById("uploadForm");
  const submitBtn = document.getElementById("submitBtn");
  const statusDiv = document.getElementById("status");

  form.addEventListener("submit", async function (e) {
    e.preventDefault();

    // Get form values
    const mediaUrl = document.getElementById("mediaUrl").value;
    let outputDir = document.getElementById("outputDir").value;

    // Get selected options
    const selectedOptions = [];
    document
      .querySelectorAll('input[name="options"]:checked')
      .forEach((option) => {
        selectedOptions.push(option.value);
      });

    // Validate form
    if (!mediaUrl) {
      showStatus("Please enter a valid media URL", "error");
      return;
    }

    if (!outputDir) {
      showStatus("Using default output directory", "info");
      outputDir = "/tmp/pegasus_downloads";
    }

    // Prepare data for submission
    const data = {
      mediaUrl: mediaUrl,
      outputDir: outputDir,
      processingOptions: selectedOptions,
    };

    // Show loading state
    submitBtn.disabled = true;
    submitBtn.textContent = "Processing...";
    showStatus("Submitting request...", "info");

    try {
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
        showStatus(`Success! Job ID: ${result.jobId}`, "success");
      } else {
        const error = await response.json();
        showStatus(`Error: ${error.message}`, "error");
      }
    } catch (error) {
      showStatus(`Network error: ${error.message}`, "error");
    } finally {
      // Reset button state
      submitBtn.disabled = false;
      submitBtn.textContent = "Submit";
    }
  });

  function showStatus(message, type) {
    statusDiv.textContent = message;
    statusDiv.className = "status";

    if (type) {
      statusDiv.classList.add(type);
    }

    statusDiv.style.display = "block";
  }
});
