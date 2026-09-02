use async_task as async_task_crate;
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

use executor_core::async_task::{self as core_async_task, AsyncTask, Runnable};

use crate::{PlatformExecutor, Priority};

#[derive(Clone, Copy, Debug)]
pub struct AppleExecutor {
    queue: DispatchQueue,
    main_queue: DispatchQueue,
}

#[derive(Debug)]
pub struct AppleTimer {
    state: Arc<TimerState>,
    source: DispatchSource,
}

impl AppleExecutor {
    pub fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        spawn_task_on_queue(self.main_queue, fut)
    }

    fn new(priority: Priority) -> Self {
        Self {
            queue: DispatchQueue::global(priority),
            main_queue: DispatchQueue::main(),
        }
    }
}

impl PlatformExecutor for AppleExecutor {
    type Timer = AppleTimer;

    fn with_priority(priority: Priority) -> Self {
        Self::new(priority)
    }

    fn sleep(duration: Duration) -> Self::Timer {
        AppleTimer::after(duration)
    }

    fn spawn<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        spawn_task_on_queue(self.queue, fut)
    }

    fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        Self::spawn_main(self, fut)
    }

    fn spawn_main_local<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future + 'static,
    {
        spawn_local_task_on_queue(self.main_queue, fut)
    }
}

impl AppleTimer {
    fn after(duration: Duration) -> Self {
        let queue = DispatchQueue::timer();
        let source = DispatchSource::timer(queue);
        let state = Arc::new(TimerState::new());
        let context = Box::new(TimerContext {
            state: Arc::clone(&state),
        });
        let context_ptr = Box::into_raw(context).cast();

        unsafe {
            dispatch_set_context(source.as_object(), context_ptr);
            dispatch_set_finalizer_f(source.as_object(), Some(timer_finalizer));
            dispatch_source_set_event_handler_f(source.as_raw(), Some(timer_handler));

            let deadline = dispatch_time(DISPATCH_TIME_NOW, duration_to_dispatch_delta(duration));
            let leeway = leeway_for_duration(duration);
            dispatch_source_set_timer(source.as_raw(), deadline, 0, leeway);
            dispatch_resume(source.as_object());
        }

        Self { state, source }
    }
}

impl Drop for AppleTimer {
    fn drop(&mut self) {
        self.source.cancel();
    }
}

impl Future for AppleTimer {
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

fn spawn_task_on_queue<Fut>(queue: DispatchQueue, fut: Fut) -> AsyncTask<Fut::Output>
where
    Fut: Future<Output: Send> + Send + 'static,
{
    let schedule_queue = queue;
    let (runnable, task) = core_async_task::spawn(fut, move |runnable| {
        schedule_queue.dispatch(runnable);
    });
    schedule_queue.dispatch(runnable);
    task
}

fn spawn_local_task_on_queue<Fut>(queue: DispatchQueue, fut: Fut) -> AsyncTask<Fut::Output>
where
    Fut: Future + 'static,
{
    let schedule_queue = queue;
    let (runnable, task) = async_task_crate::spawn_local(fut, move |runnable| {
        schedule_queue.dispatch(runnable);
    });
    schedule_queue.dispatch(runnable);
    AsyncTask::from(task)
}

#[derive(Clone, Copy, Debug)]
struct DispatchQueue {
    raw: dispatch_queue_t,
}

impl DispatchQueue {
    fn global(priority: Priority) -> Self {
        let qos = priority_to_qos(priority);
        let raw = unsafe { dispatch_get_global_queue(qos, 0) };
        assert!(!raw.is_null(), "dispatch_get_global_queue returned null");
        Self { raw }
    }

    fn main() -> Self {
        let raw = dispatch_get_main_queue_handle();
        assert!(!raw.is_null(), "dispatch_get_main_queue returned null");
        Self { raw }
    }

    fn timer() -> Self {
        Self::global(Priority::Default)
    }

    fn dispatch(self, runnable: Runnable) {
        let boxed = Box::new(runnable);
        unsafe {
            dispatch_async_f(self.raw, Box::into_raw(boxed).cast(), Some(run_runnable));
        }
    }
}

unsafe impl Send for DispatchQueue {}
unsafe impl Sync for DispatchQueue {}

#[derive(Debug)]
struct DispatchSource {
    raw: dispatch_source_t,
}

impl DispatchSource {
    fn timer(queue: DispatchQueue) -> Self {
        let raw = unsafe { dispatch_source_create(timer_source_type(), 0, 0, queue.raw) };
        assert!(!raw.is_null(), "dispatch_source_create returned null");
        Self { raw }
    }

    const fn as_raw(&self) -> dispatch_source_t {
        self.raw
    }

    const fn as_object(&self) -> dispatch_object_t {
        self.raw
    }

    fn cancel(&self) {
        unsafe {
            dispatch_source_cancel(self.raw);
        }
    }
}

unsafe impl Send for DispatchSource {}
unsafe impl Sync for DispatchSource {}

impl Drop for DispatchSource {
    fn drop(&mut self) {
        unsafe {
            dispatch_release(self.raw);
        }
    }
}

#[derive(Debug)]
struct TimerState {
    completed: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

impl TimerState {
    const fn new() -> Self {
        Self {
            completed: AtomicBool::new(false),
            waker: Mutex::new(None),
        }
    }

    fn register(&self, waker: &Waker) {
        let mut guard = self.waker.lock().expect("TimerState poisoned");
        match guard.as_ref() {
            Some(existing) if existing.will_wake(waker) => {}
            _ => *guard = Some(waker.clone()),
        }
        let pending = if self.completed.load(Ordering::Acquire) {
            guard.take()
        } else {
            None
        };
        drop(guard);
        if let Some(pending) = pending {
            pending.wake();
        }
    }

