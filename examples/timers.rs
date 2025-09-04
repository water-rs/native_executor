use native_executor::{
    spawn,
    timer::{Timer, sleep},
};
use std::time::Duration;

fn main() {
    spawn(async {
        println!("Starting timers example");

        // Use the Timer API
        println!("Waiting for 500ms...");
        Timer::after(Duration::from_millis(500)).await;
        println!("500ms elapsed");

        // Use the seconds convenience method
        println!("Waiting for 1 second...");
        Timer::after_secs(1).await;
        println!("1 second elapsed");

        // Use the sleep convenience function
        println!("Sleeping for 2 seconds...");
        sleep(2).await;
        println!("2 seconds elapsed");

        println!("Timers example completed");
    })
    .detach();

    // Keep the main thread alive
    std::thread::sleep(Duration::from_secs(4));
}
