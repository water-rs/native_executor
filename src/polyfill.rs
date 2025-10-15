//! Polyfill executor implementation using async-executor.

use futures_lite::future::block_on;
use std::{panic::catch_unwind, sync::OnceLock};

use crate::PlatformExecutor;

/// Polyfill executor implementation using async-executor.
/// This executor is used on platforms that do not have a native executor implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct PolyfillExecutor;

static EXECUTOR: OnceLock<async_executor::Executor<'static>> = OnceLock::new();

fn global() -> &'static async_executor::Executor<'static> {
    EXECUTOR.get_or_init(|| {
        let exec = async_executor::Executor::new();
        let num_threads = num_cpus::get().max(1);
        for _ in 0..num_threads {
            let executor = global();
            std::thread::spawn(move || {
                loop {
                    let _ = catch_unwind(|| {
                        block_on(executor.run(std::future::pending::<()>()));
                    });
                }
            });
        }
        exec
    })
}

static MAIN_EXECUTOR: OnceLock<async_executor::Executor<'static>> = OnceLock::new();

/// Starts the main executor on a dedicated thread.
/// This function is blocking and should be called once at the start of the program.
///
/// # Panics
///
/// Panics if the main executor has already been started.
pub fn start_main_executor() {
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
    MAIN_EXECUTOR.get().expect("Main executor not started")
}

impl PlatformExecutor for PolyfillExecutor {
    fn exec(f: impl FnOnce() + Send + 'static, _priority: crate::Priority) {
        global().spawn(async move { f() }).detach();
    }
    fn exec_after(
        delay: std::time::Duration,
        f: impl FnOnce() + Send + 'static,
        _priority: crate::Priority,
    ) {
        global()
            .spawn(async move {
                async_io::Timer::after(delay).await;
                f();
            })
            .detach();
    }
    fn exec_main(f: impl FnOnce() + Send + 'static) {
        main_executor().spawn(async move { f() }).detach();
    }
}
