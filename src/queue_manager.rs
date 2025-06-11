// src/queue_manager.rs
// Manages the download queue and worker tasks.

use crate::api::handlers::ProgressUpdate;
use crate::download::{DownloadJob, DownloadJobStatus, process_download_job};
use crate::error::{PegasusError, Result};
use crate::task_manager::TaskManager;
use dashmap::DashMap;
use std::collections::VecDeque;

use std::sync::Arc;
use tokio::sync::{Mutex, Notify, broadcast};
use tracing::{debug, error, info, warn};

/// Central manager for the download queue.
#[derive(Debug)]
pub struct DownloadQueue {
    /// All jobs, including queued, active, and completed.
    /// Key: job_id, Value: DownloadJob
    jobs: Arc<DashMap<String, DownloadJob>>,
    /// A queue of job_ids waiting to be processed.
    job_queue: Arc<Mutex<VecDeque<String>>>,
    /// Manages active child processes (yt-dlp, ffmpeg).
    task_manager: TaskManager,
    /// Notifies the worker task that a new job has been added.
    worker_notifier: Arc<Notify>,
    /// Broadcast sender for sending progress updates to the frontend.
    progress_sender: broadcast::Sender<ProgressUpdate>,
}

impl DownloadQueue {
    /// Creates a new DownloadQueue.
    pub fn new(
        task_manager: TaskManager,
        progress_sender: broadcast::Sender<ProgressUpdate>,
    ) -> Self {
        Self {
            jobs: Arc::new(DashMap::new()),
            job_queue: Arc::new(Mutex::new(VecDeque::new())),
            task_manager,
            worker_notifier: Arc::new(Notify::new()),
            progress_sender,
        }
    }

    /// Spawns the background worker task.
    pub fn spawn_worker(self: Arc<Self>) {
        info!("Spawning download queue worker.");
        tokio::spawn(async move {
            loop {
                // Wait for a notification that a job is available
                self.worker_notifier.notified().await;
                info!("Worker notified. Checking for jobs.");

                loop {
                    let job_id = {
                        let mut queue_guard = self.job_queue.lock().await;
                        queue_guard.pop_front()
                    };

                    if let Some(job_id) = job_id {
                        info!(job_id = %job_id, "Worker picked up job.");
                        // Process the job
                        self.process_job_in_worker(&job_id).await;
                        // Remove the task from the task manager after it's done
                        self.task_manager.remove_task(&job_id);
                    } else {
                        // Queue is empty, break inner loop and wait for next notification
                        debug!("Queue is empty. Worker going to sleep.");
                        break;
                    }
                }
            }
        });
    }

    /// The core logic for the worker to process a single job.
    async fn process_job_in_worker(&self, job_id: &str) {
        // Update job status to Starting
        self.update_job_status(
            job_id,
            DownloadJobStatus::Starting,
            Some("Worker is preparing the job.".to_string()),
        );

        // Get a clone of the job to work with
        let job = match self.jobs.get(job_id) {
            Some(job_ref) => job_ref.value().clone(),
            None => {
                error!(job_id = %job_id, "Job disappeared from map before processing.");
                return;
            }
        };

        // TODO: This is where we need to adapt `process_download_job` to accept a cancellation token.
        // For now, we just run it.
        match process_download_job(&job, &self.progress_sender).await {
            Ok(_) => {
                self.update_job_status(
                    job_id,
                    DownloadJobStatus::Completed,
                    Some("Job finished successfully.".to_string()),
                );
            }
            Err(PegasusError::Cancelled) => {
                warn!(job_id = %job_id, "Job was cancelled during processing.");
                // Status is already set to Cancelled by the `cancel_job` function.
            }
            Err(e) => {
                error!(job_id = %job_id, error = %e, "Job failed during processing.");
                self.update_job_status(
                    job_id,
                    DownloadJobStatus::Error,
                    Some(format!("Processing failed: {}", e)),
                );
            }
        }
    }

    /// Adds a new download job to the queue.
    pub async fn add_job(&self, job: DownloadJob) -> Result<String> {
        let job_id = job.id.clone();
        info!(job_id = %job_id, url = %job.url, "Adding new job to queue");

        // Add to the main job map first
        self.jobs.insert(job_id.clone(), job);

        // Then add the ID to the processing queue
        {
            let mut queue_guard = self.job_queue.lock().await;
            queue_guard.push_back(job_id.clone());
        }

        // Notify the worker that there's a new job
        self.worker_notifier.notify_one();

        Ok(job_id)
    }

    /// Cancels a job, whether it's queued or currently running.
    pub async fn cancel_job(&self, job_id: &str) -> Result<()> {
        info!(job_id = %job_id, "Received cancellation request.");

        // First, check if the job is in the queue and remove it if so.
        let mut queue_guard = self.job_queue.lock().await;
        if let Some(index) = queue_guard.iter().position(|id| id == job_id) {
            queue_guard.remove(index);
            drop(queue_guard); // Release lock

            self.update_job_status(
                job_id,
                DownloadJobStatus::Cancelled,
                Some("Job cancelled while in queue.".to_string()),
            );
            info!(job_id = %job_id, "Cancelled job from queue.");
            return Ok(());
        }
        drop(queue_guard); // Release lock

        // If not in the queue, it might be running. Attempt to cancel via TaskManager.
        match self.task_manager.cancel_task(job_id).await {
            Ok(_) => {
                self.update_job_status(
                    job_id,
                    DownloadJobStatus::Cancelled,
                    Some("Job cancellation requested.".to_string()),
                );
                info!(job_id = %job_id, "Cancelled running job.");
                Ok(())
            }
            Err(PegasusError::JobNotFound(_)) => {
                // It might be already completed or failed.
                if let Some(job) = self.jobs.get_mut(job_id) {
                    let current_status = job.status.clone();
                    warn!(job_id = %job_id, status = %current_status, "Attempted to cancel a job that is not running or queued.");
                    // Don't change the status if it's already completed or errored.
                    if current_status != DownloadJobStatus::Completed
                        && current_status != DownloadJobStatus::Error
                    {
                        self.update_job_status(
                            job_id,
                            DownloadJobStatus::Cancelled,
                            Some("Job cancelled.".to_string()),
                        );
                    }
                    Ok(())
                } else {
                    error!(job_id = %job_id, "Cannot cancel job: ID does not exist.");
                    Err(PegasusError::JobNotFound(job_id.to_string()))
                }
            }
            Err(e) => {
                error!(job_id = %job_id, error = %e, "Failed to cancel job.");
                Err(e)
            }
        }
    }

    /// Returns a snapshot of all jobs.
    pub fn get_all_jobs(&self) -> Vec<DownloadJob> {
        self.jobs
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Helper to update a job's status and send a progress update.
    fn update_job_status(&self, job_id: &str, status: DownloadJobStatus, message: Option<String>) {
        if let Some(mut job) = self.jobs.get_mut(job_id) {
            job.status = status.clone();
            job.status_message = message.clone();

            // Send progress update via broadcast channel
            let update = ProgressUpdate {
                job_id: job_id.to_string(),
                url: job.url.clone(),
                status: status.to_string(),
                progress: job.progress, // Use the last known progress
                message: message.unwrap_or_else(|| status.to_string()),
            };

            if self.progress_sender.send(update).is_err() {
                debug!("No active subscribers to receive progress update.");
            }
        } else {
            warn!(job_id = %job_id, "Attempted to update status for a non-existent job.");
        }
    }
}
