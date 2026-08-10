//! Platform-native async executor that bridges directly to OS event loops.
//!
//! Tasks use structured concurrency semantics: every `spawn*` call returns an
//! [`AsyncTask`] handle, and dropping that handle cancels the task unless you
//! awaited it or called [`AsyncTask::detach`] to explicitly opt into
//! fire-and-forget execution.
use std::{future::Future, time::Duration};

#[cfg(target_vendor = "apple")]
mod apple;
use executor_core::{Executor, LocalExecutor, async_task::AsyncTask};

#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(any(
    all(feature = "polyfill", not(target_arch = "wasm32")),
    target_os = "android"
))]
pub mod polyfill;

#[cfg(all(
    not(feature = "polyfill"),
    not(target_vendor = "apple"),
    not(target_os = "android"),
    not(target_arch = "wasm32")
))]
compile_error!(
    "native-executor has no backend for this target; enable the `polyfill` feature \
     to build on unsupported platforms."
);

/// Task execution priority levels for controlling scheduler behavior.
///
/// These priority levels map to platform-native scheduling priorities,
/// allowing fine-grained control over task execution order and resource allocation.
///
/// # Platform notes
/// - Apple (macOS/iOS/Catalyst) and wasm/web backends are ready to use with no extra setup.
/// - Android **requires** calling [`register_android_main_thread`](crate::register_android_main_thread)
///   on the real UI thread before any `spawn_main`/`spawn_main_local` usage, so the executor can
///   dispatch tasks back to the platform main looper.
/// - Polyfill backend (enabled via the `polyfill` feature on unsupported targets) needs you to
///   create a dedicated thread and call [`polyfill::executor::start_main_executor`] there to
///   simulate a main thread before using `spawn_main`/`spawn_local`.
///
/// # Choosing an executor
///
/// [`NativeExecutor`] is the global, work-stealing executor and has no
/// main-thread affinity, so it is always available.
///
/// [`NativeMainExecutor`] dispatches to the platform main thread and only exists
/// where such a thread does, which is why it is constructed through an
/// [`Option`] rather than failing at the first spawn.
///
/// **If your program owns an event loop — winit, GTK/glib, a frame pump — do not
/// use [`NativeMainExecutor`].** Implement [`LocalExecutor`] over that loop so
/// tasks run on the thread that owns your windows and graphics resources.
/// [`polyfill::executor::start_main_executor`] would otherwise declare an
/// unrelated thread "main" and dispatch your UI work to the wrong one.
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

trait PlatformExecutor {
    type Timer: Future<Output = ()>;
    fn with_priority(priority: Priority) -> Self;
    fn sleep(duration: Duration) -> Self::Timer;
    fn spawn<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static;
    fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static;
    fn spawn_main_local<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future + 'static;
}

#[cfg(target_vendor = "apple")]
type NativeExecutorInner = apple::AppleExecutor;

#[cfg(target_os = "android")]
type NativeExecutorInner = android::AndroidExecutor;

#[cfg(target_arch = "wasm32")]
type NativeExecutorInner = web::WebExecutor;

#[cfg(all(
    feature = "polyfill",
    not(any(target_vendor = "apple", target_os = "android", target_arch = "wasm32"))
))]
type NativeExecutorInner = polyfill::executor::PolyfillExecutor;

#[cfg(target_os = "android")]
pub use android::register_android_main_thread;

#[derive(Debug)]
pub struct NativeExecutor(NativeExecutorInner);

impl Default for NativeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeExecutor {
    #[must_use]
    pub fn new() -> Self {
        Self::with_priority(Priority::default())
    }

    #[must_use]
    pub fn with_priority(priority: Priority) -> Self {
        Self(<NativeExecutorInner as PlatformExecutor>::with_priority(
            priority,
        ))
    }

    pub fn spawn<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        <NativeExecutorInner as PlatformExecutor>::spawn(&self.0, fut)
    }
}

