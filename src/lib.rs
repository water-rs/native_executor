use std::{future::Future, time::Duration};

#[cfg(target_vendor = "apple")]
mod apple;
use executor_core::{Executor, LocalExecutor, async_task::AsyncTask};

#[cfg(target_os = "android")]
mod android;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(any(feature = "polyfill", target_os = "android"))]
pub(crate) mod polyfill;

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

impl NativeExecutor {
    pub fn new() -> Self {
        Self::with_priority(Priority::default())
    }

    pub fn with_priority(priority: Priority) -> Self {
        Self(<NativeExecutorInner as PlatformExecutor>::with_priority(
            priority,
        ))
    }

    pub fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        <NativeExecutorInner as PlatformExecutor>::spawn_main(&self.0, fut)
    }

    pub fn spawn<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        <NativeExecutorInner as PlatformExecutor>::spawn(&self.0, fut)
    }

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
        self.0.spawn(fut)
    }
}

/// # Panics
/// It panics if not on main thread.
impl LocalExecutor for NativeExecutor {
    type Task<T: 'static> = AsyncTask<T>;
    fn spawn_local<Fut>(&self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
    {
        <NativeExecutorInner as PlatformExecutor>::spawn_main_local(&self.0, fut)
    }
}

pub fn sleep(duration: Duration) -> NativeTimer {
    NativeTimer::after(duration)
}
