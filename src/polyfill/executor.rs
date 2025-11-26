#![allow(dead_code)]

use async_task::{self as async_task_crate, Runnable};
use executor_core::{Executor, async_task::AsyncTask};
use futures_lite::future::block_on;
use std::{
    future::Future,
    panic::catch_unwind,
    sync::{Once, OnceLock},
    thread,
};

use crate::{
    PlatformExecutor,
    polyfill::{assert_main_thread, register_main_thread, timer::PolyfillTimer},
};

/// Polyfill executor implementation using async-executor.
///
/// This executor spins up a pool of background worker threads and a synthetic
/// "main thread" to emulate the APIs that platforms like Apple expose.
/// Because it is not backed by a real OS event loop, its behavior differs
/// from the native executors and should be considered a portability fallback.
#[derive(Debug, Clone, Copy, Default)]
pub struct PolyfillExecutor;

static EXECUTOR: OnceLock<async_executor::Executor<'static>> = OnceLock::new();
static WORKER_THREADS: Once = Once::new();

fn global() -> &'static async_executor::Executor<'static> {
    let executor = EXECUTOR.get_or_init(async_executor::Executor::new);
    WORKER_THREADS.call_once(|| spawn_worker_threads(executor));
    executor
}

fn spawn_worker_threads(executor: &'static async_executor::Executor<'static>) {
    let num_threads = num_cpus::get().max(1);
    for idx in 0..num_threads {
        let thread_name = format!("native-executor::polyfill-{idx}");
        let exec = executor;
        thread::Builder::new()
            .name(thread_name)
            .spawn(move || run_worker_loop(exec))
            .expect("failed to spawn polyfill worker thread");
    }
}

fn run_worker_loop(executor: &'static async_executor::Executor<'static>) {
    loop {
        let _ = catch_unwind(|| {
            block_on(executor.run(std::future::pending::<()>()));
        });
    }
}

static MAIN_EXECUTOR: OnceLock<async_executor::Executor<'static>> = OnceLock::new();

/// Starts the synthetic "main thread" executor on the current thread.
///
/// Platforms that rely on the polyfill do not have an OS-provided main thread,
/// so we create one by running this function on a thread of your choice. It
/// blocks forever, driving tasks spawned via [`crate::spawn_main`] and
/// [`crate::spawn_local`]. Most applications spawn a dedicated thread for this
/// purpose when running on an unsupported target.
///
/// # Panics
///
/// Panics if the main executor has already been started.
pub fn start_main_executor() {
    register_main_thread();
    let main_exec = async_executor::Executor::new();
    MAIN_EXECUTOR
        .set(main_exec)
        .expect("Main executor already started");
    let main_exec = MAIN_EXECUTOR
        .get()
        .expect("Unexpected error: main executor not set");
    loop {
        let _ = catch_unwind(|| {
            block_on(main_exec.run(std::future::pending::<()>()));
        });
    }
}

fn main_executor() -> &'static async_executor::Executor<'static> {
    MAIN_EXECUTOR.get().expect("polyfill main executor not started. Call `native_executor::polyfill::start_main_executor` on a dedicated thread before using `spawn_main` or `spawn_local`.")
}

impl PlatformExecutor for PolyfillExecutor {
    type Timer = PolyfillTimer;
    fn with_priority(_priority: crate::Priority) -> Self {
        // PolyfillExecutor does not support priorities, so this is a no-op.
        Self
    }

    fn spawn<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        global().spawn(fut).into()
    }

    fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        main_executor().spawn(fut).into()
    }

    fn spawn_main_local<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future + 'static,
    {
        assert_main_thread("spawn_main_local");
        let (runnable, task) = async_task_crate::spawn_local(fut, |runnable: Runnable| {
            runnable.run();
        });
        runnable.run();
        task.into()
    }

    fn sleep(duration: std::time::Duration) -> Self::Timer {
        PolyfillTimer::after(duration)
    }
}

impl Executor for PolyfillExecutor {
    type Task<T: Send + 'static> = AsyncTask<T>;
    fn spawn<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        global().spawn(fut).into()
    }
}
