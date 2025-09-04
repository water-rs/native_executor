use native_executor::{MainValue, spawn_main};
use std::time::Duration;

fn main() {
    // Create a value that must be accessed on the main thread
    let main_value = MainValue::new(String::from("Hello from main thread"));

    // Spawn a task that accesses the main value
    spawn_main(async move {
        // This will be executed on the main thread
        let result = main_value
            .handle(|value| {
                println!("Accessing main thread value: {value}");
                value.len()
            })
            .await;

        println!("Length: {result}");
    })
    .detach();

    // Wait for the task to complete
    std::thread::sleep(Duration::from_secs(1));
}
