#![cfg(target_os = "android")]

use executor_core::async_task::{self as core_async_task, AsyncTask, Runnable};
use jni::errors::Error as JniError;
use libc::{F_GETFL, F_SETFL, O_NONBLOCK, fcntl, pipe, read, write};
use ndk::looper::{FdEvent, ThreadLooper};
use std::{
    ffi::c_void,
    future::Future,
    os::fd::{AsRawFd, BorrowedFd, RawFd},
    panic,
    sync::{
        OnceLock,
        atomic::{AtomicI32, Ordering},
    },
    thread::{self, ThreadId},
    time::Duration,
};
use thiserror::Error;

use crate::{
    PlatformExecutor, Priority,
    polyfill::{executor::PolyfillExecutor, timer::PolyfillTimer},
};

type Task = Box<dyn FnOnce() + Send + 'static>;
struct TaskWrapper(Option<Task>);

static PIPE_WRITE_FD: AtomicI32 = AtomicI32::new(-1);
static ANDROID_MAIN_THREAD_ID: OnceLock<ThreadId> = OnceLock::new();

#[derive(Debug, Error)]
pub enum AndroidInitError {
    #[error("library already initialized")]
    AlreadyInitialized,
    #[error("must be called from the Android UI thread")]
    WrongThread,
    #[error("failed to retrieve JavaVM")]
    VmNotFound,
    #[error("JNI error: {0}")]
    Jni(#[from] JniError),
    #[error("failed to create pipe or register with looper")]
    PipeError,
    #[error("dispatcher not initialized; call register_android_main_thread on the UI thread first")]
    NotInitialized,
}

#[derive(Debug, Clone, Copy)]
pub struct AndroidExecutor(PolyfillExecutor);

/// Initialize the Android main-thread dispatcher.
///
/// # Safety
///
/// Must be called **on the Android UI thread** (e.g. from a JNI entrypoint).
pub unsafe fn register_android_main_thread() -> Result<(), AndroidInitError> {
    if PIPE_WRITE_FD.load(Ordering::SeqCst) != -1 {
        return Err(AndroidInitError::AlreadyInitialized);
    }

    setup_pipe()?;
    let tid = thread::current().id();
    let _ = ANDROID_MAIN_THREAD_ID.set(tid);
    Ok(())
}

impl PlatformExecutor for AndroidExecutor {
    type Timer = PolyfillTimer;

    fn sleep(duration: Duration) -> Self::Timer {
        PolyfillTimer::after(duration)
    }

    fn with_priority(priority: Priority) -> Self {
        AndroidExecutor(PolyfillExecutor::with_priority(priority))
    }

    fn spawn<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        self.0.spawn(fut)
    }

    fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        let (runnable, task) = core_async_task::spawn(fut, |runnable| {
            dispatch_to_main(runnable)
                .unwrap_or_else(|e| panic!("failed to dispatch to Android UI thread: {e}"));
        });
        dispatch_to_main(runnable)
            .unwrap_or_else(|e| panic!("failed to dispatch to Android UI thread: {e}"));
        task
    }

    fn spawn_main_local<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future + 'static,
    {
        assert_android_main_thread("spawn_main_local");
        let (runnable, task) = core_async_task::spawn_local(fut, |runnable| {
            dispatch_to_main(runnable)
                .unwrap_or_else(|e| panic!("failed to dispatch to Android UI thread: {e}"));
        });
        // initial poll on the UI thread
        runnable.run();
        task.into()
    }
}

fn dispatch_to_main(runnable: Runnable) -> Result<(), AndroidInitError> {
    let fd = PIPE_WRITE_FD.load(Ordering::Acquire);
    if fd < 0 {
        return Err(AndroidInitError::NotInitialized);
    }

    let wrapper = Box::new(TaskWrapper(Some(Box::new(move || {
        runnable.run();
    }))));
    let ptr: *mut TaskWrapper = Box::into_raw(wrapper);
    let addr_bytes = (ptr as usize).to_ne_bytes();

    let res = unsafe { write(fd, addr_bytes.as_ptr() as *const c_void, addr_bytes.len()) };
    if res < 0 {
        // recover the box to avoid leak
        unsafe {
            drop(Box::from_raw(ptr));
        }
        return Err(AndroidInitError::PipeError);
    }

    Ok(())
}

fn setup_pipe() -> Result<(), AndroidInitError> {
    let mut fds: [RawFd; 2] = [0; 2];
    if unsafe { pipe(fds.as_mut_ptr()) } != 0 {
        return Err(AndroidInitError::PipeError);
    }
    let read_fd = fds[0];
    let write_fd = fds[1];

    unsafe {
        let flags = fcntl(read_fd, F_GETFL);
        fcntl(read_fd, F_SETFL, flags | O_NONBLOCK);
    }

    let looper = ThreadLooper::for_thread().ok_or(AndroidInitError::PipeError)?;
    looper
        .add_fd_with_callback(
            unsafe { BorrowedFd::borrow_raw(read_fd) },
            FdEvent::INPUT,
            move |fd, _event| {
                pipe_callback(fd.as_raw_fd());
                true
            },
        )
        .map_err(|_| AndroidInitError::PipeError)?;

    PIPE_WRITE_FD.store(write_fd, Ordering::Release);
    Ok(())
}

fn pipe_callback(fd: RawFd) {
    let mut buffer = [0u8; std::mem::size_of::<usize>()];
    loop {
        let len = unsafe { read(fd, buffer.as_mut_ptr() as *mut c_void, buffer.len()) };
        if len == std::mem::size_of::<usize>() as isize {
            let ptr_addr = usize::from_ne_bytes(buffer);
            let mut wrapper = unsafe { Box::from_raw(ptr_addr as *mut TaskWrapper) };
            let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                if let Some(task) = wrapper.0.take() {
                    task();
                }
            }));
        } else {
            break;
        }
    }
}

fn assert_android_main_thread(op: &str) {
    if let Some(main_id) = ANDROID_MAIN_THREAD_ID.get() {
        if *main_id != thread::current().id() {
            panic!("{op} must be called from the Android UI thread");
        }
    }
}
