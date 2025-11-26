//! Polyfill executor implementation using async-executor.

use std::{
    sync::OnceLock,
    thread::{self, ThreadId},
};

#[cfg(any(feature = "polyfill", target_os = "android"))]
pub mod executor;

// polyfill timer would be used on non-apple platforms, including android
#[cfg(any(feature = "polyfill", target_os = "android"))]
pub mod timer;

static MAIN_THREAD_ID: OnceLock<ThreadId> = OnceLock::new();

pub(crate) fn register_main_thread() {
    let id = thread::current().id();
    if MAIN_THREAD_ID.set(id).is_err() {
        let existing = MAIN_THREAD_ID
            .get()
            .copied()
            .expect("main thread id not initialized");
        if existing != id {
            panic!("polyfill main executor already registered on a different thread");
        }
    }
}

pub(crate) fn assert_main_thread(op: &str) {
    if !is_main_thread() {
        panic!("{op} must be called from the polyfill main thread");
    }
}

pub(crate) fn is_main_thread() -> bool {
    MAIN_THREAD_ID
        .get()
        .map(|id| *id == thread::current().id())
        .unwrap_or(false)
}
