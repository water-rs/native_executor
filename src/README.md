# Native Executor

A platform-native asynchronous task executor for Rust that leverages system-specific task scheduling capabilities.

## Overview

The Native Executor library provides a clean, efficient way to execute asynchronous tasks on platform-native threading and scheduling mechanisms. It currently uses Apple's Grand Central Dispatch (GCD) on macOS and iOS platforms, with plans to extend to other platforms in future versions.

## Core Components

### Task Execution

- **Task**: Thread-safe task type that can be awaited or canceled
- **LocalTask**: Thread-local task variant for non-Send futures
- **Priority levels**: Control over execution priority (Default, Background)

### Thread Safety Utilities

- **LocalValue**: Container ensuring values are only accessed on their original thread
- **OnceValue**: Value container that can be taken exactly once
- **MainValue**: Safe container for values that must be accessed on the main thread

### Timing Utilities

- **Timer**: Future that completes after a specified duration
- **sleep**: Convenience function for timed waits

## Usage Examples

Spawn a basic task:

```rust
use native_executor::task;

task(async {
    println!("Running in a background task");
});
```

Run code on the main thread:

```rust
use native_executor::Task;

let result = Task::on_main(async {
    // This code will execute on the main thread
    42
}).await;
```

Timer operations:

```rust
use native_executor::timer::{Timer, sleep};
use std::time::Duration;

async fn example() {
    // Wait for 500ms
    Timer::after(Duration::from_millis(500)).await;

    // Wait for 2 seconds
    sleep(2).await;
}
```

## Planned Improvements

- Support for Windows platform
- Support for Linux platform
- Support for Android platform
- Additional executor features for common use cases

## Technical Implementation

The library builds on top of executor-core and async-task, providing a clean API while efficiently utilizing platform-specific threading capabilities.
