use native_executor::{Priority, spawn, spawn_with_priority, timer::Timer};
use std::time::Duration;

fn main() {
    // Spawn a default priority task
    spawn(async {
        println!("Default priority task started");
        Timer::after_secs(1).await;
        println!("Default priority task completed");
    })
    .detach();

    // Spawn a background priority task
    spawn_with_priority(
        async {
            println!("Background priority task started");
            Timer::after_secs(1).await;
            println!("Background priority task completed");
        },
        Priority::Background,
    )
    .detach();

    // Keep the main thread alive
    std::thread::sleep(Duration::from_secs(2));
}
