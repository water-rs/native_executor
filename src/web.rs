#![cfg(target_arch = "wasm32")]

use async_task as async_task_crate;
use executor_core::async_task::{self as core_async_task, AsyncTask, Runnable};
use js_sys::Function;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
    time::Duration,
};
use wasm_bindgen::{JsCast, prelude::wasm_bindgen};
use wasm_bindgen_futures::spawn_local;

use crate::{PlatformExecutor, Priority};

#[derive(Debug, Default, Clone, Copy)]
pub struct WebExecutor;

#[derive(Debug)]
pub struct WebTimer {
    state: Arc<TimerState>,
}

#[derive(Debug)]
struct TimerState {
    completed: AtomicBool,
    waker: Mutex<Option<Waker>>,
    handle: Mutex<Option<i32>>,
}

impl TimerState {
    fn new() -> Self {
        Self {
            completed: AtomicBool::new(false),
            waker: Mutex::new(None),
            handle: Mutex::new(None),
        }
    }

    fn register(&self, waker: &Waker) {
        let mut guard = self.waker.lock().expect("WebTimer waker poisoned");
        match guard.as_ref() {
            Some(existing) if existing.will_wake(waker) => {}
            _ => *guard = Some(waker.clone()),
        }

        if self.completed.load(Ordering::Acquire) {
            if let Some(w) = guard.take() {
                w.wake();
            }
        }
    }

    fn complete(&self) {
        if !self.completed.swap(true, Ordering::AcqRel) {
            let _ = self.handle.lock().expect("WebTimer handle poisoned").take();
            if let Some(waker) = self.waker.lock().expect("WebTimer waker poisoned").take() {
                waker.wake();
            }
        }
    }

    fn cancel(&self) {
        if self.completed.load(Ordering::Acquire) {
            return;
        }

        if let Some(id) = self.handle.lock().expect("WebTimer handle poisoned").take() {
            clear_timeout(id);
        }
    }

    fn is_complete(&self) -> bool {
        self.completed.load(Ordering::Acquire)
    }
}

impl WebTimer {
    pub fn after(duration: Duration) -> Self {
        let state = Arc::new(TimerState::new());
        let state_for_cb = Arc::clone(&state);
        let millis = duration_to_millis(duration);

        let id = schedule_timeout(state_for_cb, millis);
        *state.handle.lock().expect("WebTimer handle poisoned") = Some(id);

        Self { state }
    }
}

impl Drop for WebTimer {
    fn drop(&mut self) {
        self.state.cancel();
    }
}

impl Future for WebTimer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.state.is_complete() {
            return Poll::Ready(());
        }

        self.state.register(cx.waker());
        if self.state.is_complete() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl PlatformExecutor for WebExecutor {
    type Timer = WebTimer;

    fn with_priority(_priority: Priority) -> Self {
        // Single-threaded environment; priority is ignored.
        Self
    }

    fn sleep(duration: Duration) -> Self::Timer {
        WebTimer::after(duration)
    }

    fn spawn<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        spawn_task(fut)
    }

    fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        // Web has a single thread; main and background are equivalent.
        spawn_task(fut)
    }

    fn spawn_main_local<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future + 'static,
    {
        spawn_local_task(fut)
    }
}

fn spawn_task<Fut>(fut: Fut) -> AsyncTask<Fut::Output>
where
    Fut: Future<Output: Send> + Send + 'static,
{
    let (runnable, task) = core_async_task::spawn(fut, schedule_runnable);
    schedule_runnable(runnable);
    task
}

fn spawn_local_task<Fut>(fut: Fut) -> AsyncTask<Fut::Output>
where
    Fut: Future + 'static,
{
    let (runnable, task) = async_task_crate::spawn_local(fut, schedule_runnable);
    schedule_runnable(runnable);
    AsyncTask::from(task)
}

fn schedule_runnable(runnable: Runnable) {
    spawn_local(async move {
        runnable.run();
    });
}

fn schedule_timeout(state: Arc<TimerState>, millis: i32) -> i32 {
    let closure = wasm_bindgen::closure::Closure::once(move || {
        state.complete();
    });
    let id = set_timeout(closure.as_ref().unchecked_ref(), millis);
    closure.forget();
    id
}

fn duration_to_millis(duration: Duration) -> i32 {
    duration.as_millis().min(i32::MAX as u128) as i32
}

#[wasm_bindgen(js_namespace = globalThis)]
extern "C" {
    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(callback: &Function, timeout: i32) -> i32;

    #[wasm_bindgen(js_name = clearTimeout)]
    fn clear_timeout(handle: i32);
}