/// Reports whether this target has an established main thread to dispatch to.
///
/// Apple and web targets always do. Android needs
/// [`register_android_main_thread`], and the polyfill needs
/// [`polyfill::executor::start_main_executor`].
#[must_use]
#[allow(
    clippy::missing_const_for_fn,
    reason = "only the apple/web arm is a constant; the others read a OnceLock"
)]
pub fn main_thread_available() -> bool {
    #[cfg(any(target_vendor = "apple", target_arch = "wasm32"))]
    {
        true
    }
    #[cfg(target_os = "android")]
    {
        android::is_main_thread_registered()
    }
    #[cfg(all(
        feature = "polyfill",
        not(target_vendor = "apple"),
        not(target_os = "android"),
        not(target_arch = "wasm32")
    ))]
    {
        polyfill::is_main_thread_registered()
    }
}

/// Dispatches work to the platform main thread.
///
/// This is deliberately a separate type from [`NativeExecutor`]: only targets
/// with an established main thread can produce one, so handing a main-thread
/// executor to an API that needs one is checked when you construct it rather
/// than when the first task is spawned.
///
/// **If you own an event loop, do not use this type.** Implement
/// [`LocalExecutor`] over your own loop instead — winit, GTK/glib, or a frame
/// pump — so tasks run on the thread that owns your windows. This type is for
/// hosts that have no loop of their own.
#[derive(Debug)]
pub struct NativeMainExecutor(NativeExecutorInner);

impl NativeMainExecutor {
    /// Returns `None` when no main thread has been established.
    ///
    /// That is never the case on Apple and web targets. On Android it means
    /// [`register_android_main_thread`] has not run yet; under the polyfill it
    /// means no thread is running
    /// [`polyfill::executor::start_main_executor`].
    #[must_use]
    pub fn new() -> Option<Self> {
        Self::with_priority(Priority::default())
    }

    /// Like [`NativeMainExecutor::new`], with an explicit priority.
    #[must_use]
    pub fn with_priority(priority: Priority) -> Option<Self> {
        main_thread_available().then(|| {
            Self(<NativeExecutorInner as PlatformExecutor>::with_priority(
                priority,
            ))
        })
    }

    pub fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        <NativeExecutorInner as PlatformExecutor>::spawn_main(&self.0, fut)
    }

    /// # Panics
    ///
    /// Under the polyfill, panics unless called from the thread running
    /// [`polyfill::executor::start_main_executor`]; that backend runs the task
    /// inline rather than dispatching it.
    pub fn spawn_main_local<Fut>(&self, fut: Fut) -> <Self as LocalExecutor>::Task<Fut::Output>
    where
        Fut: Future + 'static,
    {
        <NativeExecutorInner as PlatformExecutor>::spawn_main_local(&self.0, fut)
    }
}

/// A timer that completes after a specified duration.
#[derive(Debug)]
pub struct NativeTimer(<NativeExecutorInner as PlatformExecutor>::Timer);

impl NativeTimer {
    #[must_use]
    pub fn after(duration: Duration) -> Self {
        Self(<NativeExecutorInner as PlatformExecutor>::sleep(duration))
    }
}

impl Future for NativeTimer {
    type Output = ();
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::pin!(&mut self.0).poll(cx)
    }
}

impl Executor for NativeExecutor {
    type Task<T: Send + 'static> = AsyncTask<T>;
    fn spawn<Fut>(&self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        <NativeExecutorInner as PlatformExecutor>::spawn(&self.0, fut)
    }
}

/// # Panics
///
/// Under the polyfill, panics if not called from the registered main thread.
impl LocalExecutor for NativeMainExecutor {
    type Task<T: 'static> = AsyncTask<T>;
    fn spawn_local<Fut>(&self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
    {
        <NativeExecutorInner as PlatformExecutor>::spawn_main_local(&self.0, fut)
    }
}

#[must_use]
pub fn sleep(duration: Duration) -> NativeTimer {
    NativeTimer::after(duration)
}
