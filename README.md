# Native Executor

[![Crates.io](https://img.shields.io/crates/v/native-executor.svg)](https://crates.io/crates/native-executor)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![docs.rs](https://docs.rs/native-executor/badge.svg)](https://docs.rs/native-executor)

A high-performance, platform-native async task executor for Rust that leverages operating system primitives for optimal performance and integration.

## Features

- 🚀 **Platform-native execution**: Leverages Apple's Grand Central Dispatch (GCD) on macOS/iOS for optimal performance
- 🔄 **Async/await support**: Fully compatible with Rust's async/await syntax and ecosystem
- 🧵 **Thread-safe tasks**: Safe concurrent execution across thread boundaries with `Task<T>`
- 🔒 **Thread-local tasks**: Efficient thread-local execution with `LocalTask<T>`
- 🔝 **Priority control**: Fine-grained task execution priority management
- ⏱️ **Timer utilities**: High-precision timers and sleep functionality with fluent API
- 🔐 **Thread-safety utilities**: `LocalValue<T>`, `OnceValue<T>`, and `MainValue<T>` for controlled thread access patterns
- 📦 **`#[no_std]` compatible**: Works in embedded and resource-constrained environments
- 🎯 **Zero-cost abstractions**: Minimal overhead over native platform APIs

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
native-executor = "0.1.0"
```

## Quick Start

```rust
use native_executor::{task, timer::Timer};
use std::time::Duration;

fn main() {
    // Spawn a task with default priority
    let handle = task(async {
        println!("Starting async task");

        // High-precision timer using platform-native scheduling
        Timer::after(Duration::from_secs(1)).await;

        println!("Task completed after 1 second");
    });

    // Keep the main thread alive to allow tasks to complete
    std::thread::sleep(Duration::from_secs(2));
}
```

## Core Components

### Task Spawning

The library provides flexible task spawning with different execution contexts:

```rust
use native_executor::{task, Task, Priority};

// Simple task spawning with default priority
let task_handle = task(async {
    // Your async code here
    println!("Running on a background thread");
});

// Task with explicit priority control
let background_task = Task::with_priority(async {
    // Low-priority background work
    heavy_computation().await;
}, Priority::Background);

// Task guaranteed to run on the main thread
let ui_task = Task::on_main(async {
    // UI updates or main-thread-only operations
    update_user_interface().await;
});
```

### Timer Utilities

High-precision timing with platform-native scheduling:

```rust
use native_executor::timer::{Timer, sleep};
use std::time::Duration;

async fn timing_example() {
    // Precise timing with platform-native scheduling
    Timer::after(Duration::from_millis(500)).await;

    // Convenient shorthand for second-based delays
    Timer::after_secs(1).await;

    // Simple sleep function for quick delays
    sleep(2).await;
    
    println!("All timers completed with high precision!");
}
```

### Thread-safety Utilities

Specialized containers for different thread-safety requirements:

#### LocalValue

Thread-local values that enforce single-thread access:

```rust
use native_executor::LocalValue;

let local = LocalValue::new(42);
assert_eq!(*local, 42); // ✅ OK on the same thread
// ❌ Access from another thread would panic for safety
```

#### OnceValue

Values that can be consumed exactly once:

```rust
use native_executor::OnceValue;

let once = OnceValue::new("take me once");
assert_eq!(&*once.get(), "take me once"); // ✅ Read access
let value = once.take(); // ✅ Take ownership
// ❌ once.get() would now panic - value consumed
```

#### MainValue

Cross-thread safe values with main-thread access:

```rust
use native_executor::MainValue;

let ui_element = MainValue::new(String::from("Button"));

// Safe cross-thread access - execution happens on main thread
let length = ui_element.handle(|element| {
    println!("UI element: {}", element);
    element.len()
}).await;

assert_eq!(length, 6);
```

## Platform Support

### Current Support

- **Apple Platforms** (macOS, iOS, tvOS, watchOS): Full support via Grand Central Dispatch (GCD)
  - Leverages system-level thread pools and scheduling
  - Priority mapping to GCD queue priorities
  - Optimal performance and system integration

### Planned Support

- **Windows**: Native thread pool and completion ports integration
- **Linux**: epoll and io_uring based implementation
- **Android**: Android-specific optimizations
- **WebAssembly**: Browser and WASI runtime support

Each platform implementation leverages native OS primitives for maximum performance.

## Examples

Explore comprehensive usage examples in the [examples directory](examples/):

- **[Simple Task](examples/simple_task.rs)**: Basic task spawning and execution
- **[Timers](examples/timers.rs)**: Timer utilities and sleep functionality  
- **[Priority Control](examples/priority.rs)**: Task priority management
- **[Main Thread](examples/main_thread.rs)**: Main thread execution patterns
- **[Thread-Local Values](examples/local_value.rs)**: Thread-local and once-value containers

Run any example with:
```bash
cargo run --example simple_task
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
