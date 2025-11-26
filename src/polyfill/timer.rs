#![allow(dead_code)]

use std::future::Future;

#[derive(Debug)]
pub struct PolyfillTimer {
    timer: async_io::Timer,
}

impl PolyfillTimer {
    pub fn after(duration: std::time::Duration) -> Self {
        Self {
            timer: async_io::Timer::after(duration),
        }
    }
}

impl Future for PolyfillTimer {
    type Output = ();
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(&mut self.timer).poll(cx).map(|_| ())
    }
}
