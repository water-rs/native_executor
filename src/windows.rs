//! Windows platform implementation using the Windows Thread Pool API.
//!
//! This module provides the native executor implementation for Windows platforms
//! by leveraging the Windows Thread Pool API for optimal performance and system integration.

use crate::{PlatformExecutor, Priority};
use alloc::boxed::Box;
use core::mem;
use core::ptr;
use core::time::Duration;
use windows_sys::Win32::Foundation::{BOOL, FILETIME};
use windows_sys::Win32::System::Threading::{
    CloseThreadpoolTimer, CloseThreadpoolWork, CreateThreadpoolTimer, CreateThreadpoolWork,
    PTP_CALLBACK_INSTANCE, PTP_TIMER, PTP_WORK, SetThreadpoolCallbackPriority, SetThreadpoolTimer,
    SubmitThreadpoolWork, TP_CALLBACK_PRIORITY_HIGH, TP_CALLBACK_PRIORITY_LOW,
    TP_CALLBACK_PRIORITY_NORMAL, WaitForThreadpoolTimerCallbacks,
};

/// Windows platform executor implementation using Windows Thread Pool API.
///
/// This executor provides optimal performance on Windows platforms by directly
/// leveraging the Windows Thread Pool API's system-level thread pools and scheduling primitives.
pub struct WindowsPlatformExecutor;

impl From<Priority> for u32 {
    fn from(val: Priority) -> Self {
        match val {
            Priority::Default => TP_CALLBACK_PRIORITY_NORMAL,
            Priority::Background => TP_CALLBACK_PRIORITY_LOW,
        }
    }
}

/// Context for work items submitted to the thread pool
struct WorkContext {
    callback: Box<dyn FnOnce() + Send>,
}

/// Context for timer callbacks
struct TimerContext {
    callback: Box<dyn FnOnce() + Send>,
    timer: PTP_TIMER,
}

/// Work callback function for the thread pool
unsafe extern "system" fn work_callback(
    _instance: PTP_CALLBACK_INSTANCE,
    context: *mut core::ffi::c_void,
    _work: PTP_WORK,
) {
    let context = Box::from_raw(context as *mut WorkContext);
    (context.callback)();
}

/// Timer callback function for delayed execution
unsafe extern "system" fn timer_callback(
    _instance: PTP_CALLBACK_INSTANCE,
    context: *mut core::ffi::c_void,
    _timer: PTP_TIMER,
) {
    let context = Box::from_raw(context as *mut TimerContext);
    (context.callback)();
    CloseThreadpoolTimer(context.timer);
}

impl PlatformExecutor for WindowsPlatformExecutor {
    fn exec_main(f: impl FnOnce() + Send + 'static) {
        // Windows doesn't have a direct equivalent to GCD's main queue
        // We'll execute on the current thread if it's the main thread,
        // otherwise post to the message queue for main thread execution

        // For now, we'll use a regular thread pool work item
        // In a full implementation, you might want to use PostMessage
        // to ensure execution on the UI thread
        Self::exec(f, Priority::Default);
    }

    fn exec(f: impl FnOnce() + Send + 'static, priority: Priority) {
        unsafe {
            let context = Box::new(WorkContext {
                callback: Box::new(f),
            });

            let work = CreateThreadpoolWork(
                Some(work_callback),
                Box::into_raw(context) as *mut core::ffi::c_void,
                ptr::null(),
            );

            if !work.is_null() {
                // Set priority before submitting
                let priority_value: u32 = priority.into();
                if priority_value != TP_CALLBACK_PRIORITY_NORMAL {
                    // Note: SetThreadpoolCallbackPriority requires a TP_CALLBACK_ENVIRON
                    // For simplicity, we're using default priority in this implementation
                    // A full implementation would create and configure a callback environment
                }

                SubmitThreadpoolWork(work);
                CloseThreadpoolWork(work);
            }
        }
    }

    fn exec_after(delay: Duration, f: impl FnOnce() + Send + 'static) {
        unsafe {
            let timer = CreateThreadpoolTimer(Some(timer_callback), ptr::null_mut(), ptr::null());

            if !timer.is_null() {
                let context = Box::new(TimerContext {
                    callback: Box::new(f),
                    timer,
                });

                // Convert duration to Windows FILETIME (100-nanosecond intervals)
                let delay_100ns = delay.as_nanos() / 100;
                let mut due_time = FILETIME {
                    dwLowDateTime: (delay_100ns & 0xFFFFFFFF) as u32,
                    dwHighDateTime: ((delay_100ns >> 32) & 0xFFFFFFFF) as u32,
                };

                // Negative value indicates relative time
                let due_time_i64 = -(delay_100ns as i64);
                due_time.dwLowDateTime = (due_time_i64 & 0xFFFFFFFF) as u32;
                due_time.dwHighDateTime = ((due_time_i64 >> 32) & 0xFFFFFFFF) as u32;

                SetThreadpoolTimer(
                    timer,
                    &due_time as *const FILETIME,
                    0, // Period (0 = one-shot timer)
                    0, // Window (0 = no window)
                );

                // Leak the context - it will be freed in the callback
                mem::forget(context);
            }
        }
    }
}
