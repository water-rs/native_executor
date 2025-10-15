//! Android platform executor implementation.
//!
//! This module provides a minimal native executor for Android targets.
//! It leverages long-lived worker threads to execute queued jobs and
//! supports delayed scheduling for timer integration.
use core::time::Duration;
use std::{
    sync::{OnceLock, mpsc},
    thread,
};

use crate::{PlatformExecutor, Priority};

type Job = Box<dyn FnOnce() + Send + 'static>;

#[derive(Debug)]
struct ExecutorQueue {
    sender: mpsc::Sender<Job>,
}

impl ExecutorQueue {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<Job>();
        // The worker processes jobs sequentially on a dedicated OS thread.
        let _ = thread::spawn(move || {
            while let Ok(job) = receiver.recv() {
                job();
            }
        });
        Self { sender }
    }

    fn dispatch(&self, job: Job) {
        match self.sender.send(job) {
            Ok(()) => {}
            Err(err) => (err.0)(),
        }
    }

    fn dispatch_after(&self, delay: Duration, job: Job) {
        if delay.is_zero() {
            self.dispatch(job);
            return;
        }

        let sender = self.sender.clone();
        let _ = thread::spawn(move || {
            thread::sleep(delay);
            if let Err(err) = sender.send(job) {
                (err.0)();
            }
        });
    }
}

struct AndroidRuntime {
    main: ExecutorQueue,
    default: ExecutorQueue,
    background: ExecutorQueue,
}

impl AndroidRuntime {
    fn instance() -> &'static Self {
        static RUNTIME: OnceLock<AndroidRuntime> = OnceLock::new();

        RUNTIME.get_or_init(|| AndroidRuntime {
            main: ExecutorQueue::new(),
            default: ExecutorQueue::new(),
            background: ExecutorQueue::new(),
        })
    }

    fn queue_for_priority(&self, priority: Priority) -> &ExecutorQueue {
        match priority {
            Priority::Background | Priority::Utility => &self.background,
            Priority::UserInteractive => &self.main,
            Priority::UserInitiated => &self.default,
            Priority::Default => &self.default,
        }
    }
}

/// Android native executor.
///
/// This executor routes work onto a small set of dedicated worker threads,
/// providing basic priority separation and delayed scheduling support that
/// integrates with the crate's timer utilities.
#[derive(Clone, Copy, Debug, Default)]
pub struct AndroidPlatformExecutor;

impl PlatformExecutor for AndroidPlatformExecutor {
    fn exec_main(f: impl FnOnce() + Send + 'static) {
        AndroidRuntime::instance().main.dispatch(Box::new(f));
    }

    fn exec(f: impl FnOnce() + Send + 'static, priority: Priority) {
        AndroidRuntime::instance()
            .queue_for_priority(priority)
            .dispatch(Box::new(f));
    }

    fn exec_after(delay: Duration, f: impl FnOnce() + Send + 'static, priority: Priority) {
        AndroidRuntime::instance()
            .queue_for_priority(priority)
            .dispatch_after(delay, Box::new(f));
    }
}
