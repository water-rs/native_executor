use async_task as async_task_crate;
use executor_core::async_task::{self as core_async_task, AsyncTask, Runnable};
use std::{
    collections::VecDeque,
    future::Future,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};

use crate::{
    PlatformExecutor, Priority,
    polyfill::{self, executor::PolyfillExecutor, timer::PolyfillTimer},
};

#[derive(Debug, Clone, Copy)]
pub struct AndroidExecutor(PolyfillExecutor);

static MAIN_DISPATCHER: OnceLock<MainDispatcher> = OnceLock::new();

/// Register the real Android UI thread as the main executor target.
///
/// This must be called on the application's main (UI) thread before calling
/// [`NativeExecutor::spawn_main`] or [`NativeExecutor::spawn_local`] on Android.
/// The function binds to the thread's choreographer and uses it to schedule
/// main-thread tasks without spinning up an additional thread.
#[cfg(target_os = "android")]
pub fn register_android_main_thread() {
    polyfill::register_main_thread();
    MainDispatcher::install();
}

impl PlatformExecutor for AndroidExecutor {
    type Timer = PolyfillTimer;

    fn sleep(duration: Duration) -> Self::Timer {
        PolyfillTimer::after(duration)
    }

    fn with_priority(priority: Priority) -> Self {
        AndroidExecutor(PolyfillExecutor::with_priority(priority))
    }

    fn spawn<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        self.0.spawn(fut)
    }

    fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        let dispatcher = MainDispatcher::get();
        let (runnable, task) = core_async_task::spawn(fut, |runnable| {
            dispatcher.enqueue(runnable);
        });
        dispatcher.enqueue(runnable);
        task
    }

    fn spawn_main_local<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future + 'static,
    {
        polyfill::assert_main_thread("spawn_main_local");
        let (runnable, task) = async_task_crate::spawn_local(fut, |runnable| runnable.run());
        runnable.run();
        task.into()
    }
}

#[derive(Debug)]
struct MainDispatcher {
    choreographer: *mut ndk_sys::AChoreographer,
    queue: Mutex<VecDeque<Runnable>>,
    callback_scheduled: AtomicBool,
}

impl MainDispatcher {
    fn install() {
        if MAIN_DISPATCHER.get().is_some() {
            return;
        }

        let choreographer = unsafe { ndk_sys::AChoreographer_getInstance() };
        assert!(
            !choreographer.is_null(),
            "AChoreographer_getInstance returned null. Call register_android_main_thread from the UI thread."
        );

        MAIN_DISPATCHER
            .set(MainDispatcher {
                choreographer,
                queue: Mutex::new(VecDeque::new()),
                callback_scheduled: AtomicBool::new(false),
            })
            .expect("MainDispatcher already initialized");
    }

    fn get() -> &'static MainDispatcher {
        MAIN_DISPATCHER
            .get()
            .unwrap_or_else(|| panic!("register_android_main_thread must be called on the UI thread before spawning to the main executor"))
    }

    fn enqueue(&self, runnable: Runnable) {
        {
            let mut guard = self.queue.lock().expect("MainDispatcher queue poisoned");
            guard.push_back(runnable);
        }
        self.schedule_callback();
    }

    fn schedule_callback(&self) {
        if self
            .callback_scheduled
            .swap(true, Ordering::AcqRel)
        {
            return;
        }
        unsafe {
            ndk_sys::AChoreographer_postFrameCallback(
                self.choreographer,
                Some(run_frame_callbacks),
                self as *const _ as *mut _,
            );
        }
    }

    fn run_ready(&self) {
        loop {
            let runnable = {
                let mut guard = self.queue.lock().expect("MainDispatcher queue poisoned");
                guard.pop_front()
            };
            match runnable {
                Some(r) => r.run(),
                None => break,
            }
        }
        self.callback_scheduled.store(false, Ordering::Release);

        if !self.queue.lock().expect("MainDispatcher queue poisoned").is_empty() {
            self.schedule_callback();
        }
    }
}

unsafe extern "C" fn run_frame_callbacks(
    _frame_time_nanos: libc::c_long,
    data: *mut core::ffi::c_void,
) {
    if data.is_null() {
        return;
    }
    let dispatcher = unsafe { &*(data as *const MainDispatcher) };
    dispatcher.run_ready();
}
