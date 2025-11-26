#![cfg(target_arch = "wasm32")]

use native_executor::{sleep, NativeExecutor};
use std::time::Duration;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn spawn_and_join_background_task() {
    let exec = NativeExecutor::new();
    let task = exec.spawn(async { 21 + 21 });
    assert_eq!(task.await, 42);
}

#[wasm_bindgen_test(async)]
async fn timer_waits_before_completing() {
    let start = js_sys::Date::now();
    sleep(Duration::from_millis(10)).await;
    let elapsed = js_sys::Date::now() - start;
    assert!(
        elapsed >= 8.0,
        "timer completed too early: elapsed={elapsed}ms"
    );
}
