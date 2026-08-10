#![cfg(all(
    feature = "polyfill",
    not(target_vendor = "apple"),
    not(target_arch = "wasm32"),
    not(target_os = "android")
))]

//! Lives in its own integration binary so it runs in a fresh process: the
//! assertion is about the state *before* any main executor starts, which a
//! sibling test in the same process could otherwise invalidate.

use native_executor::{NativeExecutor, NativeMainExecutor};

#[test]
fn main_executor_is_unavailable_until_a_main_thread_is_started() {
    assert!(
        !native_executor::main_thread_available(),
        "no thread has run start_main_executor yet"
    );
    assert!(
        NativeMainExecutor::new().is_none(),
        "a main-thread executor must not be obtainable without a main thread"
    );

    // The global executor has no main-thread affinity and stays available.
    let _global = NativeExecutor::new();
}
