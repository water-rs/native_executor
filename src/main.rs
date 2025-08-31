//! Example binary demonstrating the task execution capabilities.
//!
//! This binary showcases how to use the `waterui_task` crate to run async tasks.

use native_executor::{task, timer::Timer};
use std::time::Duration;

fn main() {
    task(hello());
}

/// Example async function that prints a greeting message.
///
/// This function demonstrates a simple async operation that can be
/// executed using the task runner.
pub async fn hello() {
    Timer::after(Duration::from_millis(100)).await;
    println!("Hello,world");
}
