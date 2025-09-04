use native_executor::{spawn, timer::Timer};
use std::time::Duration;

fn main() {
    println!("Starting example");

    // Spawn a task with default priority
    spawn(async {
        println!("Task started");

        // Wait for 1 second
        Timer::after_secs(1).await;

        println!("Task completed after 1 second");
    })
    .detach();

    // Keep the main thread alive
    std::thread::sleep(Duration::from_secs(2));

    println!("Example completed");
}
