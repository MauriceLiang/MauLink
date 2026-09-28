use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::{AppError, AppInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownReport {
    pub joined_tasks: usize,
    pub aborted_tasks: usize,
}

#[derive(Default)]
struct TaskState {
    accepting: bool,
    handles: Vec<JoinHandle<()>>,
}

struct AppCoreInner {
    info: AppInfo,
    cancellation: CancellationToken,
    tasks: Mutex<TaskState>,
}

/// Root owner for process-wide core tasks.
#[derive(Clone)]
pub struct AppCore {
    inner: Arc<AppCoreInner>,
}

impl Default for AppCore {
    fn default() -> Self {
        Self::new()
    }
}

impl AppCore {
    pub fn new() -> Self {
        Self::with_info(AppInfo::current())
    }

    pub fn with_info(info: AppInfo) -> Self {
        Self {
            inner: Arc::new(AppCoreInner {
                info,
                cancellation: CancellationToken::new(),
                tasks: Mutex::new(TaskState {
                    accepting: true,
                    handles: Vec::new(),
                }),
            }),
        }
    }

    pub fn info(&self) -> AppInfo {
        self.inner.info.clone()
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.inner.cancellation.clone()
    }

    pub fn spawn<F>(&self, future: F) -> Result<(), Box<AppError>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut tasks = self
            .inner
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !tasks.accepting {
            return Err(Box::new(AppError::shutting_down()));
        }

        tasks.handles.push(tokio::spawn(future));
        Ok(())
    }

    pub async fn shutdown(&self, timeout: Duration) -> ShutdownReport {
        let handles = {
            let mut tasks = self
                .inner
                .tasks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            tasks.accepting = false;
            std::mem::take(&mut tasks.handles)
        };

        self.inner.cancellation.cancel();
        let deadline = tokio::time::Instant::now() + timeout;
        let total = handles.len();
        let mut joined = 0;
        let mut remaining = handles.into_iter();

        while let Some(mut handle) = remaining.next() {
            let wait = deadline.saturating_duration_since(tokio::time::Instant::now());
            if wait.is_zero() || tokio::time::timeout(wait, &mut handle).await.is_err() {
                handle.abort();
                for handle in remaining {
                    handle.abort();
                }
                break;
            }
            joined += 1;
        }

        ShutdownReport {
            joined_tasks: joined,
            aborted_tasks: total - joined,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_cancels_and_joins_registered_tasks() {
        let core = AppCore::new();
        let token = core.cancellation_token();
        core.spawn(async move { token.cancelled().await })
            .expect("register task");

        let report = core.shutdown(Duration::from_secs(1)).await;
        assert_eq!(report.joined_tasks, 1);
        assert_eq!(report.aborted_tasks, 0);
        assert!(core.spawn(async {}).is_err());
    }

    #[tokio::test]
    async fn shutdown_aborts_tasks_that_ignore_cancellation() {
        let core = AppCore::new();
        core.spawn(std::future::pending()).expect("register task");

        let report = core.shutdown(Duration::from_millis(1)).await;
        assert_eq!(report.joined_tasks, 0);
        assert_eq!(report.aborted_tasks, 1);
    }
}
