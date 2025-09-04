#![doc = include_str!("../README.md")]
#![no_std]
#![warn(missing_docs, missing_debug_implementations)]
extern crate alloc;
extern crate std;

mod apple;
#[cfg(target_vendor = "apple")]
use async_task::Task;
use executor_core::{Executor, LocalExecutor};
#[cfg(target_vendor = "apple")]
mod main_value;
#[cfg(target_vendor = "apple")]
pub use main_value::MainValue;
pub mod mailbox;
#[cfg(target_vendor = "apple")]
pub mod timer;
use core::mem::ManuallyDrop;
use core::time::Duration;
pub use futures_lite::*;

#[cfg(target_vendor = "apple")]
type DefaultExecutor = apple::ApplePlatformExecutor;

// Only provide stub implementations when building docs (e.g., on docs.rs)
#[cfg(all(not(target_vendor = "apple"), docsrs))]
struct DefaultExecutor;

// Compile-time error for unsupported platforms (except when building docs)
#[cfg(all(not(target_vendor = "apple"), not(docsrs)))]
compile_error!(
    "This crate only supports Apple platforms (macOS, iOS, tvOS, watchOS) with Grand Central Dispatch (GCD). Linux support requires GDK event loop integration (not yet implemented)."
);

trait PlatformExecutor {
    fn exec_main(f: impl FnOnce() + Send + 'static);
    fn exec(f: impl FnOnce() + Send + 'static, priority: Priority);

    fn exec_after(delay: Duration, f: impl FnOnce() + Send + 'static);
}

#[cfg(target_vendor = "apple")]
impl Executor for DefaultExecutor {
    fn spawn<T: Send + 'static>(&self, fut: impl Future<Output = T> + Send + 'static) -> Task<T> {
        spawn(fut)
    }
}

/// An executor that runs tasks on the main thread.
///
/// `MainExecutor` is designed for executing futures that need to run specifically
/// on the main thread, such as UI updates or main-thread-only API calls.
/// This executor implements both `Executor` and `LocalExecutor` traits.
#[cfg(target_vendor = "apple")]
#[derive(Debug)]
pub struct MainExecutor;

#[cfg(target_vendor = "apple")]
impl Executor for MainExecutor {
    fn spawn<T: 'static>(&self, fut: impl Future<Output = T> + 'static) -> Task<T> {
        spawn_local(fut)
    }
}

#[cfg(target_vendor = "apple")]
impl LocalExecutor for MainExecutor {
    fn spawn<T: 'static>(&self, fut: impl Future<Output = T> + 'static) -> Task<T> {
        spawn_local(fut)
    }
}

// Stub implementation only for documentation builds
#[cfg(all(not(target_vendor = "apple"), docsrs))]
impl PlatformExecutor for DefaultExecutor {
    fn exec_main(_f: impl FnOnce() + Send + 'static) {
        // Documentation-only stub - not available at runtime
        unimplemented!("This function is only available for documentation generation")
    }

    fn exec(_f: impl FnOnce() + Send + 'static, _priority: Priority) {
        // Documentation-only stub - not available at runtime
        unimplemented!("This function is only available for documentation generation")
    }

    fn exec_after(_delay: Duration, _f: impl FnOnce() + Send + 'static) {
        // Documentation-only stub - not available at runtime
        unimplemented!("This function is only available for documentation generation")
    }
}

