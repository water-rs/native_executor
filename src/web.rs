//! Integration for Web/WASM environments.
//!
//! This module provides executor implementations for web browsers and other
//! WASM environments using `wasm-bindgen-futures`.

use crate::{PlatformExecutor, Priority};
use alloc::boxed::Box;
use core::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(closure: &Closure<dyn FnMut()>, time: u32);
}

/// Web-based executor implementation for WASM targets.
///
/// This executor uses `wasm-bindgen-futures::spawn_local` to execute futures
/// in web environments.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebExecutor;

impl PlatformExecutor for WebExecutor {
    fn exec_main(f: impl FnOnce() + Send + 'static) {
        spawn_local(async move { f() });
    }

    fn exec(f: impl FnOnce() + Send + 'static, _priority: Priority) {
        // Priority is ignored in web executor
        spawn_local(async move { f() });
    }

    fn exec_after(delay: Duration, f: impl FnOnce() + Send + 'static, _priority: Priority) {
        let closure = Closure::once(Box::new(f) as Box<dyn FnOnce()>);
        set_timeout(&closure, delay.as_millis() as u32);
        closure.forget();
    }
}
