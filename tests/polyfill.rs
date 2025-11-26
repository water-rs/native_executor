#![cfg(all(feature = "polyfill", not(target_arch = "wasm32"), not(target_os = "android")))]

use executor_core::Executor;
use futures_lite::future::block_on;
use native_executor::polyfill::timer::PolyfillTimer;
use native_executor::polyfill::executor::PolyfillExecutor;
use std::{sync::Once, time::{Duration, Instant}};

static START_MAIN: Once = Once::new();

fn ensure_polyfill_main_executor() {
    START_MAIN.call_once(|| {
        std::thread::Builder::new()
            .name("polyfill-main-executor".into())
            .spawn(|| native_executor::polyfill::executor::start_main_executor())
            .expect("failed to start polyfill main executor");
        // Give the executor a moment to initialize before scheduling work.
        std::thread::sleep(Duration::from_millis(30));
    });
}

#[test]
fn polyfill_spawn_runs_background_task() {
    ensure_polyfill_main_executor();
    let exec = PolyfillExecutor::default();
    let task = exec.spawn(async { 2 + 5 });
    assert_eq!(block_on(task), 7);
}

#[test]
fn polyfill_main_executor_can_start() {
    ensure_polyfill_main_executor();
}

#[test]
fn polyfill_timer_respects_duration() {
    let start = Instant::now();
    block_on(PolyfillTimer::after(Duration::from_millis(15)));
    assert!(
        start.elapsed() >= Duration::from_millis(10),
        "polyfill timer should wait close to the requested duration"
    );
}
