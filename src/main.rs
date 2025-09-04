//! Example binary demonstrating native-executor capabilities.
//!
//! This binary showcases the basic usage of the native-executor library,
//! demonstrating task spawning and high-precision timing with platform-native
//! scheduling primitives.

use native_executor::{spawn, timer::Timer};
use std::time::Duration;

fn main() {
    spawn(hello()).detach();
}

/// Example async function demonstrating timer usage.
///
/// This function showcases a simple async operation with platform-native
/// timing, executed using the native-executor task system.
pub async fn hello() {
    // Use high-precision platform-native timing
    Timer::after(Duration::from_millis(100)).await;
    println!("Hello, world from native-executor!");
}
