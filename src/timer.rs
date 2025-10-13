//! Platform-native timers with high precision.
//!
//! Leverages OS scheduling primitives (GCD `dispatch_after` on Apple) for accurate
//! timing without busy-waiting.
//!
//! ```rust
//! use native_executor::timer::{Timer, sleep};
//! use std::time::Duration;
//!
//! # async {
//! Timer::after(Duration::from_millis(100)).await;  // Precise timing
//! Timer::after_secs(2).await;                      // Convenience method  
//! sleep(1).await;                                  // Simple sleep
//! # };
//! ```

use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
    time::Duration,
};

use alloc::sync::Arc;

use crate::{NativeExecutor, PlatformExecutor};

/// A high-precision future that completes after a specified duration.
///
/// `Timer` provides platform-native timing capabilities that leverage operating system
/// scheduling primitives for accurate delays without busy-waiting. The timer integrates
/// seamlessly with async/await and provides zero-cost abstraction over native OS APIs.
///
/// # Platform Behavior
/// - **Apple platforms**: Uses GCD's `dispatch_after` for precise scheduling
/// - **Other platforms**: Will use platform-specific high-resolution APIs
///
/// # Performance
/// Unlike thread-based sleep implementations, `Timer` doesn't block threads and
/// allows the executor to handle other tasks while waiting.
///
/// # Examples
/// ```rust
/// use native_executor::timer::Timer;
/// use std::time::Duration;
///
/// async fn precise_timing() {
///     // Millisecond precision timing
///     Timer::after(Duration::from_millis(250)).await;
///     
///     // Second-based convenience method
///     Timer::after_secs(2).await;
/// }
/// ```
#[derive(Debug)]
pub struct Timer {
    /// The duration to wait. This is taken (set to None) after the timer is started.
    duration: Option<Duration>,
    /// Atomic flag to track whether the timer has completed.
    /// This is shared between the future and the callback that will be executed after the duration.
    finished: Arc<AtomicBool>,
}

impl Timer {
    /// Creates a new `Timer` that will complete after the specified duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - The amount of time to wait before the timer completes.
    ///
    /// # Returns
    ///
    /// A new `Timer` instance that can be awaited.
    ///
    /// # Example
    ///
    /// ```
    /// use native_executor::timer::Timer;
    /// use std::time::Duration;
    ///
    /// async fn example() {
    ///     // Wait for 1 second
    ///     Timer::after(Duration::from_secs(1)).await;
    ///     println!("One second has passed!");
    /// }
    /// ```
    #[must_use]
    pub fn after(duration: Duration) -> Self {
        Self {
            duration: Some(duration),
            finished: Arc::default(),
        }
    }

    /// Creates a new `Timer` that will complete after the specified number of seconds.
    ///
    /// This is a convenience method that wraps `Timer::after` with `Duration::from_secs`.
    ///
    /// # Arguments
    ///
    /// * `secs` - The number of seconds to wait before the timer completes.
    ///
    /// # Returns
    ///
    /// A new `Timer` instance that can be awaited.
    ///
    /// # Example
    ///
    /// ```
    /// use native_executor::timer::Timer;
    ///
    /// async fn example() {
    ///     // Wait for 5 seconds
    ///     Timer::after_secs(5).await;
    ///     println!("Five seconds have passed!");
    /// }
    /// ```
    #[must_use]
    pub fn after_secs(secs: u64) -> Self {
        Self::after(Duration::from_secs(secs))
    }
}

impl Future for Timer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // If the timer has already finished, return Ready
        if self.finished.load(Ordering::Acquire) {
            return Poll::Ready(());
        }

        // If this is the first poll, set up the timer
        if let Some(duration) = self.duration.take() {
            let waker = cx.waker().clone();
            let finished = self.finished.clone();

            // Schedule the callback to run after the specified duration
            NativeExecutor::exec_after(duration, move || {
                // Mark the timer as finished
                finished.store(true, Ordering::Release);
                // Wake the task that's waiting on this timer
                waker.wake();
            }, crate::Priority::Default);
        }

        // The timer hasn't completed yet
        Poll::Pending
    }
}

/// Suspends the current async task for the specified number of seconds.
///
/// This convenience function provides a simple interface for second-based delays,
/// using the same high-precision platform-native timing as `Timer::after`.
///
/// # Arguments
/// * `secs` - The number of seconds to sleep
///
/// # Platform Integration
/// Uses the same platform-native scheduling as `Timer` for consistent precision.
///
/// # Examples
/// ```rust
/// use native_executor::timer::sleep;
///
/// async fn delayed_operation() {
///     println!("Starting operation");
///     sleep(2).await;  // High-precision 2-second delay
///     println!("Operation resumed after 2 seconds");
/// }
/// ```
pub async fn sleep(secs: u64) {
    Timer::after(Duration::from_secs(secs)).await;
}
