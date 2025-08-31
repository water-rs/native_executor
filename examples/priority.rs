use native_executor::{Task, Priority, timer::Timer};
use std::time::Duration;

fn main() {
    // Spawn a default priority task
    Task::new(async {
        println!("Default priority task started");
        Timer::after_secs(1).await;
        println!("Default priority task completed");
    });
    
    // Spawn a background priority task
    Task::with_priority(async {
        println!("Background priority task started");
        Timer::after_secs(1).await;
        println!("Background priority task completed");
    }, Priority::Background);
    
    // Keep the main thread alive
    std::thread::sleep(Duration::from_secs(2));
}