// Stub implementation only for documentation builds
#[cfg(all(not(target_vendor = "apple"), docsrs))]
impl Executor for DefaultExecutor {
    fn spawn<T: Send + 'static>(&self, fut: impl Future<Output = T> + Send + 'static) -> Task<T> {
        let (runnable, task) = async_task::spawn(fut, |_| {});
        task
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
/// use native_executor::{spawn_with_priority, Priority};
///
/// // High-priority task for time-sensitive operations
/// let urgent = spawn_with_priority(async {
///     // Your time-sensitive work here
///     42
/// }, Priority::Default);
///
/// // Background task that won't interfere with UI responsiveness
/// let cleanup = spawn_with_priority(async {
///     // Your background work here
///     "done"
/// }, Priority::Background);
/// ```
pub fn spawn_with_priority<Fut>(future: Fut, priority: Priority) -> Task<Fut::Output>
where
    Fut: Future + Send + 'static,
    Fut::Output: Send,
{
    let (runnable, task) = async_task::spawn(future, move |runnable: Runnable| {
        exec(
            move || {
                runnable.run();
            },
            priority,
        );
    });

    runnable.schedule();
    task
}

/// Creates a new thread-local task that runs on the current thread.
///
/// This function is designed for futures that are not `Send` and must execute
/// on the same thread where they were created. The task will be scheduled to
/// run on the main thread using platform-native scheduling.
///
/// # Arguments
/// * `future` - The non-Send future to execute on the current thread
///
/// # Returns
/// A `Task` handle that can be awaited to retrieve the result
///
/// # Panics
/// This function may panic if called from a thread other than the main thread,
/// depending on the platform implementation.
///
/// # Examples
/// ```rust
/// use native_executor::spawn_local;
/// use std::rc::Rc;
///
/// // Rc is not Send, so we need spawn_local
/// let local_data = Rc::new(42);
/// let task = spawn_local(async move {
///     *local_data + 58
/// });
/// ```
pub fn spawn_local<Fut>(future: Fut) -> Task<Fut::Output>
where
    Fut: Future + 'static,
{
    let (runnable, task) = async_task::spawn_local(future, move |runnable: Runnable| {
        exec_main(move || {
            runnable.run();
        });
    });

    runnable.schedule();
    task
}

/// Creates a new task with default priority.
///
/// This is the primary function for spawning async tasks. The task will be
/// executed with default priority using platform-native scheduling.
///
/// # Arguments
/// * `future` - The future to execute asynchronously
///
/// # Returns
/// A `Task` handle that can be awaited to retrieve the result
///
/// # Examples
/// ```rust
/// use native_executor::spawn;
///
/// let task = spawn(async {
///     // Your async work here
///     42
/// });
/// ```
pub fn spawn<Fut>(future: Fut) -> Task<Fut::Output>
where
    Fut: Future + Send + 'static,
    Fut::Output: Send,
{
    spawn_with_priority(future, Priority::default())
}

/// Creates a new task that executes on the main thread.
///
/// This function schedules a `Send` future to run specifically on the main thread.
/// This is useful for operations that must happen on the main thread, such as
/// UI updates or accessing main-thread-only APIs.
///
/// # Arguments
/// * `future` - The Send future to execute on the main thread
///
/// # Returns
/// A `Task` handle that can be awaited to retrieve the result
///
/// # Examples
/// ```rust
/// use native_executor::spawn_main;
///
/// let task = spawn_main(async {
///     // This runs on the main thread
///     println!("Running on main thread");
///     "done"
/// });
/// ```
pub fn spawn_main<Fut>(future: Fut) -> Task<Fut::Output>
where
    Fut: Future + Send + 'static,
    Fut::Output: Send,
{
    let (runnable, task) = async_task::spawn(future, move |runnable: Runnable| {
        exec_main(move || {
            runnable.run();
        });
    });

    runnable.schedule();
    task
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

        runnable.schedule();
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

/// Schedules a function to be executed on the main thread.
///
/// # Parameters
/// * `f` - The function to execute on the main thread
#[cfg(target_vendor = "apple")]
fn exec_main(f: impl FnOnce() + Send + 'static) {
    DefaultExecutor::exec_main(f);
}

#[cfg(all(not(target_vendor = "apple"), docsrs))]
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

#[cfg(all(not(target_vendor = "apple"), docsrs))]
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

#[cfg(all(not(target_vendor = "apple"), docsrs))]
fn exec_after(delay: Duration, f: impl FnOnce() + Send + 'static) {
    DefaultExecutor::exec_after(delay, f);
}
