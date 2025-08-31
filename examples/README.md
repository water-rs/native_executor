# Examples

This directory contains examples demonstrating the usage of the native-executor library.

## Simple Task

```rust
use native_executor::{task, timer::Timer};
use std::time::Duration;

fn main() {
    println!("Starting example");

    // Spawn a task with default priority
    task(async {
        println!("Task started");

        // Wait for 1 second
        Timer::after_secs(1).await;

        println!("Task completed after 1 second");
    });

    // Keep the main thread alive
    std::thread::sleep(Duration::from_secs(2));

    println!("Example completed");
}
```

## Main Thread Execution

```rust
use native_executor::{Task, MainValue};

fn main() {
    // Create a value that must be accessed on the main thread
    let main_value = MainValue::new(String::from("Hello from main thread"));

    // Spawn a task that accesses the main value
    let task = Task::new(async move {
        // This will be executed on the main thread
        let result = main_value.handle(|value| {
            println!("Accessing main thread value: {}", value);
            value.len()
        }).await;

        println!("Length: {}", result);
    });

    // Wait for the task to complete
    std::thread::sleep(Duration::from_secs(1));
}
```

## Priority Levels

```rust
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
```

## Thread-Local Values

```rust
use native_executor::{task, LocalValue, OnceValue};

fn main() {
    // Create a thread-local value
    let local = LocalValue::new(42);

    // Access it on the same thread
    println!("Value: {}", *local);

    // Create a once-value
    let once = OnceValue::new("take me once");

    // Take the value
    let value = once.take();
    println!("Taken value: {}", value);

    // Keep the main thread alive
    std::thread::sleep(Duration::from_secs(1));
}
```

## Timer Functions

```rust
use native_executor::{task, timer::{Timer, sleep}};
use std::time::Duration;

fn main() {
    task(async {
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
    });

    // Keep the main thread alive
    std::thread::sleep(Duration::from_secs(4));
}
```
