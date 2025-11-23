#![cfg(target_vendor = "apple")]

use futures_lite::future::block_on;
use native_executor::{NativeExecutor, NativeTimer, Priority};
use std::{cell::Cell, rc::Rc, time::{Duration, Instant}};

#[test]
fn spawn_background_runs_off_main_thread() {
    let exec = NativeExecutor::with_priority(Priority::Background);
    let task = exec.spawn(async { unsafe { libc::pthread_main_np() != 0 } });
    let on_main = block_on(task);
    assert!(
        !on_main,
        "background tasks should avoid the main thread on Apple platforms"
    );
}

#[test]
#[ignore = "requires a running main-thread runloop; enable when exercising with a GUI runner"]
fn spawn_main_runs_on_main_thread() {
    let exec = NativeExecutor::new();
    let task = exec.spawn_main(async { unsafe { libc::pthread_main_np() != 0 } });
    let on_main = block_on(task);
    assert!(on_main, "main tasks should execute on the main dispatch queue");
}

#[test]
#[ignore = "requires a running main-thread runloop; enable when exercising with a GUI runner"]
fn spawn_main_local_handles_non_send_on_main_thread() {
    let exec = NativeExecutor::new();
    let counter = Rc::new(Cell::new(0));
    let local = counter.clone();
    let task = exec.spawn_main_local(async move {
        local.set(1);
        (local.get(), unsafe { libc::pthread_main_np() != 0 })
    });
    let (value, on_main) = block_on(task);
    assert_eq!(value, 1);
    assert!(on_main, "local main tasks should stay on the main thread");
}

#[test]
fn timer_waits_for_requested_duration() {
    let start = Instant::now();
    block_on(NativeTimer::after(Duration::from_millis(25)));
    assert!(
        start.elapsed() >= Duration::from_millis(20),
        "timer should not complete significantly before the requested delay"
    );
}
