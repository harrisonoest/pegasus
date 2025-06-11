// src/task_manager.rs
// Manages active download/processing tasks and their cancellation.

use crate::error::{PegasusError, Result};
use dashmap::DashMap;
use std::process::Child;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tracing::{debug, error, info, warn};

/// Represents an active task (yt-dlp or ffmpeg process).
#[derive(Debug)]
pub struct ActiveTask {
    // We need a Mutex here because `Child::kill` and `Child::try_wait` take `&mut self`.
    // The `Arc` allows sharing the Mutex guard across async boundaries if needed,
    // though direct manipulation of `Child` should be localized.
    pub child: Arc<Mutex<Child>>,
    pub cancel_notifier: Arc<Notify>,
}

impl ActiveTask {
    pub fn new(child_process: Child) -> Self {
        Self {
            child: Arc::new(Mutex::new(child_process)),
            cancel_notifier: Arc::new(Notify::new()),
        }
    }

    /// Signals the task to cancel.
    pub fn request_cancellation(&self) {
        self.cancel_notifier.notify_one();
    }

    /// Attempts to kill the child process.
    pub async fn kill_child_process(&self) -> Result<()> {
        let mut child_guard = self.child.lock().await;
        match child_guard.kill() {
            Ok(_) => {
                info!(
                    "Successfully sent kill signal to child process {}",
                    child_guard.id()
                );
                // Optionally wait for the process to ensure it's terminated
                // match child_guard.wait() {
                //     Ok(status) => info!("Child process {} exited with status: {}", child_guard.id(), status),
                //     Err(e) => warn!("Error waiting for child process {}: {}", child_guard.id(), e),
                // };
                Ok(())
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::InvalidInput {
                    // This can happen if the process has already exited.
                    warn!(
                        "Failed to kill child process {}: process already exited? ({})",
                        child_guard.id(),
                        e
                    );
                    Ok(())
                } else {
                    error!("Failed to kill child process {}: {}", child_guard.id(), e);
                    Err(PegasusError::ExternalCommandError(format!(
                        "Failed to kill child process {}: {}",
                        child_guard.id(),
                        e
                    )))
                }
            }
        }
    }
}

/// Manages a collection of active tasks.
#[derive(Debug, Clone, Default)]
pub struct TaskManager {
    tasks: Arc<DashMap<String, Arc<ActiveTask>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a new task to the manager.
    ///
    /// # Arguments
    /// * `job_id` - The unique ID for the job/task.
    /// * `child_process` - The `std::process::Child` to manage.
    ///
    /// # Returns
    /// An `Arc<Notify>` that can be used to listen for cancellation requests for this task.
    pub fn add_task(&self, job_id: &str, child_process: Child) -> Arc<Notify> {
        info!(job_id = %job_id, pid = %child_process.id(), "Adding new task to TaskManager");
        let task = Arc::new(ActiveTask::new(child_process));
        let notifier = task.cancel_notifier.clone();
        self.tasks.insert(job_id.to_string(), task);
        notifier
    }

    /// Retrieves the cancellation notifier for a specific task.
    pub fn get_task_cancellation_notifier(&self, job_id: &str) -> Option<Arc<Notify>> {
        self.tasks
            .get(job_id)
            .map(|task_ref| task_ref.value().cancel_notifier.clone())
    }

    /// Signals a task to cancel and attempts to kill its child process.
    pub async fn cancel_task(&self, job_id: &str) -> Result<()> {
        info!(job_id = %job_id, "Attempting to cancel task");
        if let Some(task_ref) = self.tasks.get(job_id) {
            let task = task_ref.value();
            task.request_cancellation(); // Signal any long-running operations within our code
            debug!(job_id = %job_id, "Cancellation requested, attempting to kill child process.");
            // Attempt to kill the external process
            task.kill_child_process().await?;
            // The task is not removed here; removal should happen when the worker confirms cancellation.
            Ok(())
        } else {
            warn!(job_id = %job_id, "Task not found for cancellation");
            Err(PegasusError::JobNotFound(job_id.to_string()))
        }
    }

    /// Removes a task from the manager. This should be called when a task is confirmed completed, failed, or cancelled.
    pub fn remove_task(&self, job_id: &str) -> Option<Arc<ActiveTask>> {
        info!(job_id = %job_id, "Removing task from TaskManager");
        self.tasks.remove(job_id).map(|(_key, task)| task)
    }

    /// Checks if a task is currently managed.
    #[allow(dead_code)] // Might be useful later
    pub fn has_task(&self, job_id: &str) -> bool {
        self.tasks.contains_key(job_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn test_add_and_remove_task() {
        let manager = TaskManager::new();
        let child = Command::new("sleep").arg("10").spawn().unwrap();
        let job_id = "test_job_1";

        let notifier = manager.add_task(job_id, child);
        assert!(manager.has_task(job_id));
        assert_eq!(Arc::strong_count(&notifier), 2); // One in manager, one here

        let removed_task = manager.remove_task(job_id).unwrap();
        assert!(!manager.has_task(job_id));
        assert_eq!(Arc::strong_count(&removed_task.cancel_notifier), 1); // Only in removed_task
        // Ensure child process is killed on drop or explicitly
        removed_task.kill_child_process().await.ok();
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let manager = TaskManager::new();
        let child_process = Command::new("sleep").arg("5").spawn().unwrap();
        let child_pid = child_process.id();
        let job_id = "test_job_cancel";

        let notifier = manager.add_task(job_id, child_process);

        let cancellation_check = tokio::spawn(async move {
            notifier.notified().await;
            // This confirms that our internal cancellation signal works.
            // The actual process kill is tested by checking if it exits early.
        });

        // Give a moment for the process to start
        sleep(Duration::from_millis(100)).await;

        let cancel_result = manager.cancel_task(job_id).await;
        assert!(cancel_result.is_ok());

        // Wait for the cancellation signal to be processed by the spawned task
        cancellation_check.await.unwrap();

        // Check if the process was actually killed (or exited early due to kill)
        // This is a bit platform-dependent and timing-sensitive for robust testing here.
        // A simple check is to see if we can still find the process by PID (might not be reliable)
        // For now, we trust that `child.kill()` did its job or the process exited.
        // A more robust test would involve the child process writing a file on exit and checking that.
        info!("Cancelled child process with PID: {}", child_pid);

        // Clean up
        manager.remove_task(job_id);
    }

    #[tokio::test]
    async fn test_cancel_non_existent_task() {
        let manager = TaskManager::new();
        let result = manager.cancel_task("non_existent_job").await;
        assert!(matches!(result, Err(PegasusError::JobNotFound(_))));
    }
}
