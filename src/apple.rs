use executor_core::{Executor, LocalExecutor, async_task::AsyncTask};

use crate::PlatformExecutor;

#[derive(Debug)]
pub struct AppleExecutor;

#[derive(Debug)]
pub struct AppleTimer {}

impl Future for AppleTimer {
    type Output = ();
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!()
    }
}

impl AppleExecutor {
    /// Send a task to be executed on the main thread.
    pub fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        todo!()
    }
}

impl PlatformExecutor for AppleExecutor {
    type Timer = AppleTimer;
    fn spawn<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        todo!()
    }

    fn spawn_main<Fut>(&self, fut: Fut) -> AsyncTask<Fut::Output>
    where
        Fut: Future<Output: Send> + Send + 'static,
    {
        todo!()
    }

    fn with_priority(priority: crate::Priority) -> Self {
        todo!()
    }
    fn sleep(duration: std::time::Duration) -> Self::Timer {
        todo!()
    }
}
