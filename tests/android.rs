#![cfg(target_os = "android")]

use futures_lite::future::block_on;
use native_executor::{sleep, NativeExecutor};
use std::time::{Duration, Instant};

#[test]
fn spawn_background_returns_value() {
    let exec = NativeExecutor::new();
    let task = exec.spawn(async { 3 + 4 });
    assert_eq!(block_on(task), 7);
}

#[test]
fn timer_waits_for_delay() {
    let start = Instant::now();
    block_on(sleep(Duration::from_millis(20)));
    assert!(
        start.elapsed() >= Duration::from_millis(15),
        "timer completed too early"
    );
}
