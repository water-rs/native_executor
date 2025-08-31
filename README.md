# Native Executor

[![Crates.io](https://img.shields.io/crates/v/native-executor.svg)](https://crates.io/crates/native-executor)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![docs.rs](https://docs.rs/native-executor/badge.svg)](https://docs.rs/native-executor)

A platform-native async task executor for Rust with a clean and simple API.

## Features

- 🚀 **Platform-native execution**: Uses Apple's Grand Central Dispatch (GCD) on macOS/iOS
- 🔄 **Async/await support**: Fully compatible with Rust's async/await syntax
- 🧵 **Thread-safe tasks**: Safe execution across thread boundaries with `Task`
- 🔒 **Thread-local tasks**: Support for thread-local execution with `LocalTask`
- 🔝 **Priority levels**: Control task execution priority
- ⏱️ **Timer utilities**: Built-in timers and sleep functionality
- 🔍 **Thread-safety utilities**: `LocalValue`, `OnceValue`, and `MainValue` for controlled thread access
- 📦 **`#[no_std]` support**: Compatible with environments without the standard library

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
    task(async {
        println!("Starting task");

        // Sleep for 1 second
        Timer::after(Duration::from_secs(1)).await;

        println!("Task completed after 1 second");
    });

    // Keep the main thread alive to allow tasks to complete
    std::thread::sleep(Duration::from_secs(2));
}
```

## Core Components

### Task Spawning

The library provides several ways to spawn tasks:

```rust
use native_executor::{task, Task, Priority};

// Simple task spawning
let task_handle = task(async { /* your async code */ });

// Task with explicit priority
let background_task = Task::with_priority(async { /* background work */ }, Priority::Background);

// Task that must run on the main thread
let ui_task = Task::on_main(async { /* UI update code */ });
```

### Timer Utilities

```rust
use native_executor::timer::{Timer, sleep};
use std::time::Duration;

async fn example() {
    // Create a timer with fluent API
    Timer::after(Duration::from_millis(500)).await;

    // Shorthand for seconds
    Timer::after_secs(1).await;

    // Simple sleep function
    sleep(2).await;
}
```

### Thread-safety Utilities

#### LocalValue

For values that must only be accessed on a single thread:

```rust
use native_executor::LocalValue;

let local = LocalValue::new(42);
assert_eq!(*local, 42); // OK if on the same thread
// Access from another thread would panic
```

#### OnceValue

For values that can be taken only once:

```rust
use native_executor::OnceValue;

let once = OnceValue::new("hello");
assert_eq!(&*once.get(), "hello");
let value = once.take(); // Take ownership
// once.get() would now panic
```

#### MainValue

For values that must be accessed only on the main thread:

```rust
use native_executor::MainValue;

let main_value = MainValue::new(String::from("UI element"));

// Access value on the main thread
let result = main_value.handle(|value| value.len()).await;
assert_eq!(result, 10);
```

## Platform Support

Currently, the library supports:

- macOS, iOS, and other Apple platforms via Grand Central Dispatch

Future versions will add support for:

- Windows
- Linux
- Android

## Examples

See the [examples directory](https://github.com/waterui/native-executor/examples) for more usage examples.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
