#![doc = include_str!("../README.md")]
#![no_std]
extern crate alloc;
extern crate std;

mod apple;
mod local_value;
use executor_core::Executor;
pub use local_value::{LocalValue, OnceValue};
#[cfg(target_vendor = "apple")]
mod main_value;
#[cfg(target_vendor = "apple")]
pub use main_value::MainValue;
#[cfg(target_vendor = "apple")]
pub mod timer;
use core::mem::ManuallyDrop;
use core::time::Duration;
pub use futures_lite::*;

#[cfg(target_vendor = "apple")]
type DefaultExecutor = apple::ApplePlatformExecutor;

trait PlatformExecutor {
    fn exec_main(f: impl FnOnce() + Send + 'static);
    fn exec(f: impl FnOnce() + Send + 'static, priority: Priority);

    fn exec_after(delay: Duration, f: impl FnOnce() + Send + 'static);
}

#[cfg(target_vendor = "apple")]
impl Executor for DefaultExecutor {
    fn spawn<T: Send + 'static>(
        &self,
        fut: impl Future<Output = T> + Send + 'static,
    ) -> impl executor_core::Task<Output = T> {
        Task::new(fut)
    }
}

use async_task::Runnable;

/// Task execution priority levels for controlling scheduler behavior.
///
/// These priority levels map to platform-native scheduling priorities,
/// allowing fine-grained control over task execution order and resource allocation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// Standard priority level for most application tasks.
    ///
    /// This is the default priority that provides balanced execution
    /// suitable for general-purpose async operations.
    #[default]
    Default,
    /// Lower priority for background tasks and non-critical operations.
    ///
    /// Background tasks yield CPU time to higher-priority tasks and are
    /// ideal for operations like cleanup, logging, or data processing
    /// that don't require immediate completion.
    Background,
}

/// A handle to a spawned asynchronous task that can be shared between threads.
///
/// `Task<T>` represents a future that will complete with the output of the spawned task.
/// Tasks are automatically scheduled for execution using platform-native primitives
/// and can be awaited to retrieve their results.
///
/// # Thread Safety
///
/// Tasks are `Send` and `Sync`, allowing them to be moved between threads and
/// shared safely. The underlying execution is handled by the platform's scheduler.
///
/// # Examples
///
/// ```rust
/// use native_executor::{Task, Priority};
///
/// // Spawn a task with default priority
/// let task = Task::new(async { 42 });
///
/// // Spawn a background task
/// let bg_task = Task::with_priority(async { "background" }, Priority::Background);
/// ```
#[derive(Debug)]
pub struct Task<T: 'static + Send> {
    inner: ManuallyDrop<async_task::Task<T>>,
}

impl<T: Send> executor_core::Task for Task<T> {
    async fn result(self) -> Result<Self::Output, executor_core::Error> {
        Ok(self.await)
    }

    fn cancel(self) {
        drop(ManuallyDrop::into_inner(self.inner));
    }
}

impl<T: 'static + Send> Task<T> {
    /// Creates a new task with default priority.
    ///
    /// The task is immediately scheduled for execution using the platform's
    /// default priority level, which provides balanced performance for most use cases.
    ///
    /// # Arguments
    /// * `future` - The future to execute asynchronously
    ///
    /// # Returns
    /// A `Task` handle that can be awaited to retrieve the result
    ///
    /// # Examples
    /// ```rust
    /// use native_executor::Task;
    ///
    /// let task = Task::new(async { 42 + 58 });
    /// // let result = task.await; // Returns 100
    /// ```
    pub fn new<Fut>(future: Fut) -> Self
    where
        Fut: Future<Output = T> + Send + 'static,
    {
        Self::with_priority(future, Priority::default())
    }

    /// Creates a new task with the specified execution priority.
    ///
    /// This allows fine-grained control over task scheduling, enabling
    /// background tasks to yield to higher-priority operations.
    ///
    /// # Arguments
    /// * `future` - The future to execute asynchronously
    /// * `priority` - The scheduling priority for this task
    ///
    /// # Returns
    /// A `Task` handle that can be awaited to retrieve the result
    ///
    /// # Examples
    /// ```rust
    /// use native_executor::{Task, Priority};
    ///
    /// // High-priority task for time-sensitive operations
    /// let urgent = Task::with_priority(async {
    ///     process_user_input().await
    /// }, Priority::Default);
    ///
    /// // Background task that won't interfere with UI responsiveness
    /// let cleanup = Task::with_priority(async {
    ///     clean_temporary_files().await
    /// }, Priority::Background);
    /// ```
    pub fn with_priority<Fut>(future: Fut, priority: Priority) -> Self
    where
        Fut: Future<Output = T> + Send + 'static,
    {
        let (runnable, task) = async_task::spawn(future, move |runnable: Runnable| {
            exec(
                move || {
                    runnable.run();
                },
                priority,
            );
        });

        exec(
            move || {
                runnable.run();
            },
            priority,
        );
        Self {
            inner: ManuallyDrop::new(task),
        }
    }

    /// Schedules a task to run exclusively on the main thread.
    ///
    /// This is essential for operations that must execute on the main thread,
    /// such as UI updates, main-thread-only API calls, or accessing thread-local
    /// resources that are bound to the main thread.
    ///
    /// # Arguments
    /// * `future` - The future to execute on the main thread
    ///
    /// # Returns
    /// A `Task` handle that can be awaited to retrieve the result
    ///
    /// # Platform Behavior
    /// - **Apple platforms**: Uses GCD's main queue for execution
    /// - **Other platforms**: Will be implemented with platform-specific main thread scheduling
    ///
    /// # Examples
    /// ```rust
    /// use native_executor::Task;
    ///
    /// // UI update that must happen on the main thread
    /// let ui_task = Task::on_main(async {
    ///     update_window_title("Processing...").await;
    ///     "UI updated"
    /// });
    /// ```
    pub fn on_main<Fut>(future: Fut) -> Self
    where
        Fut: Future<Output = T> + Send + 'static,
    {
        let (runnable, task) = async_task::spawn(future, move |runnable: Runnable| {
            exec_main(move || {
                runnable.run();
            });
        });

        exec_main(move || {
            runnable.run();
        });
        Self {
            inner: ManuallyDrop::new(task),
        }
    }
}