    fn complete(&self) {
        if self.completed.swap(true, Ordering::AcqRel) {
            return;
        }
        // Release the lock before waking: a waker is free to re-enter the timer.
        let pending = self.waker.lock().expect("TimerState poisoned").take();
        if let Some(pending) = pending {
            pending.wake();
        }
    }

    fn is_complete(&self) -> bool {
        self.completed.load(Ordering::Acquire)
    }
}

struct TimerContext {
    state: Arc<TimerState>,
}

unsafe extern "C" fn run_runnable(ctx: *mut core::ffi::c_void) {
    if ctx.is_null() {
        return;
    }
    unsafe {
        let runnable = Box::from_raw(ctx.cast::<Runnable>());
        runnable.run();
    }
}

unsafe extern "C" fn timer_handler(ctx: *mut core::ffi::c_void) {
    if ctx.is_null() {
        return;
    }
    unsafe {
        let context = &*ctx.cast::<TimerContext>();
        context.state.complete();
    }
}

unsafe extern "C" fn timer_finalizer(ctx: *mut core::ffi::c_void) {
    if ctx.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(ctx.cast::<TimerContext>()));
    }
}

const fn timer_source_type() -> dispatch_source_type_t {
    &raw const _dispatch_source_type_timer
}

fn duration_to_dispatch_delta(duration: Duration) -> i64 {
    i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX)
}

fn leeway_for_duration(duration: Duration) -> u64 {
    // Let the OS coalesce wakeups by firing a timer up to this late.
    const MAX_LEEWAY_NS: u64 = 5_000_000;
    if duration.is_zero() {
        return 0;
    }
    u64::try_from(duration.as_nanos())
        .unwrap_or(u64::MAX)
        .min(MAX_LEEWAY_NS)
}

#[allow(unreachable_patterns)]
const fn priority_to_qos(priority: Priority) -> libc::c_long {
    use libc::qos_class_t::{
        QOS_CLASS_BACKGROUND, QOS_CLASS_DEFAULT, QOS_CLASS_UNSPECIFIED, QOS_CLASS_USER_INITIATED,
        QOS_CLASS_USER_INTERACTIVE, QOS_CLASS_UTILITY,
    };
    match priority {
        Priority::Background => QOS_CLASS_BACKGROUND as libc::c_long,
        Priority::Utility => QOS_CLASS_UTILITY as libc::c_long,
        Priority::UserInitiated => QOS_CLASS_USER_INITIATED as libc::c_long,
        Priority::UserInteractive => QOS_CLASS_USER_INTERACTIVE as libc::c_long,
        Priority::Default => QOS_CLASS_DEFAULT as libc::c_long,
        _ => QOS_CLASS_UNSPECIFIED as libc::c_long,
    }
}

#[allow(non_camel_case_types)]
type dispatch_queue_t = *mut core::ffi::c_void;
#[allow(non_camel_case_types)]
#[repr(C)]
struct dispatch_queue_s {
    _private: [u8; 0],
}
#[allow(non_camel_case_types)]
type dispatch_source_t = *mut core::ffi::c_void;
#[allow(non_camel_case_types)]
type dispatch_object_t = *mut core::ffi::c_void;
#[allow(non_camel_case_types)]
type dispatch_source_type_t = *const dispatch_source_type_s;
#[allow(non_camel_case_types)]
type dispatch_time_t = u64;
#[allow(non_camel_case_types)]
type dispatch_function_t = Option<unsafe extern "C" fn(*mut core::ffi::c_void)>;

#[repr(C)]
#[allow(non_camel_case_types)]
struct dispatch_source_type_s {
    _private: [u8; 0],
}

const DISPATCH_TIME_NOW: dispatch_time_t = 0;

#[cfg_attr(
    any(target_os = "macos", target_os = "ios"),
    link(name = "System", kind = "dylib")
)]
#[cfg_attr(
    not(any(target_os = "macos", target_os = "ios")),
    link(name = "dispatch", kind = "dylib")
)]
unsafe extern "C" {
    unsafe fn dispatch_get_global_queue(
        identifier: libc::c_long,
        flags: libc::c_ulong,
    ) -> dispatch_queue_t;
    unsafe fn dispatch_async_f(
        queue: dispatch_queue_t,
        context: *mut core::ffi::c_void,
        work: dispatch_function_t,
    );
    unsafe fn dispatch_time(when: dispatch_time_t, delta: i64) -> dispatch_time_t;
    unsafe fn dispatch_source_create(
        type_: dispatch_source_type_t,
        handle: usize,
        mask: usize,
        queue: dispatch_queue_t,
    ) -> dispatch_source_t;
    unsafe fn dispatch_source_set_timer(
        source: dispatch_source_t,
        start: dispatch_time_t,
        interval: u64,
        leeway: u64,
    );
    unsafe fn dispatch_resume(object: dispatch_object_t);
    unsafe fn dispatch_source_cancel(source: dispatch_source_t);
    unsafe fn dispatch_release(object: dispatch_object_t);
    unsafe fn dispatch_set_context(object: dispatch_object_t, context: *mut core::ffi::c_void);
    unsafe fn dispatch_source_set_event_handler_f(
        source: dispatch_source_t,
        handler: dispatch_function_t,
    );
    unsafe fn dispatch_set_finalizer_f(object: dispatch_object_t, finalizer: dispatch_function_t);
    unsafe static _dispatch_source_type_timer: dispatch_source_type_s;
    unsafe static _dispatch_main_q: dispatch_queue_s;
}

fn dispatch_get_main_queue_handle() -> dispatch_queue_t {
    (&raw const _dispatch_main_q)
        .cast::<core::ffi::c_void>()
        .cast_mut()
}
