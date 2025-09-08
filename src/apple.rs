//! Apple platform implementation using Grand Central Dispatch (GCD).
//!
//! This module provides the native executor implementation for Apple platforms
//! (macOS, iOS, tvOS, watchOS) by leveraging Grand Central Dispatch for optimal
//! performance and system integration.

use core::time::Duration;
use dispatch::{Queue, QueuePriority};

use crate::{PlatformExecutor, Priority};

impl From<Priority> for QueuePriority {
    fn from(val: Priority) -> Self {
        match val {
            Priority::Background => Self::Background,
            Priority::Utility => Self::Low,
            Priority::UserInitiated | Priority::UserInteractive => Self::High,
            // Fallback to Default for any future variants
            _ => Self::Default,
        }
    }
}
/// Apple platform executor implementation using Grand Central Dispatch.
///
/// This executor provides optimal performance on Apple platforms by directly
/// leveraging GCD's system-level thread pools and scheduling primitives.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApplePlatformExecutor;

impl PlatformExecutor for ApplePlatformExecutor {
    fn exec_main(f: impl FnOnce() + Send + 'static) {
        let main = Queue::main();
        main.exec_async(f);
    }

    fn exec(f: impl FnOnce() + Send + 'static, priority: Priority) {
        let queue = Queue::global(priority.into());
        queue.exec_async(f);
    }

    fn exec_after(delay: Duration, f: impl FnOnce() + Send + 'static) {
        let queue = Queue::global(dispatch::QueuePriority::Default);
        queue.exec_after(delay, f);
    }
}