impl<T: Send> Future for Task<T> {
    type Output = T;
    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        self.inner.poll(cx)
    }
}

/// A handle to a thread-local asynchronous task.
///
/// `LocalTask<T>` is designed for futures that are not `Send` and must execute
/// on the same thread where they were created. This is useful for working with
/// thread-local storage, non-thread-safe types, or platform APIs that require
/// specific thread contexts.
///
/// Unlike `Task<T>`, `LocalTask<T>` does not implement `Send` or `Sync`,
/// ensuring compile-time safety for thread-local operations.
///
/// # Examples
/// ```rust
/// use native_executor::LocalTask;
/// use std::rc::Rc;
///
/// // Rc is not Send, so we need LocalTask
/// let local_data = Rc::new(42);
/// let task = LocalTask::on_main(async move {
///     *local_data + 58
/// });
/// ```
#[derive(Debug)]
pub struct LocalTask<T> {
    inner: ManuallyDrop<async_task::Task<T>>,
}

impl<T: 'static> LocalTask<T> {
    /// Schedules a thread-local task to run on the main thread.
    ///
    /// This method is specifically designed for futures that are not `Send`,
    /// allowing them to be executed safely on the main thread while maintaining
    /// thread-local guarantees.
    ///
    /// # Arguments
    /// * `future` - The non-Send future to execute on the main thread
    ///
    /// # Returns
    /// A `LocalTask` handle that can be awaited to retrieve the result
    ///
    /// # Safety
    /// The future will only be polled on the main thread, ensuring that
    /// thread-local data and non-Send types remain safe to use.
    ///
    /// # Examples
    /// ```rust
    /// use native_executor::LocalTask;
    /// use std::rc::Rc;
    ///
    /// let shared_data = Rc::new("thread-local data");
    /// let task = LocalTask::on_main(async move {
    ///     println!("Data: {}", shared_data);
    ///     shared_data.len()
    /// });
    /// ```
    pub fn on_main<Fut>(future: Fut) -> Self
    where
        Fut: Future<Output = T> + 'static,
    {
        let (runnable, task) = async_task::spawn_local(future, move |runnable: Runnable| {
            exec_main(move || {
                runnable.run();
            });
        });

        exec_main(move || {
            runnable.run();
        });
        Self {
            inner: ManuallyDrop::new(task),
        }
    }

    /// Cancels the local task and waits for it to stop.
    ///
    /// # Returns
    /// A future that resolves when the task has been cancelled
    pub async fn cancel(self) {
        ManuallyDrop::into_inner(self.inner).cancel().await;
    }
}

impl<T> Future for LocalTask<T> {
    type Output = T;
    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        self.inner.poll(cx)
    }
}

/// Convenience function to spawn a new task with default priority.
///
/// This is the most common way to spawn tasks, providing a simple interface
/// for creating and scheduling asynchronous work with optimal platform integration.
///
/// # Arguments
/// * `fut` - The future to execute asynchronously
///
/// # Returns
/// A `Task` handle that can be awaited to retrieve the result
///
/// # Examples
/// ```rust
/// use native_executor::task;
///
/// // Spawn a simple task
/// let handle = task(async {
///     expensive_computation().await
/// });
///
/// // Wait for completion
/// let result = handle.await;
/// ```
pub fn task<Fut>(fut: Fut) -> Task<Fut::Output>
where
    Fut: Future + Send + 'static,
    Fut::Output: Send,
{
    Task::new(fut)
}

/// Schedules a function to be executed on the main thread.
///
/// # Parameters
/// * `f` - The function to execute on the main thread
#[cfg(target_vendor = "apple")]
fn exec_main(f: impl FnOnce() + Send + 'static) {
    DefaultExecutor::exec_main(f);
}

/// Schedules a function to be executed with the specified priority.
///
/// # Parameters
/// * `f` - The function to execute
/// * `priority` - The execution priority for the function
#[cfg(target_vendor = "apple")]
fn exec(f: impl FnOnce() + Send + 'static, priority: Priority) {
    DefaultExecutor::exec(f, priority);
}

/// Schedules a function to be executed after a specified delay.
///
/// # Parameters
/// * `delay` - The duration to wait before executing the function
/// * `f` - The function to execute after the delay
#[cfg(target_vendor = "apple")]
fn exec_after(delay: Duration, f: impl FnOnce() + Send + 'static) {
    DefaultExecutor::exec_after(delay, f);
}
