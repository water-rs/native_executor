# Native Executor Examples

This directory contains comprehensive examples demonstrating the capabilities of the native-executor library. Each example focuses on specific features and usage patterns.

## Running Examples

Execute any example using Cargo:

```bash
cargo run --example simple_task
cargo run --example timers
cargo run --example priority
cargo run --example main_thread
cargo run --example local_value
```

## Simple Task Spawning

**File:** `simple_task.rs`

Demonstrates basic task creation and execution with platform-native scheduling:

```rust
use native_executor::{spawn_local, timer::Timer};
use std::time::Duration;

fn main() {
    println!("Starting native executor example");

    // Spawn a task with default priority using platform-native scheduling
    let handle = spawn_local(async {
        println!("Task started on background thread");

        // High-precision timer using platform scheduling
        Timer::after_secs(1).await;

        println!("Task completed after 1 second");
        42 // Return value
    });

    // Keep the main thread alive for task completion
    std::thread::sleep(Duration::from_secs(2));

    println!("Example completed");
}
```

## Main Thread Execution

**File:** `main_thread.rs`

Shows how to safely access main-thread-only values from any thread:

```rust
use native_executor::{spawn_main, MainValue};
use std::time::Duration;

fn main() {
    // Create a value that must be accessed on the main thread
    let ui_element = MainValue::new(String::from("Window Title"));

    // Spawn a task that safely accesses the main-thread value
    let _task = spawn_main(async move {
        // This closure runs on the main thread, even though
        // the task was spawned from a background context
        let length = ui_element.handle(|value| {
            println!("Accessing UI element: {}", value);
            value.len() // Safe main-thread access
        }).await;

        println!("UI element length: {}", length);
    });

    // Allow time for task completion
    std::thread::sleep(Duration::from_secs(1));
}
```

## Priority Control

**File:** `priority.rs`

Demonstrates task priority management for optimal resource allocation:

```rust
use native_executor::{spawn, spawn_with_priority, Priority, timer::Timer};
use std::time::Duration;

fn main() {
    // Default priority for time-sensitive operations
    spawn(async {
        println!("Default priority task started");
        Timer::after_secs(1).await;
        println!("Default priority task completed");
    });

    // Background priority for non-critical work
    // These tasks yield CPU time to higher-priority tasks
    spawn_with_priority(async {
        println!("Background priority task started");
        Timer::after_secs(1).await;
        println!("Background priority task completed");
    }, Priority::Background);

    // Allow both tasks to complete
    std::thread::sleep(Duration::from_secs(3));
}
```

## Thread-Local Values

**File:** `local_value.rs`

Showcases thread-safety utilities for controlled access patterns:

```rust
use native_executor::{LocalValue, OnceValue};
use std::time::Duration;

fn main() {
    // LocalValue enforces single-thread access
    let local = LocalValue::new(42);

    // Safe access on the same thread
    println!("Thread-local value: {}", *local);
    println!("Dereferenced: {:?}", local);

    // OnceValue allows single consumption
    let once = OnceValue::new("consume me once");

    // First access - read the value
    println!("Reading once-value: {}", &*once.get());

    // Take ownership - value is consumed
    let consumed = once.take();
    println!("Consumed value: {}", consumed);

    // Subsequent access would panic (safely prevented)
    // once.get(); // ❌ Would panic - value already consumed

    std::thread::sleep(Duration::from_secs(1));
}
```

## High-Precision Timers

**File:** `timers.rs`

Demonstrates platform-native timing capabilities with various APIs:

```rust
use native_executor::{spawn, timer::{Timer, sleep}};
use std::time::Duration;

fn main() {
    spawn(async {
        println!("Starting high-precision timers example");

        // Precise timing with Duration
        println!("Waiting for 500ms with platform-native precision...");
        Timer::after(Duration::from_millis(500)).await;
        println!("✓ 500ms elapsed with high precision");

        // Convenient seconds API
        println!("Waiting for 1 second using convenience method...");
        Timer::after_secs(1).await;
        println!("✓ 1 second elapsed");

        // Simple sleep function for quick delays
        println!("Sleeping for 2 seconds using sleep function...");
        sleep(2).await;
        println!("✓ 2 seconds elapsed");

        println!("All timers completed successfully!");
    });

    // Keep main thread alive for task completion
    std::thread::sleep(Duration::from_secs(5));
}
```

## Key Features Demonstrated

- **Platform-native scheduling**: All examples leverage OS primitives for optimal performance
- **Thread safety**: Examples show safe cross-thread communication patterns
- **Priority control**: Background tasks yield to higher-priority operations
- **Main-thread safety**: UI and thread-local operations remain safe and predictable
- **High-precision timing**: Platform-native timers provide accurate delays
- **Zero-cost abstractions**: Minimal overhead over direct OS API usage
