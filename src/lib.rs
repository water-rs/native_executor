#![doc = include_str!("../README.md")]
#![no_std]
#![warn(missing_docs, missing_debug_implementations)]
extern crate alloc;

#[cfg(target_vendor = "apple")]
mod apple;

#[cfg(all(not(target_vendor = "apple"), not(docsrs)))]
compile_error!("native_executor currently only supports Apple platforms, more to come soon!");

use async_task::Task;
use executor_core::{async_task::AsyncTask, Executor, LocalExecutor};
pub mod mailbox;
pub mod timer;
use core::time::Duration;

#[cfg(target_vendor = "apple")]
pub use apple::ApplePlatformExecutor as NativeExecutor;

mod unsupported {
    use core::time::Duration;

    use crate::{PlatformExecutor, Priority};

    #[allow(unused)]
    /// A stub executor for unsupported platforms that panics on use.
    pub struct UnsupportedExecutor;
    impl PlatformExecutor for UnsupportedExecutor {
        fn exec_main(_f: impl FnOnce() + Send + 'static) {
            panic!("exec_main is not supported on this platform");
        }

        fn exec(_f: impl FnOnce() + Send + 'static, _priority: Priority) {
            panic!("exec is not supported on this platform");
        }

        fn exec_after(_delay: Duration, _f: impl FnOnce() + Send + 'static) {
            panic!("exec_after is not supported on this platform");
        }
    }
}

#[cfg(not(target_vendor = "apple"))]
/// The native executor implementation.
pub use unsupported::UnsupportedExecutor as NativeExecutor;

trait PlatformExecutor {
    fn exec_main(f: impl FnOnce() + Send + 'static);
    fn exec(f: impl FnOnce() + Send + 'static, priority: Priority);

    fn exec_after(delay: Duration, f: impl FnOnce() + Send + 'static);
}

impl Executor for NativeExecutor {
    type Task<T: Send + 'static>=AsyncTask<T>;

    fn spawn<Fut>(&self, fut: Fut) -> Self::Task<Fut::Output>
        where
            Fut: Future<Output: Send> + Send + 'static {
        spawn(fut).into()
    } 
    
}



impl LocalExecutor for NativeExecutor {
    type Task<T: 'static> = AsyncTask<T>;
    fn spawn_local<Fut>(&self, fut: Fut) -> Self::Task<Fut::Output>
        where
            Fut: Future + 'static {
         spawn_local(fut).into()
    }
}

use async_task::Runnable;

/// Task execution priority levels for controlling scheduler behavior.
///
/// These priority levels map to platform-native scheduling priorities,
/// allowing fine-grained control over task execution order and resource allocation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
    /// Higher priority for user-initiated tasks that require prompt execution.
    /// This priority is suitable for tasks that directly impact user experience,
    /// such as responding to user input or updating the UI.
    UserInitiated,
    /// Highest priority for tasks that require immediate attention to maintain
    /// application responsiveness.
    /// This priority should be reserved for critical operations that must
    /// complete as soon as possible, such as rendering UI updates or handling
    /// real-time data.
    UserInteractive,
    /// Lowest priority for tasks that can be deferred until the system is idle.
    /// This priority is suitable for maintenance tasks, prefetching data,
    /// or other operations that do not need to run immediately and can wait
    /// until the system is less busy.
    Utility,
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
        NativeExecutor::exec(
            move || {
                runnable.run();
            },
            priority,
        );
    });

    runnable.schedule();
    task
}

/// Creates a new thread-local task that runs on the main thread.
///
/// This function is designed for futures that are not `Send` and must execute
/// on the main thread.
///
/// # Arguments
/// * `future` - The non-Send future to execute on the main thread
///
/// # Returns
/// A `Task` handle that can be awaited to retrieve the result
///
/// # Panics
/// This function may panic if not called from a main thread
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
        NativeExecutor::exec_main(move || {
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
        NativeExecutor::exec_main(move || {
            runnable.run();
        });
    });

    runnable.schedule();
    task
}
